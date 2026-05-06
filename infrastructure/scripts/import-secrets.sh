#!/usr/bin/env bash
#
# One-time migration: import existing .env files into the secrets DB.
# Run from the homelab laptop after deploying the secrets feature.
#
# Usage: ./import-secrets.sh
#
set -euo pipefail

API_URL="${API_URL:-http://127.0.0.1:5170}"
API_SECRET="${INTERNAL_HOOK_SECRET:?Set INTERNAL_HOOK_SECRET}"
SERVICES_DIR="${SERVICES_DIR:-/opt/homelab/repo/services}"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log()  { echo -e "${GREEN}[import]${NC} $1"; }
warn() { echo -e "${YELLOW}[import]${NC} $1"; }

for service_dir in "${SERVICES_DIR}"/*/; do
    service_name=$(basename "${service_dir}")
    env_file="${service_dir}.env"

    if [[ ! -f "${env_file}" ]]; then
        warn "${service_name}: no .env file, skipping."
        continue
    fi

    log "${service_name}: registering service..."
    curl -sf -X POST "${API_URL}/api/v1/services" \
        -H "Authorization: Bearer ${API_SECRET}" \
        -H "Content-Type: application/json" \
        -d "{\"name\": \"${service_name}\"}" >/dev/null 2>&1 || log "${service_name}: already registered."

    # Build JSON object from .env file
    json="{"
    first=true
    while IFS= read -r line || [[ -n "${line}" ]]; do
        # Skip comments and empty lines
        [[ -z "${line}" || "${line}" =~ ^[[:space:]]*# ]] && continue

        key="${line%%=*}"
        value="${line#*=}"

        # Strip surrounding quotes from value
        value="${value#\"}"
        value="${value%\"}"
        value="${value#\'}"
        value="${value%\'}"

        # Escape double quotes and backslashes for JSON
        value="${value//\\/\\\\}"
        value="${value//\"/\\\"}"

        if [[ "${first}" == "true" ]]; then
            first=false
        else
            json+=","
        fi
        json+="\"${key}\": \"${value}\""
    done < "${env_file}"
    json+="}"

    if [[ "${json}" == "{}" ]]; then
        warn "${service_name}: .env is empty, skipping."
        continue
    fi

    log "${service_name}: importing secrets..."
    if curl -sf -X PUT "${API_URL}/api/v1/services/${service_name}/secrets" \
        -H "Authorization: Bearer ${API_SECRET}" \
        -H "Content-Type: application/json" \
        -d "${json}" >/dev/null; then
        log "${service_name}: done."
    else
        warn "${service_name}: import failed."
    fi
done

log "Import complete."
