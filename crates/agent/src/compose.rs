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
        // Start from a clean slate every deploy: a leftover clone from a
        // previous deploy makes `git clone` fail ("already exists and is
        // not an empty directory"), which would break every redeploy.
        // `up --build` rebuilds everything anyway, so nothing here is
        // worth keeping across deploys — .env is rewritten below.
        let _ = tokio::fs::remove_dir_all(&work_dir).await;
        tokio::fs::create_dir_all(&work_dir).await?;

        // 1. Clone repo if needed
        if let Some(git_source) = &spec.git_source {
            tracing::info!(name = %spec.name, repo = %git_source.repo_url, "cloning compose repo");
            crate::git_build::clone_repo(&git_source.repo_url, &git_source.git_ref, &work_dir).await?;
        }

        // Write .env file
        let env_content = spec.env.join("\n");
        tokio::fs::write(work_dir.join(".env"), env_content).await?;

        // 2. Run docker compose up -d
        tracing::info!(name = %spec.name, "running docker compose up -d");

        let file_arg = if spec.compose_file_path.is_empty() {
            "docker-compose.yml".to_string()
        } else {
            spec.compose_file_path.clone()
        };

        // Compose files written for a shared reverse-proxy setup often
        // declare `networks: web: external: true`, assuming that network
        // already exists on the host. Harbory deploys each stack standalone
        // with no shared infra, so that network never exists yet — without
        // this, `up` fails outright with "declared as external, but could
        // not be found" and zero containers get created. Create any missing
        // external networks up front so those compose files work as-is.
        Self::ensure_external_networks(&work_dir, &file_arg, &spec.name).await;

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
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    DeployError::Command(
                        "'docker' executable not found on this host — install Docker and restart harbory-agent.".into(),
                    )
                } else {
                    DeployError::Io(e)
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DeployError::Command(format!("failed to up: {stderr}")));
        }

        Ok(())
    }

    /// Resolves the compose file's declared networks via `docker compose
    /// config` and `docker network create`s any marked `external: true`
    /// that don't exist yet on this host. Best-effort: a failure here just
    /// means `up` fails with its own (now-familiar) error, same as before
    /// this existed — never blocks the deploy on its own.
    async fn ensure_external_networks(work_dir: &PathBuf, file_arg: &str, project_name: &str) {
        let output = Command::new("docker")
            .arg("compose")
            .arg("-f")
            .arg(file_arg)
            .arg("-p")
            .arg(project_name)
            .arg("config")
            .arg("--format")
            .arg("json")
            .current_dir(work_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        let Ok(output) = output else { return };
        if !output.status.success() {
            return;
        }

        let Ok(config): Result<serde_json::Value, _> = serde_json::from_slice(&output.stdout) else {
            return;
        };
        let Some(networks) = config.get("networks").and_then(|n| n.as_object()) else {
            return;
        };

        let existing = Self::list_network_names().await;

        for (key, def) in networks {
            let is_external = match def.get("external") {
                Some(serde_json::Value::Bool(b)) => *b,
                Some(serde_json::Value::Object(_)) => true,
                _ => false,
            };
            if !is_external {
                continue;
            }
            let network_name = def
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or(key.as_str());

            if existing.contains(network_name) {
                continue;
            }

            tracing::info!(network = %network_name, "creating missing external network declared by compose file");
            let create = Command::new("docker")
                .args(["network", "create", network_name])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await;
            if let Ok(create) = create {
                if !create.status.success() {
                    tracing::warn!(
                        network = %network_name,
                        stderr = %String::from_utf8_lossy(&create.stderr),
                        "failed to create external network"
                    );
                }
            }
        }
    }

    async fn list_network_names() -> std::collections::HashSet<String> {
        let output = Command::new("docker")
            .args(["network", "ls", "--format", "{{.Name}}"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;
        match output {
            Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect(),
            _ => std::collections::HashSet::new(),
        }
    }

    pub async fn remove(&self, name: &str) -> Result<(), DeployError> {
        let work_dir = PathBuf::from("/var/lib/harbory-agent").join("compose").join(name);

        // Preferred path: run `down` from the project's own directory, where
        // compose can find its compose file. Running it bare (`-p name` from
        // an unrelated cwd) fails for *running* projects with "no
        // configuration file provided" — which is exactly why removals of
        // running stacks used to silently no-op while stopped ones (which
        // compose can resolve from container labels alone) went through.
        if work_dir.exists() {
            let output = Command::new("docker")
                .arg("compose")
                .arg("-p")
                .arg(name)
                .arg("down")
                .arg("-v")
                .current_dir(&work_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::warn!(%name, stderr = %stderr, "compose down from project dir failed, falling back to label sweep");
                self.label_sweep_remove(name).await;
            }
        } else {
            // Work dir already gone (e.g. removed by hand) — the containers
            // may still be running, so sweep by project label.
            self.label_sweep_remove(name).await;
        }

        self.errors.lock().await.remove(name);

        // Clean up the working directory
        if work_dir.exists() {
            let _ = tokio::fs::remove_dir_all(&work_dir).await;
        }

        Ok(())
    }

    /// File-less removal: every resource compose creates carries the
    /// `com.docker.compose.project` label, so force-removing by label
    /// converges even without any compose file on disk. Best-effort per
    /// resource type — a failure on one shouldn't block the others.
    async fn label_sweep_remove(&self, name: &str) {
        let project_label = format!("com.docker.compose.project={name}");

        if let Err(err) = self
            .sweep_resources(&["ps", "-aq", "--filter", &format!("label={project_label}")], &["rm", "-f"])
            .await
        {
            tracing::warn!(%name, %err, "label sweep: failed to remove compose containers");
        }

        if let Err(err) = self
            .sweep_resources(&["network", "ls", "-q", "--filter", &format!("label={project_label}")], &["network", "rm"])
            .await
        {
            tracing::warn!(%name, %err, "label sweep: failed to remove compose networks");
        }

        if let Err(err) = self
            .sweep_resources(&["volume", "ls", "-q", "--filter", &format!("label={project_label}")], &["volume", "rm"])
            .await
        {
            tracing::warn!(%name, %err, "label sweep: failed to remove compose volumes");
        }
    }

    /// Runs `docker <list_args>` and force-runs `docker <verb_args...> <id>`
    /// on each id it yields. Best-effort per id — one stuck resource
    /// shouldn't block the rest of the sweep.
    async fn sweep_resources(&self, list_args: &[&str], verb_args: &[&str]) -> Result<(), DeployError> {
        let listed = Command::new("docker")
            .args(list_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    DeployError::Command("'docker' executable not found on this host.".into())
                } else {
                    DeployError::Io(e)
                }
            })?;

        if !listed.status.success() {
            return Err(DeployError::Command(format!(
                "docker {} failed: {}",
                list_args.join(" "),
                String::from_utf8_lossy(&listed.stderr)
            )));
        }

        let stdout = String::from_utf8_lossy(&listed.stdout).to_string();
        let ids: Vec<&str> = stdout.lines().map(str::trim).filter(|l| !l.is_empty()).collect();

        for id in ids {
            let mut cmd = Command::new("docker");
            for arg in verb_args {
                cmd.arg(arg);
            }
            let output = cmd
                .arg(id)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(DeployError::Io)?;
            if !output.status.success() {
                tracing::warn!(
                    id = %id,
                    stderr = %String::from_utf8_lossy(&output.stderr),
                    "docker {} failed for one resource",
                    verb_args.join(" ")
                );
            }
        }
        Ok(())
    }

    /// Recent combined logs for a whole stack (`docker compose logs`).
    /// Run from the project dir when it still exists so compose can find
    /// its file; bare `-p name` works when the project's containers carry
    /// their compose labels even without one.
    pub async fn logs(&self, name: &str, tail: u32) -> Result<String, String> {
        let tail_str = if tail == 0 { "100".to_string() } else { tail.to_string() };
        let work_dir = PathBuf::from("/var/lib/harbory-agent").join("compose").join(name);

        let mut cmd = Command::new("docker");
        cmd.args(["compose", "-p", name, "logs", "--tail", &tail_str]);
        if work_dir.exists() {
            cmd.current_dir(&work_dir);
        }
        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    "'docker' executable not found on this host.".to_string()
                } else {
                    format!("failed to run docker compose logs: {e}")
                }
            })?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
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
