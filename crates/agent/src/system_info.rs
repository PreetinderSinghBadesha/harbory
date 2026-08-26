use bollard::Docker;
use harbory_protocol::v1::SystemInfoResponse;

/// Host resource snapshot for the dashboard's "system" panel. Pulls most
/// fields from `docker info` (already the daemon this agent talks to
/// everywhere else) and fills in the handful it doesn't report — disk usage,
/// primary outbound IP, uptime, CPU model — from `/proc` and a stdlib UDP
/// trick. Linux-only, matching the rest of the agent (systemd, apt/yum,
/// nginx paths already assume it).
pub struct SystemInfoManager {
    docker: Docker,
}

impl SystemInfoManager {
    pub fn connect() -> Result<Self, bollard::errors::Error> {
        Ok(Self { docker: Docker::connect_with_local_defaults()? })
    }

    pub async fn snapshot(&self, request_id: String) -> SystemInfoResponse {
        let mut resp = SystemInfoResponse { request_id, ..Default::default() };

        match self.docker.info().await {
            Ok(info) => {
                resp.hostname = info.name.unwrap_or_default();
                resp.os = info.operating_system.unwrap_or_default();
                resp.kernel_version = info.kernel_version.unwrap_or_default();
                resp.docker_version = info.server_version.unwrap_or_default();
                resp.cpu_count = info.ncpu.unwrap_or(0) as u32;
                resp.mem_total_bytes = info.mem_total.unwrap_or(0) as u64;
            }
            Err(err) => {
                resp.error = format!("failed to query Docker: {err}");
            }
        }

        let (total, used, free) = disk_usage("/var/lib/harbory-agent");
        resp.disk_total_bytes = total;
        resp.disk_used_bytes = used;
        resp.disk_free_bytes = free;

        resp.primary_ip = primary_ip().unwrap_or_default();
        resp.uptime_seconds = uptime_seconds().unwrap_or(0);
        resp.cpu_model = cpu_model().unwrap_or_default();

        resp
    }
}

/// Disk usage for the filesystem containing `path`, via `statvfs` (through
/// `df`'s underlying syscall — no extra crate needed since we already shell
/// out elsewhere in this codebase). Falls back to a zeroed triple on any
/// failure (path missing, non-Linux) rather than erroring the whole
/// snapshot.
fn disk_usage(path: &str) -> (u64, u64, u64) {
    use std::process::Command;
    let output = Command::new("df").args(["-B1", "--output=size,used,avail", path]).output();
    let Ok(output) = output else { return (0, 0, 0) };
    if !output.status.success() {
        return (0, 0, 0);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let Some(data_line) = text.lines().nth(1) else { return (0, 0, 0) };
    let mut parts = data_line.split_whitespace();
    let total = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let used = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let free = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (total, used, free)
}

/// The IP this host would use to reach the internet — connecting a UDP
/// socket doesn't send any packets, it just asks the kernel to pick the
/// route/local address it would use, which is the outbound-facing IP in
/// the common single-NIC case.
fn primary_ip() -> Option<String> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip().to_string())
}

fn uptime_seconds() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/uptime").ok()?;
    let first = text.split_whitespace().next()?;
    first.parse::<f64>().ok().map(|v| v as u64)
}

fn cpu_model() -> Option<String> {
    let text = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    text.lines()
        .find(|l| l.starts_with("model name"))
        .and_then(|l| l.split(':').nth(1))
        .map(|s| s.trim().to_string())
}
