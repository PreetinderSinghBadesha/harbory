#!/usr/bin/env bash
# Installs harbory-agent as a systemd service with the least privilege it
# actually needs to do its job:
#   - membership in the `docker` group (ContainerManager needs the socket)
#   - ownership of exactly one pre-created nginx config file (so it can
#     write its managed config without write access to all of /etc/nginx)
#   - a sudoers rule scoped to exactly `nginx -t` and `nginx -s reload`
#     (reload requires signalling the root-owned nginx master process;
#     nothing short of root or an equivalent sudo grant can do that)
#
# This script only sets things up — it does not pair or start the service,
# since that needs a pairing token you may not have yet. Once it's done,
# pair (and as many times after as you like) with:
#   sudo harbory-agent-pair <pairing-token>
#
# Usage:
#   ./install-agent.sh [--control-plane-addr=https://host]
#
# Safe to re-run: every step is idempotent. Must run as a user with sudo
# access (builds the binary as the invoking user via `cargo`, then uses
# `sudo` only for the system-level setup steps below).
set -euo pipefail

REPO_URL="https://github.com/PreetinderSinghBadesha/harbory.git"
PAIR_SCRIPT_URL="https://raw.githubusercontent.com/PreetinderSinghBadesha/harbory/master/deploy/pair-agent.sh"
SERVICE_USER="harbory-agent"
DATA_DIR="/var/lib/harbory-agent"
ENV_FILE="/etc/harbory/agent.env"
BIN_PATH="/usr/local/bin/harbory-agent"
PAIR_BIN_PATH="/usr/local/bin/harbory-agent-pair"
NGINX_CONF="/etc/nginx/conf.d/harbory.conf"
NGINX_WRAPPER="/usr/local/bin/harbory-nginx-ctl"
SUDOERS_FILE="/etc/sudoers.d/harbory-agent-nginx"
SERVICE_FILE="/etc/systemd/system/harbory-agent.service"
CONTROL_PLANE_ADDR="${CONTROL_PLANE_ADDR:-https://harbory-client.preetindersingh.tech}"

for arg in "$@"; do
  case "$arg" in
    --control-plane-addr=*) CONTROL_PLANE_ADDR="${arg#*=}" ;;
    *) echo "unknown argument: $arg" >&2; exit 1 ;;
  esac
done

log() { echo "==> $*"; }

if ! command -v sudo >/dev/null 2>&1; then
  echo "sudo is required (this script builds as your user, then uses sudo for system setup)" >&2
  exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found. Install Rust first: https://rustup.rs" >&2
  exit 1
fi
if ! command -v docker >/dev/null 2>&1; then
  echo "docker not found. harbory-agent requires Docker and fails fast without it — install Docker first." >&2
  exit 1
fi
DOCKER_BIN="$(command -v docker)"
if ! sudo "$DOCKER_BIN" info >/dev/null 2>&1; then
  echo "docker is installed but not usable via sudo — is the docker daemon running?" >&2
  exit 1
fi
if ! command -v curl >/dev/null 2>&1; then
  echo "curl not found — needed to fetch the pairing script." >&2
  exit 1
fi

MISSING_PKGS=()
if ! command -v nginx >/dev/null 2>&1; then
  log "nginx not found. Attempting to install..."
  MISSING_PKGS+=(nginx)
fi
if ! command -v git >/dev/null 2>&1; then
  log "git not found. Installing it too — git-sourced container deploys shell out to it."
  MISSING_PKGS+=(git)
fi

if [ "${#MISSING_PKGS[@]}" -gt 0 ]; then
  if command -v apt-get >/dev/null 2>&1; then
    # A single stale third-party repo/PPA makes `apt-get update` fail
    # wholesale even when everything we need is fetchable from healthy
    # repos — treat an update failure as a warning and let the actual
    # install step be the thing that decides success or failure.
    if ! sudo apt-get update; then
      echo "WARNING: 'apt-get update' failed (often a broken third-party PPA) — attempting install anyway" >&2
    fi
    sudo apt-get install -y "${MISSING_PKGS[@]}"
  elif command -v yum >/dev/null 2>&1; then
    sudo yum install -y "${MISSING_PKGS[@]}"
  else
    echo "Could not automatically install: ${MISSING_PKGS[*]}. Please install them manually first." >&2
    exit 1
  fi
fi
HAVE_NGINX=1

log "Building harbory-agent (cargo install --git, as $(whoami))"
cargo install --git "$REPO_URL" --force harbory-agent
CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin/harbory-agent"
if [ ! -x "$CARGO_BIN" ]; then
  echo "expected built binary at $CARGO_BIN, not found" >&2
  exit 1
fi

log "Installing binary to $BIN_PATH"
sudo install -o root -g root -m 755 "$CARGO_BIN" "$BIN_PATH"

log "Ensuring service user '$SERVICE_USER' exists"
if ! id "$SERVICE_USER" >/dev/null 2>&1; then
  sudo useradd --system --home-dir "$DATA_DIR" --create-home --shell /usr/sbin/nologin "$SERVICE_USER"
fi

log "Adding '$SERVICE_USER' to the docker group"
sudo usermod -aG docker "$SERVICE_USER"

log "Setting up data directory $DATA_DIR"
sudo mkdir -p "$DATA_DIR"
sudo chown "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR"
sudo chmod 700 "$DATA_DIR"

sudo mkdir -p /etc/harbory
if [ ! -f "$ENV_FILE" ]; then
  log "Writing $ENV_FILE"
  sudo tee "$ENV_FILE" >/dev/null <<EOF
CONTROL_PLANE_ADDR=$CONTROL_PLANE_ADDR
AGENT_KEY_PATH=$DATA_DIR/agent-key
AGENT_CREDENTIAL_PATH=$DATA_DIR/agent-credential
EOF
else
  log "$ENV_FILE already exists, leaving it as-is"
fi
sudo chown root:root "$ENV_FILE"
sudo chmod 640 "$ENV_FILE"

if [ "$HAVE_NGINX" -eq 1 ]; then
  log "Setting up nginx permissions (config write + scoped reload)"
  sudo mkdir -p "$(dirname "$NGINX_CONF")"
  if [ ! -f "$NGINX_CONF" ]; then
    sudo tee "$NGINX_CONF" >/dev/null <<'EOF'
# Managed by Harbory — do not edit directly, changes will be overwritten.
EOF
  fi
  sudo chown "$SERVICE_USER:$SERVICE_USER" "$NGINX_CONF"
  sudo chmod 644 "$NGINX_CONF"

  NGINX_BIN="$(command -v nginx)"
  sudo tee "$NGINX_WRAPPER" >/dev/null <<EOF
#!/bin/sh
# Lets $SERVICE_USER run exactly two nginx invocations as root, via the
# sudoers rule in $SUDOERS_FILE. Nothing broader: reloading nginx means
# signalling its root-owned master process, so *some* privilege escalation
# is unavoidable — this is the narrowest one that still works.
exec sudo -n "$NGINX_BIN" "\$@"
EOF
  sudo chmod 755 "$NGINX_WRAPPER"

  SUDOERS_CONTENT="$SERVICE_USER ALL=(root) NOPASSWD: $NGINX_BIN -t, $NGINX_BIN -s reload"
  TMP_SUDOERS="$(mktemp)"
  echo "$SUDOERS_CONTENT" > "$TMP_SUDOERS"
  if sudo visudo -c -f "$TMP_SUDOERS" >/dev/null 2>&1; then
    sudo install -o root -g root -m 440 "$TMP_SUDOERS" "$SUDOERS_FILE"
  else
    rm -f "$TMP_SUDOERS"
    echo "generated sudoers rule failed validation, skipping nginx reload permission setup" >&2
    HAVE_NGINX=0
  fi
  rm -f "$TMP_SUDOERS"

  if [ -f "$SUDOERS_FILE" ]; then
    log "Nginx reload permission installed at $SUDOERS_FILE (scoped to nginx -t / nginx -s reload)"
    # Idempotent set, not append-if-missing — a stale value from an
    # earlier run (or a manual edit) would otherwise never get corrected,
    # since a rerun would see the key already present and skip it.
    sudo sed -i '/^NGINX_BINARY_PATH=/d' "$ENV_FILE"
    echo "NGINX_BINARY_PATH=$NGINX_WRAPPER" | sudo tee -a "$ENV_FILE" >/dev/null

    # Self-test the full chain right now — service user -> wrapper ->
    # sudo -> nginx -t — rather than letting a broken link only surface
    # later as an opaque "Last apply error" on the dashboard with no
    # access to this host's logs to debug it from.
    log "Verifying $SERVICE_USER can reach nginx through the wrapper..."
    if sudo -u "$SERVICE_USER" "$NGINX_WRAPPER" -t 2>/tmp/harbory-nginx-selftest.log; then
      log "OK: $SERVICE_USER -> $NGINX_WRAPPER -> nginx -t works."
    else
      echo "FAILED: '$SERVICE_USER' could not run nginx -t through $NGINX_WRAPPER. Output:" >&2
      cat /tmp/harbory-nginx-selftest.log >&2
      echo "Proxy-route deploys will fail until this is fixed — check the sudoers rule ($SUDOERS_FILE) and $NGINX_WRAPPER." >&2
    fi
    rm -f /tmp/harbory-nginx-selftest.log
  fi
else
  log "nginx not installed — skipping proxy-route permission setup (agent only touches nginx if a ProxyConfig is ever sent to it)"
fi

log "Installing systemd unit (not started yet — needs pairing first)"
sudo tee "$SERVICE_FILE" >/dev/null <<EOF
[Unit]
Description=Harbory agent
After=network-online.target docker.service
Wants=network-online.target

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
WorkingDirectory=$DATA_DIR
EnvironmentFile=$ENV_FILE
# Pre-flight: the binary itself already fails fast if Docker isn't
# reachable (container management is its whole job — see main.rs), but
# that shows up as a Rust panic buried in the journal after a process
# actually started. This fails at the same point, before that, so
# 'systemctl status' shows Docker's own error directly instead — and
# runs as $SERVICE_USER (same as ExecStart), so it's testing the exact
# access path the real process uses, not just whether Docker exists.
ExecStartPre=$DOCKER_BIN info
ExecStart=$BIN_PATH
Restart=always
RestartSec=2
# NoNewPrivileges must stay off (systemd's default) — the nginx reload
# wrapper needs to exec sudo, which needs the setuid escalation this would block.

[Install]
WantedBy=multi-user.target
EOF
sudo systemctl daemon-reload

log "Installing pairing helper to $PAIR_BIN_PATH"
sudo curl -fsSL "$PAIR_SCRIPT_URL" -o "$PAIR_BIN_PATH"
sudo chmod 755 "$PAIR_BIN_PATH"

log "Setup complete. Not paired yet — get a pairing token from the dashboard, then run:"
echo "    sudo harbory-agent-pair <pairing-token>"
