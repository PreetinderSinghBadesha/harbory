use std::fmt::Write as _;
use std::path::PathBuf;

use harbory_protocol::v1::ProxyRoute;
use tokio::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("failed to write nginx config: {0}")]
    Io(#[from] std::io::Error),
    #[error("nginx config validation failed: {0}")]
    Validation(String),
    #[error("nginx reload failed: {0}")]
    Reload(String),
}

/// One route = one Nginx `server{}` block. v1 does not merge multiple
/// routes sharing a server_name+listen_port into one block with several
/// `location`s — see /docs/proxy-management.md for why, and for the
/// operator-facing consequence (keep server_name distinct per route).
fn render_route(route: &ProxyRoute) -> Option<String> {
    // Defense-in-depth: even if a route bypasses the control plane's
    // validator (issue #7), never emit an Nginx config that could carry an
    // injected directive — doing so is host file disclosure + SSRF. Skip
    // any route whose fields aren't plain hostname/path/host values.
    if !route_is_safe(route) {
        return None;
    }

    let server_name = if route.server_name.is_empty() { "_" } else { route.server_name.as_str() };
    let path_prefix = if route.path_prefix.is_empty() { "/" } else { route.path_prefix.as_str() };

    let mut out = String::new();
    let _ = writeln!(out, "server {{");
    let _ = writeln!(out, "    listen {};", route.listen_port);
    let _ = writeln!(out, "    server_name {server_name};");
    let _ = writeln!(out);
    let _ = writeln!(out, "    location {path_prefix} {{");
    let _ = writeln!(out, "        proxy_pass http://{}:{};", route.upstream_host, route.upstream_port);
    let _ = writeln!(out, "        proxy_set_header Host $host;");
    let _ = writeln!(out, "        proxy_set_header X-Real-IP $remote_addr;");
    let _ = writeln!(out, "        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;");
    let _ = writeln!(out, "        proxy_set_header X-Forwarded-Proto $scheme;");
    let _ = writeln!(out, "    }}");
    let _ = writeln!(out, "}}");
    Some(out)
}

/// Mirrors the control plane's validator: only plain hostname / URL path /
/// host values may ever reach the Nginx config. Rejects anything containing
/// `;`, `{`, `}`, whitespace, quotes, or newlines. See issue #7.
fn route_is_safe(route: &ProxyRoute) -> bool {
    const MAX_LEN: usize = 255;

    if route.server_name.len() > MAX_LEN || route.path_prefix.len() > MAX_LEN || route.upstream_host.len() > MAX_LEN {
        return false;
    }

    // server_name and path_prefix are optional — empty means "catch-all"
    // (`server_name _;`) and "/" respectively, so those are allowed.
    let server_name = route.server_name.trim_start_matches("*.");
    if !route.server_name.is_empty()
        && (server_name.is_empty()
            || !server_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_'))
    {
        return false;
    }

    if !route.path_prefix.is_empty()
        && (!route.path_prefix.starts_with('/')
            || !route
                .path_prefix
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "/._~-".contains(c)))
    {
        return false;
    }

    if route.upstream_host.is_empty()
        || !route.upstream_host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return false;
    }

    true
}

pub fn render(routes: &[ProxyRoute]) -> String {
    let mut out = String::from("# Managed by Harbory — do not edit directly, changes will be overwritten.\n\n");
    for route in routes {
        if let Some(block) = render_route(route) {
            out.push_str(&block);
            out.push('\n');
        }
    }
    out
}

pub struct ProxyManager {
    nginx_binary: String,
    config_path: PathBuf,
    /// Serializes concurrent applies — "concurrent config changes should
    /// serialize, not clobber" per the roadmap. In practice ProxyConfig
    /// commands only ever arrive one at a time from the single-threaded
    /// per-connection message loop, but this makes that guarantee
    /// structural rather than incidental.
    lock: Mutex<()>,
}

impl ProxyManager {
    pub fn new(nginx_binary: impl Into<String>, config_path: impl Into<PathBuf>) -> Self {
        Self { nginx_binary: nginx_binary.into(), config_path: config_path.into(), lock: Mutex::new(()) }
    }

    /// Validate-before-apply with rollback, graceful reload:
    /// 1. Back up whatever's currently on disk at `config_path`.
    /// 2. Write the newly rendered config in its place.
    /// 3. `nginx -t` — this tests the *real* effective config (main
    ///    nginx.conf + all its includes), which is why we write to the
    ///    real path rather than a shadow copy: a shadow copy wouldn't be
    ///    included by the running nginx.conf, so `-t` wouldn't actually
    ///    validate our content.
    /// 4. On failure: restore the backup (or delete the file if there was
    ///    none), return the error. No reload is issued, so the running
    ///    nginx workers — still serving the old in-memory config — are
    ///    completely unaffected by the brief window where the on-disk
    ///    file held invalid content.
    /// 5. On success: `nginx -s reload` (graceful — existing connections
    ///    drain, new workers start with the new config), never a restart.
    pub async fn apply(&self, routes: &[ProxyRoute]) -> Result<(), ProxyError> {
        let _guard = self.lock.lock().await;

        let new_content = render(routes);
        let previous = tokio::fs::read_to_string(&self.config_path).await.ok();

        if let Some(parent) = self.config_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&self.config_path, &new_content).await?;

        if let Err(err) = self.run_nginx(&["-t"]).await {
            match &previous {
                Some(prev) => {
                    let _ = tokio::fs::write(&self.config_path, prev).await;
                }
                None => {
                    let _ = tokio::fs::remove_file(&self.config_path).await;
                }
            }
            return Err(ProxyError::Validation(err));
        }

        self.run_nginx(&["-s", "reload"]).await.map_err(ProxyError::Reload)
    }

    async fn run_nginx(&self, args: &[&str]) -> Result<(), String> {
        let output = tokio::process::Command::new(&self.nginx_binary)
            .args(args)
            .output()
            .await
            .map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    format!(
                        "nginx binary '{}' not found on this host — install nginx (or fix NGINX_BINARY_PATH \
                         in /etc/harbory/agent.env), then restart harbory-agent. Remove this agent's proxy \
                         routes if it should stay container-only.",
                        self.nginx_binary
                    )
                } else {
                    format!("failed to run '{}': {err}", self.nginx_binary)
                }
            })?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(name: &str) -> ProxyRoute {
        ProxyRoute {
            name: name.into(),
            server_name: "app.example.test".into(),
            listen_port: 80,
            path_prefix: "/".into(),
            upstream_host: "127.0.0.1".into(),
            upstream_port: 8080,
        }
    }

    #[test]
    fn renders_expected_server_block() {
        let output = render(&[route("web")]);
        assert!(output.contains("listen 80;"));
        assert!(output.contains("server_name app.example.test;"));
        assert!(output.contains("location / {"));
        assert!(output.contains("proxy_pass http://127.0.0.1:8080;"));
    }

    #[test]
    fn empty_server_name_becomes_catchall() {
        let mut r = route("web");
        r.server_name = String::new();
        let output = render(&[r]);
        assert!(output.contains("server_name _;"));
    }

    #[test]
    fn empty_path_prefix_defaults_to_slash() {
        let mut r = route("web");
        r.path_prefix = String::new();
        let output = render(&[r]);
        assert!(output.contains("location / {"));
    }

    #[test]
    fn braces_are_balanced_for_multiple_routes() {
        let output = render(&[route("web"), route("api")]);
        let opens = output.matches('{').count();
        let closes = output.matches('}').count();
        assert_eq!(opens, closes);
        assert_eq!(opens, 4); // 2 routes * (server + location)
    }

    #[test]
    fn empty_route_set_renders_just_the_header_comment() {
        let output = render(&[]);
        assert!(!output.contains("server {"));
    }

    #[test]
    fn unsafe_host_file_injection_is_not_rendered() {
        let mut r = route("evil");
        r.server_name = "evil.com;\nlocation /etc-secret { alias /etc/; return 200; }".into();
        let output = render(&[r]);
        assert!(!output.contains("location /etc-secret"));
        assert!(!output.contains("evil.com;"));
        assert!(!output.contains("alias /etc/;"));
    }

    #[test]
    fn unsafe_upstream_host_is_not_rendered() {
        let mut r = route("ssrf");
        r.upstream_host = "169.254.169.254; return".into();
        let output = render(&[r]);
        assert!(!output.contains("169.254.169.254"));
    }

    #[test]
    fn unsafe_path_prefix_is_not_rendered() {
        let mut r = route("path");
        r.path_prefix = "/api { return 200; }".into();
        let output = render(&[r]);
        assert!(!output.contains("return 200"));
    }

    #[test]
    fn unsafe_route_does_not_break_adjacent_safe_routes() {
        let output = render(&[route("evil_but_safe"), route("good")]);
        assert!(output.contains("server_name app.example.test;"));
    }

    #[test]
    fn overlong_fields_are_rejected() {
        let mut r = route("long");
        r.server_name = "a".repeat(256);
        assert!(!route_is_safe(&r));
    }

    #[test]
    fn valid_route_is_safe() {
        assert!(route_is_safe(&route("web")));
        let mut r = route("wild");
        r.server_name = "*.example.test".into();
        assert!(route_is_safe(&r));
    }
}
