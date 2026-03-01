# HomeLab PaaS

Mini-PaaS built in Rust running on an HP laptop (Ubuntu Server 24.04) at `192.168.1.100`. SSH shortcut: `ssh homelab`.

## How It Works

Push to `main` → GitHub Actions CI (fmt, clippy, test, docker build) → `deploy` job on self-hosted runner at `/opt/homelab/repo` → runs `infrastructure/scripts/deploy.sh` which rebuilds infrastructure and any changed services.

## Networking

All traffic flows: **Internet → Cloudflare Edge → cloudflared tunnel → Traefik → container**

- Wildcard DNS: `*.jacobmaynard.dev` → tunnel CNAME
- Wildcard tunnel ingress: `*.jacobmaynard.dev` → `http://homelab-traefik:80`
- Traefik routes by `Host` header via Docker label auto-discovery
- No per-service DNS or tunnel config needed — just add Traefik labels and it works

## Project Layout

```
infrastructure/           Core stack (Traefik, cloudflared, PaaS API)
  docker-compose.yml      Started on every deploy
  Dockerfile              Multi-stage Rust build for homelab-api
  traefik/dynamic.yml     Middlewares (security headers, rate limit, scanner blocker)
  scripts/
    setup-host.sh          Run once on fresh Ubuntu install
    setup-runner.sh        Register GitHub Actions self-hosted runner
    deploy.sh              Called by CI — pulls code, rebuilds, restarts services

services/                 Self-hosted apps (each independent docker-compose)
  n8n/                    Workflow automation
  plausible/              Privacy-first analytics (+ ClickHouse + Postgres)

crates/                   Rust workspace (7 crates)
  homelab-core/           Shared types (App, Deployment, EnvVar, AppStatus)
  homelab-db/             SQLite via sqlx
  homelab-docker/         Docker API via bollard
  homelab-cloudflare/     Tunnel + DNS management via reqwest
  homelab-git/            Bare repos + post-receive hooks
  homelab-api/            axum HTTP server (main binary, port 5170)
  homelab-dashboard/      HTMX + Askama templates

migrations/               SQLite migrations (embedded at compile time by sqlx)
docs/                     Detailed reference docs
```

## Adding a New Service

1. Create `services/<name>/docker-compose.yml` — follow the n8n/plausible pattern:
   - Join `homelab` external network for Traefik discovery
   - Use a `<name>-internal` network for backing databases
   - Add Traefik labels: `traefik.http.routers.homelab-<name>.rule=Host(\`<name>.${BASE_DOMAIN}\`)`
   - Use `traefik.http.routers.homelab-<name>.middlewares=secure-chain@file`
   - Health checks with `depends_on: condition: service_healthy`
2. Create `.env.example` (committed) and `.env` (gitignored, has secrets)
3. SSH into the laptop (`ssh homelab`) and create the `.env` at `/opt/homelab/repo/services/<name>/.env` — this file won't be deployed by git push since it's gitignored
4. Push to `main` — deploy.sh auto-detects changed service dirs and runs `docker compose up -d`

## Conventions

- Container names: `homelab-<name>` (e.g. `homelab-n8n`, `homelab-plausible`)
- Router names in Traefik labels: `homelab-<name>`
- Load balancer port label matches the app's internal port
- All services use `restart: unless-stopped`
- Backing databases (postgres, clickhouse) are NOT on the homelab network — only internal

## Environment Variables

- `.env` files are gitignored — secrets live only on the laptop
- `infrastructure/.env` has Cloudflare credentials, base domain, hook secret
- Each service has its own `.env` in `services/<name>/`
- `BASE_DOMAIN=jacobmaynard.dev`

## Deploy Script Behavior (`deploy.sh`)

1. `git fetch origin main && git reset --hard origin/main`
2. `docker compose build && docker compose up -d` in `infrastructure/`
3. For each `services/*/`: if `git diff` shows changes since last deploy, runs `docker compose up -d`
4. `docker image prune -f`

## Tech Stack

- **Language:** Rust (axum, bollard, sqlx, reqwest)
- **Database:** SQLite (single file at `/data/homelab.db`)
- **Proxy:** Traefik (Docker label auto-discovery, `secure-chain` middleware)
- **Tunnel:** Cloudflare Tunnel (remote-managed, wildcard ingress)
- **Dashboard:** HTMX + Askama + Tailwind/Pico CSS
- **CI:** GitHub Actions → self-hosted runner on the laptop
