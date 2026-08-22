#!/usr/bin/env bash
# Installs harbory-control-plane as a systemd service running as a
# dedicated, non-root system user with its own writable data directory
# (for the Ed25519 signing key) instead of whatever directory happened to
# be the current one when someone ran the binary by hand.
#
# Usage:
#   ./install-control-plane.sh
#   (edit /etc/harbory/control-plane.env, then: sudo systemctl restart harbory-control-plane)
#
# Won't touch an already-installed harbory-control-plane.service — pass
# --force to replace it (only do this if you understand it may point a
# different user/env-file/working-directory at your existing deployment).
set -euo pipefail

REPO_URL="https://github.com/PreetinderSinghBadesha/harbory.git"
SERVICE_USER="harbory-control-plane"
DATA_DIR="/var/lib/harbory-control-plane"
ENV_FILE="/etc/harbory/control-plane.env"
BIN_PATH="/usr/local/bin/harbory-control-plane"
SERVICE_FILE="/etc/systemd/system/harbory-control-plane.service"

FORCE=0
for arg in "$@"; do
  case "$arg" in
    --force) FORCE=1 ;;
    *) echo "unknown argument: $arg" >&2; exit 1 ;;
  esac
done

log() { echo "==> $*"; }

if [ -f "$SERVICE_FILE" ] && [ "$FORCE" -ne 1 ]; then
  echo "$SERVICE_FILE already exists — refusing to overwrite an existing deployment." >&2
  echo "If you're intentionally replacing it (check its current User=/EnvironmentFile= first), re-run with --force." >&2
  exit 1
fi

if ! command -v sudo >/dev/null 2>&1; then
  echo "sudo is required (this script builds as your user, then uses sudo for system setup)" >&2
  exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found. Install Rust first: https://rustup.rs" >&2
  exit 1
fi

log "Building harbory-control-plane (cargo install --git, as $(whoami))"
cargo install --git "$REPO_URL" --force harbory-control-plane
CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin/harbory-control-plane"
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

log "Setting up data directory $DATA_DIR (holds the signing key — keep this private)"
sudo mkdir -p "$DATA_DIR"
sudo chown "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR"
sudo chmod 700 "$DATA_DIR"

sudo mkdir -p /etc/harbory
if [ ! -f "$ENV_FILE" ]; then
  log "Writing $ENV_FILE with placeholders — edit these before starting the service"
  sudo tee "$ENV_FILE" >/dev/null <<EOF
# Fill these in, then: sudo systemctl restart harbory-control-plane
DATABASE_URL=postgres://user:password@host:5432/dbname
# At least one of the next two is required (Supabase signs session JWTs
# with a legacy HS256 secret on older projects, or ES256/JWKS on newer
# ones — see docs/dashboard.md):
#SUPABASE_JWT_SECRET=
#SUPABASE_URL=https://your-project.supabase.co

GRPC_LISTEN_ADDR=127.0.0.1:50051
HTTP_LISTEN_ADDR=127.0.0.1:8080
CONTROL_PLANE_SIGNING_KEY_PATH=$DATA_DIR/signing-key
EOF
  NEEDS_CONFIG=1
else
  log "$ENV_FILE already exists, leaving it as-is"
  NEEDS_CONFIG=0
fi
sudo chown root:root "$ENV_FILE"
sudo chmod 640 "$ENV_FILE"

log "Installing systemd unit"
sudo tee "$SERVICE_FILE" >/dev/null <<EOF
[Unit]
Description=Harbory control plane
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
WorkingDirectory=$DATA_DIR
EnvironmentFile=$ENV_FILE
ExecStart=$BIN_PATH
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable harbory-control-plane.service

if [ "${NEEDS_CONFIG:-0}" -eq 1 ]; then
  log "Edit $ENV_FILE with your real DATABASE_URL/SUPABASE_* values, then run:"
  echo "    sudo systemctl start harbory-control-plane"
else
  log "Restarting harbory-control-plane.service"
  sudo systemctl restart harbory-control-plane.service
fi

log "Done. Check status with: systemctl status harbory-control-plane.service"
