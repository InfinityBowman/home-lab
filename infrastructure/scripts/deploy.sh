#!/usr/bin/env bash
#
# HomeLab Deploy Script
# Called by the GitHub Actions deploy job on the self-hosted runner.
# Pulls latest code and rebuilds/restarts infrastructure and services.
#
set -euo pipefail

# ─── Configuration ──────────────────────────────────────────────────────────
REPO_DIR="/opt/homelab/repo"
INFRA_DIR="${REPO_DIR}/infrastructure"
SERVICES_DIR="${REPO_DIR}/services"

# ─── Colors ─────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log()  { echo -e "${GREEN}[deploy]${NC} $1"; }
warn() { echo -e "${YELLOW}[deploy]${NC} $1"; }
fail() { echo -e "${RED}[deploy]${NC} $1"; exit 1; }

# ─── Preflight ──────────────────────────────────────────────────────────────
if [[ ! -d "${REPO_DIR}/.git" ]]; then
    fail "Repo not found at ${REPO_DIR}. Run setup-runner.sh first."
fi

echo ""
echo "══════════════════════════════════════════════════"
echo "  HomeLab Deploy"
echo "  $(date '+%Y-%m-%d %H:%M:%S')"
echo "══════════════════════════════════════════════════"
echo ""

# ─── 1. Pull latest code ───────────────────────────────────────────────────
log "Pulling latest code..."
cd "${REPO_DIR}"

BEFORE_SHA=$(git rev-parse HEAD)

git fetch origin main
git reset --hard origin/main

AFTER_SHA=$(git rev-parse HEAD)
log "Updated: ${BEFORE_SHA:0:8} -> ${AFTER_SHA:0:8}"

# ─── 2. Clean up stale containers from interrupted deploys ────────────────
# Docker renames old containers with a hash prefix during recreation.
# If a previous deploy was interrupted, these stick around and block the next one.
docker container prune -f --filter "label=com.docker.compose.project=infrastructure" >/dev/null 2>&1 || true

# ─── 3. Build and deploy infrastructure ────────────────────────────────────
log "Building infrastructure..."
cd "${INFRA_DIR}"

# Force no-cache rebuild if Rust source or Dockerfile changed to avoid
# BuildKit serving stale layers.
BUILD_ARGS=""
if ! git diff --quiet "${BEFORE_SHA}" "${AFTER_SHA}" -- "../crates/" "../Cargo.toml" "../Cargo.lock" "Dockerfile"; then
    log "Rust source changed — building without cache..."
    BUILD_ARGS="--no-cache"
fi

# Build the image first, separately. If the build fails, old containers
# keep running and the script exits with an error.
if docker compose build $BUILD_ARGS; then
    log "Build succeeded. Deploying..."
    docker compose up -d
    log "Infrastructure containers updated."
else
    fail "Docker build failed. Old containers are still running. Fix and retry."
fi

# ─── 4. Update services if changed ─────────────────────────────────────────
if [[ -d "${SERVICES_DIR}" ]]; then
    for service_dir in "${SERVICES_DIR}"/*/; do
        service_name=$(basename "${service_dir}")
        compose_file="${service_dir}docker-compose.yml"

        if [[ ! -f "${compose_file}" ]]; then
            warn "Skipping ${service_name}: no docker-compose.yml"
            continue
        fi

        if git diff --quiet "${BEFORE_SHA}" "${AFTER_SHA}" -- "services/${service_name}/"; then
            log "Service ${service_name}: no changes, skipping."
        else
            log "Service ${service_name}: changes detected, updating..."
            cd "${service_dir}"
            docker compose up -d
            cd "${REPO_DIR}"
            log "Service ${service_name} updated."
        fi
    done
else
    log "No services directory found, skipping."
fi

# ─── 5. Apply Terraform (Cloudflare zone config) ─────────────────────────────
TERRAFORM_DIR="${INFRA_DIR}/terraform"
if [[ -d "${TERRAFORM_DIR}" ]] && command -v terraform &>/dev/null; then
    if git diff --quiet "${BEFORE_SHA}" "${AFTER_SHA}" -- "infrastructure/terraform/"; then
        log "Terraform: no changes, skipping."
    else
        log "Terraform: changes detected, applying..."
        cd "${TERRAFORM_DIR}"
        terraform init -input=false
        terraform apply -auto-approve -input=false
        cd "${REPO_DIR}"
        log "Terraform applied."
    fi
else
    if [[ -d "${TERRAFORM_DIR}" ]]; then
        warn "Terraform directory found but 'terraform' not installed, skipping."
    fi
fi

# ─── 6. Cleanup ─────────────────────────────────────────────────────────────
log "Pruning dangling images..."
docker image prune -f

# ─── 7. Status ──────────────────────────────────────────────────────────────
echo ""
log "Deploy complete. Container status:"
echo ""
docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Image}}" | grep homelab || true
echo ""
