use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::Mutex;
use harbory_protocol::v1::{ComposeSpec, ComposeState};
use crate::git_build;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum DeployError {
    #[error("docker compose error: {0}")]
    Command(String),
    #[error(transparent)]
    Build(#[from] git_build::BuildError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct ComposeManager {
    errors: Mutex<HashMap<String, String>>,
}

impl ComposeManager {
    pub fn new() -> Self {
        Self { errors: Mutex::new(HashMap::new()) }
    }

    pub async fn deploy(&self, spec: &ComposeSpec) -> Result<(), DeployError> {
        match self.try_deploy(spec).await {
            Ok(()) => {
                self.errors.lock().await.remove(&spec.name);
                Ok(())
            }
            Err(err) => {
                self.errors.lock().await.insert(spec.name.clone(), err.to_string());
                Err(err)
            }
        }
    }

    async fn try_deploy(&self, spec: &ComposeSpec) -> Result<(), DeployError> {
        let work_dir = PathBuf::from("/var/lib/harbory-agent").join("compose").join(&spec.name);
        tokio::fs::create_dir_all(&work_dir).await?;

        // 1. Clone repo if needed
        if let Some(git_source) = &spec.git_source {
            tracing::info!(name = %spec.name, repo = %git_source.repo_url, "cloning compose repo");
            crate::git_build::clone_repo(&git_source.repo_url, &git_source.git_ref, &work_dir).await?;
        }

        // 2. Run docker compose up -d
        tracing::info!(name = %spec.name, "running docker compose up -d");
        
        let file_arg = if spec.compose_file_path.is_empty() {
            "docker-compose.yml".to_string()
        } else {
            spec.compose_file_path.clone()
        };

        let output = Command::new("docker")
            .arg("compose")
            .arg("-f")
            .arg(&file_arg)
            .arg("-p")
            .arg(&spec.name)
            .arg("up")
            .arg("-d")
            .arg("--build")
            .current_dir(&work_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DeployError::Command(format!("failed to up: {stderr}")));
        }

        Ok(())
    }

    pub async fn remove(&self, name: &str) -> Result<(), DeployError> {
        tracing::info!(%name, "running docker compose down");
        
        let output = Command::new("docker")
            .arg("compose")
            .arg("-p")
            .arg(name)
            .arg("down")
            .arg("-v")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!(%name, stderr = %stderr, "docker compose down returned non-zero");
        }

        self.errors.lock().await.remove(name);
        
        // Clean up the working directory
        let work_dir = PathBuf::from("/var/lib/harbory-agent").join("compose").join(name);
        if work_dir.exists() {
            let _ = tokio::fs::remove_dir_all(&work_dir).await;
        }

        Ok(())
    }

    pub async fn list_state(&self) -> Result<Vec<ComposeState>, std::io::Error> {
        // We use `docker compose ls --format json`
        let output = Command::new("docker")
            .arg("compose")
            .arg("ls")
            .arg("-a") // all
            .arg("--format")
            .arg("json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            tracing::error!("failed to run docker compose ls");
            return Ok(vec![]);
        }

        let out_str = String::from_utf8_lossy(&output.stdout);
        let Ok(parsed): Result<Vec<serde_json::Value>, _> = serde_json::from_str(&out_str) else {
            tracing::error!("failed to parse docker compose ls json");
            return Ok(vec![]);
        };

        let mut observed = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for item in parsed {
            if let Some(name) = item.get("Name").and_then(|n| n.as_str()) {
                let status_str = item.get("Status").and_then(|s| s.as_str()).unwrap_or("");
                let status = if status_str.contains("running") {
                    1
                } else if status_str.contains("exited") || status_str.contains("stopped") {
                    2
                } else {
                    1 // default to running if unsure
                };

                seen.insert(name.to_string());
                observed.push(ComposeState {
                    name: name.to_string(),
                    status: status as i32,
                    error: String::new(),
                });
            }
        }

        // Add errored stacks that failed `try_deploy`
        let errors = self.errors.lock().await;
        for (name, error) in errors.iter() {
            if !seen.contains(name) {
                observed.push(ComposeState {
                    name: name.clone(),
                    status: 4,
                    error: error.clone(),
                });
            }
        }

        Ok(observed)
    }
}
