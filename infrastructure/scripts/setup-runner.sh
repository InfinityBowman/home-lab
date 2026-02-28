#!/usr/bin/env bash
#
# HomeLab GitHub Actions Runner Setup
# Run once on the Ubuntu laptop to register a self-hosted runner.
#
# Prerequisites:
#   - setup-host.sh has been run (Docker, user in docker group)
#   - A runner registration token from:
#     GitHub > Repo Settings > Actions > Runners > New self-hosted runner
#
# Usage:
#   ./setup-runner.sh <RUNNER_TOKEN>
#
set -euo pipefail

# ─── Configuration ──────────────────────────────────────────────────────────
HOMELAB_DIR="/opt/homelab"
RUNNER_DIR="${HOMELAB_DIR}/actions-runner"
REPO_DIR="${HOMELAB_DIR}/repo"
GITHUB_REPO="InfinityBowman/home-lab"
RUNNER_VERSION="2.332.0"
RUNNER_ARCH="linux-x64"
RUNNER_TARBALL="actions-runner-${RUNNER_ARCH}-${RUNNER_VERSION}.tar.gz"
RUNNER_URL="https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/${RUNNER_TARBALL}"

# ─── Colors ─────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log()  { echo -e "${GREEN}[+]${NC} $1"; }
warn() { echo -e "${YELLOW}[!]${NC} $1"; }
err()  { echo -e "${RED}[x]${NC} $1"; exit 1; }

# ─── Preflight ──────────────────────────────────────────────────────────────
if [[ $# -lt 1 ]]; then
    err "Usage: $0 <RUNNER_TOKEN>"
fi

RUNNER_TOKEN="$1"

if [[ $EUID -eq 0 ]]; then
    err "Do NOT run this as root. Run as your normal user (who is in the docker group)."
fi

if ! docker info &>/dev/null; then
    err "Cannot connect to Docker. Is your user in the docker group? Log out and back in."
fi

echo ""
echo "══════════════════════════════════════════════════"
echo "  GitHub Actions Self-Hosted Runner Setup"
echo "══════════════════════════════════════════════════"
echo ""

# ─── 1. Clone repo if needed ───────────────────────────────────────────────
if [[ -d "${REPO_DIR}/.git" ]]; then
    log "Repo already cloned at ${REPO_DIR}"
else
    log "Cloning repo to ${REPO_DIR}..."
    sudo mkdir -p "${REPO_DIR}"
    sudo chown "$(whoami):$(whoami)" "${REPO_DIR}"
    git clone "https://github.com/${GITHUB_REPO}.git" "${REPO_DIR}"
    log "Repo cloned."
fi

# ─── 2. Download runner ────────────────────────────────────────────────────
if [[ -f "${RUNNER_DIR}/config.sh" ]]; then
    warn "Runner directory already exists at ${RUNNER_DIR}. Skipping download."
else
    log "Creating runner directory at ${RUNNER_DIR}..."
    sudo mkdir -p "${RUNNER_DIR}"
    sudo chown "$(whoami):$(whoami)" "${RUNNER_DIR}"

    log "Downloading GitHub Actions Runner v${RUNNER_VERSION}..."
    curl -sL "${RUNNER_URL}" -o "/tmp/${RUNNER_TARBALL}"

    log "Extracting runner..."
    tar xzf "/tmp/${RUNNER_TARBALL}" -C "${RUNNER_DIR}"
    rm -f "/tmp/${RUNNER_TARBALL}"
fi

# ─── 3. Configure runner ───────────────────────────────────────────────────
log "Configuring runner for ${GITHUB_REPO}..."
cd "${RUNNER_DIR}"

./config.sh \
    --url "https://github.com/${GITHUB_REPO}" \
    --token "${RUNNER_TOKEN}" \
    --name "homelab-laptop" \
    --labels "self-hosted,homelab" \
    --work "${RUNNER_DIR}/_work" \
    --unattended \
    --replace

log "Runner configured."

# ─── 4. Install as systemd service ─────────────────────────────────────────
log "Installing runner as systemd service..."
sudo ./svc.sh install "$(whoami)"
sudo ./svc.sh start
log "Runner service started."

# ─── 5. Verify ─────────────────────────────────────────────────────────────
echo ""
echo "══════════════════════════════════════════════════"
echo "  Runner setup complete!"
echo "══════════════════════════════════════════════════"
echo ""
log "Runner name:   homelab-laptop"
log "Labels:        self-hosted, homelab"
log "Runner dir:    ${RUNNER_DIR}"
log "Repo clone:    ${REPO_DIR}"
echo ""
warn "Next steps:"
echo "  1. Copy .env.example files and fill in your secrets:"
echo "     cp ${REPO_DIR}/infrastructure/.env.example ${REPO_DIR}/infrastructure/.env"
echo "     cp ${REPO_DIR}/services/n8n/.env.example ${REPO_DIR}/services/n8n/.env"
echo "  2. Edit the .env files with your actual values."
echo "  3. Push to main — the deploy job will run automatically."
echo ""
