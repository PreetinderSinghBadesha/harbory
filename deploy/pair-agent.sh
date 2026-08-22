#!/usr/bin/env bash
# Pairs an already-installed harbory-agent (see install-agent.sh) with a
# pairing token and (re)starts its systemd service. Separate from install
# so pairing/re-pairing never requires rebuilding or re-touching system
# permissions — install once, pair as many times as you need.
#
# Usage:
#   sudo harbory-agent-pair <pairing-token>
#   sudo harbory-agent-pair --force <pairing-token>   # wipe stored credential, re-pair
set -euo pipefail

SERVICE_USER="harbory-agent"
DATA_DIR="/var/lib/harbory-agent"
ENV_FILE="/etc/harbory/agent.env"
BIN_PATH="/usr/local/bin/harbory-agent"
CREDENTIAL_PATH="$DATA_DIR/agent-credential"

FORCE=0
PAIRING_TOKEN=""
for arg in "$@"; do
  case "$arg" in
    --force) FORCE=1 ;;
    -*) echo "unknown flag: $arg" >&2; exit 1 ;;
    *) PAIRING_TOKEN="$arg" ;;
  esac
done

log() { echo "==> $*"; }

if ! command -v sudo >/dev/null 2>&1; then
  echo "sudo is required" >&2
  exit 1
fi
if ! id "$SERVICE_USER" >/dev/null 2>&1 || [ ! -f "$ENV_FILE" ]; then
  echo "harbory-agent isn't installed yet — run install-agent.sh first:" >&2
  echo "  curl -fsSL https://raw.githubusercontent.com/PreetinderSinghBadesha/harbory/master/deploy/install-agent.sh | bash" >&2
  exit 1
fi

if [ "$FORCE" -eq 1 ]; then
  log "--force: removing stored credential to force re-pairing"
  sudo rm -f "$CREDENTIAL_PATH"
fi

if [ -f "$CREDENTIAL_PATH" ]; then
  log "Already paired (credential exists at $CREDENTIAL_PATH) — pass --force to re-pair with a new token"
else
  if [ -z "$PAIRING_TOKEN" ]; then
    echo "no stored credential and no pairing token given — usage: $0 <pairing-token>" >&2
    exit 1
  fi

  # shellcheck disable=SC1090
  set -a; source "$ENV_FILE"; set +a
  CONTROL_PLANE_ADDR="${CONTROL_PLANE_ADDR:-https://harbory-client.preetindersingh.tech}"

  log "Pairing with $CONTROL_PLANE_ADDR"
  sudo -u "$SERVICE_USER" env \
    CONTROL_PLANE_ADDR="$CONTROL_PLANE_ADDR" \
    AGENT_KEY_PATH="$DATA_DIR/agent-key" \
    AGENT_CREDENTIAL_PATH="$CREDENTIAL_PATH" \
    "$BIN_PATH" "$PAIRING_TOKEN" &
  PAIR_PID=$!
  for _ in $(seq 1 30); do
    if [ -f "$CREDENTIAL_PATH" ]; then break; fi
    sleep 1
  done
  kill "$PAIR_PID" 2>/dev/null || true
  wait "$PAIR_PID" 2>/dev/null || true
  if [ ! -f "$CREDENTIAL_PATH" ]; then
    echo "pairing did not complete within 30s — check the token (they're single-use and expire) and CONTROL_PLANE_ADDR" >&2
    exit 1
  fi
  log "Paired. Credential stored at $CREDENTIAL_PATH"
fi

log "Enabling and (re)starting harbory-agent.service"
sudo systemctl enable harbory-agent.service
sudo systemctl restart harbory-agent.service

log "Done. Check status with: systemctl status harbory-agent.service"
