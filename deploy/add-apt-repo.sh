#!/usr/bin/env bash
# Registers Harbory's apt repository (key + source) so `apt install
# harbory-agent` works, and every install after this one is a plain
# `apt update && apt upgrade harbory-agent` — no re-running this script.
#
# This is the one-time bootstrap every third-party apt repo needs (same
# reason Docker's get-docker.sh and Google Chrome's/VS Code's installer
# .deb exist): apt refuses to trust a source it hasn't been told about,
# by design. This script just collapses that into one command instead of
# the two/three you'd otherwise run by hand — see deploy/apt-repo.md.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/PreetinderSinghBadesha/harbory/master/deploy/add-apt-repo.sh | sudo bash
#   sudo apt install harbory-agent
#
# Safe to re-run — every step is idempotent.
set -euo pipefail

APT_HOST="https://harbory-apt.preetindersingh.tech/apt"
KEYRING_PATH="/usr/share/keyrings/harbory-archive-keyring.gpg"
SOURCE_FILE="/etc/apt/sources.list.d/harbory.list"

log() { echo "==> $*"; }

if [ "$(id -u)" -ne 0 ]; then
  echo "must be run as root (writes to /usr/share/keyrings and /etc/apt) — try: curl ... | sudo bash" >&2
  exit 1
fi
if ! command -v curl >/dev/null 2>&1; then
  echo "curl not found — install it first (apt-get install curl)." >&2
  exit 1
fi
if ! command -v gpg >/dev/null 2>&1; then
  echo "gpg not found — install it first (apt-get install gnupg)." >&2
  exit 1
fi

log "Fetching and installing the repo signing key to $KEYRING_PATH"
curl -fsSL "$APT_HOST/harbory-archive-keyring.asc" | gpg --dearmor -o "$KEYRING_PATH"

log "Registering the repo at $SOURCE_FILE"
echo "deb [signed-by=$KEYRING_PATH] $APT_HOST stable main" > "$SOURCE_FILE"

log "Running apt update"
apt-get update

log "Done. Install (or upgrade) the agent with:"
echo "    sudo apt install harbory-agent"
