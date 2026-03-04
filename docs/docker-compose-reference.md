# Docker Compose Reference

Full breakdown of the infrastructure Docker Compose configuration.

## Services

### Traefik (Reverse Proxy)

```yaml
traefik:
  image: traefik:v3.6.9
  container_name: homelab-traefik
  restart: unless-stopped
  command:
    - "--api.dashboard=true"
    - "--api.insecure=true"
    - "--providers.docker=true"
    - "--providers.docker.exposedbydefault=false"
    - "--providers.docker.network=homelab"
    - "--entrypoints.web.address=:80"
    - "--entrypoints.websecure.address=:443"
    - "--entrypoints.web.http.redirections.entryPoint.to=websecure"
  ports:
    - "80:80"
    - "443:443"
    - "127.0.0.1:8080:8080"  # Traefik dashboard (localhost only)
  volumes:
    - /var/run/docker.sock:/var/run/docker.sock:ro
  networks:
    - homelab
```

Key config explained:
- `providers.docker=true` — Traefik watches Docker for new containers
- `providers.docker.exposedbydefault=false` — only expose containers with `traefik.enable=true` label
- `providers.docker.network=homelab` — route to containers on the `homelab` network
- Docker socket mounted read-only for auto-discovery

### cloudflared (Tunnel Connector)

```yaml
cloudflared:
  image: cloudflare/cloudflared:latest
  container_name: homelab-cloudflared
  restart: unless-stopped
  command: tunnel run
  environment:
    - TUNNEL_TOKEN=${CLOUDFLARE_TUNNEL_TOKEN}
  networks:
    - homelab
```

Runs in "connector" mode with just a token. No local config file needed. Ingress rules are managed via the Cloudflare API by the PaaS API.

### PaaS API (The Rust Server)

```yaml
paas-api:
  build:
    context: ..
    dockerfile: infrastructure/Dockerfile
  container_name: homelab-paas-api
  restart: unless-stopped
  environment:
    - DATABASE_URL=sqlite:///data/homelab.db
    - DOCKER_HOST=unix:///var/run/docker.sock
    - GIT_REPOS_PATH=/git-repos
    - CLOUDFLARE_API_TOKEN=${CLOUDFLARE_API_TOKEN}
    - CLOUDFLARE_ACCOUNT_ID=${CLOUDFLARE_ACCOUNT_ID}
    - CLOUDFLARE_TUNNEL_ID=${CLOUDFLARE_TUNNEL_ID}
    - CLOUDFLARE_ZONE_ID=${CLOUDFLARE_ZONE_ID}
    - BASE_DOMAIN=${BASE_DOMAIN}
    - INTERNAL_HOOK_SECRET=${INTERNAL_HOOK_SECRET}
    - RUST_LOG=info,homelab=debug
  volumes:
    - /var/run/docker.sock:/var/run/docker.sock
    - ../git-repos:/git-repos
    - ../data:/data
  ports:
    - "127.0.0.1:5170:5170"  # Localhost only — access via Traefik
  networks:
    - homelab
  labels:
    - "traefik.enable=true"
    - "traefik.http.routers.paas-api.rule=Host(`paas.${BASE_DOMAIN}`)"
    - "traefik.http.routers.paas-api.entrypoints=web"
    - "traefik.http.services.paas-api.loadbalancer.server.port=5170"
  depends_on:
    - traefik
    - cloudflared
```

Key details:
- Docker socket is mounted **read-write** (the API creates/manages containers)
- `git-repos` volume is shared between the host (SSH push target) and the container
- `data` volume holds the SQLite database
- Traefik labels expose the dashboard at `paas.lab.yourdomain.com`

## Network

```yaml
networks:
  homelab:
    name: homelab
    driver: bridge
```

All services and deployed app containers join this network. This is how:
- Traefik can reach app containers
- cloudflared can reach Traefik
- App containers can reach each other (by container name)

## Volumes Summary

| Mount | Host Path | Container Path | Used By | Purpose |
|-------|-----------|----------------|---------|---------|
| Docker socket | `/var/run/docker.sock` | `/var/run/docker.sock` | Traefik (ro), PaaS API (rw) | Container discovery/management |
| Git repos | `../git-repos` | `/git-repos` | PaaS API | Bare git repos for app source |
| Data | `../data` | `/data` | PaaS API | SQLite database file |

## Dockerfile (Multi-Stage Rust Build)

```dockerfile
# Stage 1: Build the Rust binary
FROM rust:1.91-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY migrations/ migrations/
RUN cargo build --release --bin homelab-api

# Stage 2: Minimal runtime image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates git curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/homelab-api /usr/local/bin/homelab-api
COPY crates/homelab-dashboard/static/ /app/static/
COPY migrations/ /app/migrations/
ENV DATABASE_URL=sqlite:///data/homelab.db
EXPOSE 5170
CMD ["homelab-api"]
```

The runtime image needs:
- `ca-certificates` — for HTTPS calls to Cloudflare API
- `git` — for checking out code from bare repos during builds
- `curl` — useful for debugging

## Environment File

Create `.env` in the `infrastructure/` directory:

```bash
# Cloudflare
CLOUDFLARE_API_TOKEN=
CLOUDFLARE_ACCOUNT_ID=
CLOUDFLARE_TUNNEL_ID=
CLOUDFLARE_TUNNEL_TOKEN=
CLOUDFLARE_ZONE_ID=
BASE_DOMAIN=lab.yourdomain.com

# Security
INTERNAL_HOOK_SECRET=      # generate with: openssl rand -hex 32
```

Docker Compose automatically loads `.env` from the same directory as `docker-compose.yml`.

## Commands

```bash
# Start everything
docker compose up -d

# View logs
docker compose logs -f
docker compose logs -f paas-api

# Rebuild after code changes
docker compose up -d --build paas-api

# Stop everything
docker compose down

# Stop and remove volumes (DESTRUCTIVE — deletes data)
docker compose down -v

# Check status
docker compose ps
```
