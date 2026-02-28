# HomeLab PaaS — Architecture

## System Overview

A self-hosted mini-PaaS built in Rust. Push code via git, it builds a Docker image, deploys it as a container, and makes it accessible via Cloudflare Tunnel.

## Traffic Flow

```
Internet
  │
  ▼
Cloudflare Edge (DDoS protection, SSL termination)
  │
  ▼
cloudflared (tunnel connector, runs as Docker container)
  │  forwards all traffic to Traefik
  ▼
Traefik (reverse proxy, Docker container)
  │  routes by Host header using Docker label auto-discovery
  ▼
App Container (e.g. homelab-my-app)
```

All Cloudflare Tunnel ingress rules point to `http://homelab-traefik:80`. Traefik then routes based on the `Host` header, which cloudflared preserves. This keeps routing in a single layer.

## Component Architecture

```
┌─────────────────────────────────────────────────────┐
│  HP Laptop (Ubuntu Server 24.04 + Docker)           │
│                                                     │
│  ┌────────────┐  ┌───────────┐  ┌──────────────┐   │
│  │  Traefik   │  │  PaaS API │  │  cloudflared │   │
│  │  (proxy)   │  │  (Rust)   │  │  (tunnel)    │   │
│  └──────┬─────┘  └─────┬─────┘  └──────────────┘   │
│         │              │                            │
│         │     ┌────────┼────────┐                   │
│         │     │        │        │                   │
│         │   SQLite   Docker   Cloudflare            │
│         │            Engine    API                   │
│         │              │                            │
│  ┌──────┴──────────────┴───────────────────────┐    │
│  │              Docker Network: homelab         │    │
│  │  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐           │    │
│  │  │App 1│ │App 2│ │App 3│ │ DB  │  ...       │    │
│  │  └─────┘ └─────┘ └─────┘ └─────┘           │    │
│  └─────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────┘
```

## Rust Crate Workspace

```
crates/
├── homelab-core/         Shared types: App, Deployment, EnvVar, AppStatus, etc.
│                         Zero deps beyond serde/chrono/uuid. The "vocabulary" crate.
│
├── homelab-db/           SQLite via sqlx. One repo module per table:
│                         AppRepo, DeploymentRepo, EnvVarRepo, AuditRepo
│
├── homelab-docker/       Wraps bollard (Docker API). Modules:
│                         client.rs    — Docker connection
│                         builder.rs   — tar archive → build_image → tag
│                         containers.rs — create/start/stop/restart/remove
│                         logs.rs      — log streaming (SSE-compatible)
│                         labels.rs    — Traefik label generation
│                         network.rs   — Docker network management
│
├── homelab-cloudflare/   Cloudflare API via reqwest. Modules:
│                         tunnel.rs    — Tunnel config CRUD
│                         dns.rs       — CNAME record management
│                         ingress.rs   — Ingress rule assembly + PUT
│
├── homelab-git/          Git operations. Modules:
│                         bare_repo.rs — Create/manage bare git repos
│                         hooks.rs     — Generate post-receive hook scripts
│                         webhook.rs   — HTTP handler for hook callbacks
│
├── homelab-api/          axum HTTP server. The main binary.
│                         main.rs      — entrypoint, wires everything together
│                         state.rs     — AppState (DB pool, Docker client, config)
│                         router.rs    — route assembly
│                         handlers/    — one file per resource
│                         middleware/  — auth, logging
│
└── homelab-dashboard/    HTMX + Askama templates. Served by same axum process.
                          templates/   — Jinja2-style HTML templates
                          static/      — htmx.min.js, CSS
```

## Key Design Decisions

| Decision | Choice | Why |
|----------|--------|-----|
| Cargo workspace (7 crates) | vs mono-crate | Clear boundaries, faster incremental builds |
| Git bare repo + post-receive hook | vs GitHub webhooks | No external dependency, works offline |
| Remote-managed CF Tunnel | vs local config file | API-driven, no config sync, cloudflared needs only a token |
| All CF ingress → Traefik | vs direct to containers | Single routing layer, Traefik handles Host matching |
| HTMX + Askama | vs React/Leptos | No JS build chain, compile-time template checking, all Rust |
| SQLite | vs Postgres | Single file, zero ops, perfect for home lab scale |
| Container naming: `homelab-<app>` | convention | Avoids conflicts, easy to identify PaaS-managed containers |
| Image tagging: `homelab/<app>:<sha>` | convention | Enables rollback to any previous build |

## AppState (Central Wiring)

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub docker: Docker,           // bollard client
    pub cloudflare: IngressManager,
    pub config: AppConfig,
}

#[derive(Clone)]
pub struct AppConfig {
    pub git_repos_path: String,
    pub base_domain: String,
    pub internal_hook_secret: String,
}
```

The axum server builds this state once at startup and passes it to all handlers via `State<AppState>`.

## Domain Model

```rust
pub struct App {
    pub id: String,           // UUID
    pub name: String,         // URL-safe slug: "my-app"
    pub domain: String,       // "my-app.lab.example.com"
    pub git_repo_path: String,
    pub docker_image: String,
    pub port: u16,            // Internal container port
    pub status: AppStatus,    // Created | Building | Running | Stopped | Failed
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct Deployment {
    pub id: String,
    pub app_id: String,
    pub commit_sha: String,
    pub image_tag: String,
    pub status: DeployStatus, // Pending | Building | Deploying | Succeeded | Failed
    pub build_log: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

pub struct EnvVar {
    pub id: String,
    pub app_id: String,
    pub key: String,
    pub value: String,
    pub created_at: DateTime<Utc>,
}
```

## Error Handling Strategy

- `thiserror` in library crates (homelab-core, homelab-db, homelab-docker, etc.) for typed errors
- `anyhow` in the binary crate (homelab-api) for convenience at the top level
- All API responses use a uniform envelope: `{ "success": bool, "data": T, "error": string? }`
