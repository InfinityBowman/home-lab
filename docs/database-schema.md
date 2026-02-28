# Database Schema

SQLite database at `/data/homelab.db`, managed via sqlx migrations.

## Migration 001: Initial Schema

```sql
CREATE TABLE apps (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    domain TEXT NOT NULL UNIQUE,
    git_repo_path TEXT NOT NULL,
    docker_image TEXT NOT NULL DEFAULT '',
    port INTEGER NOT NULL DEFAULT 3000,
    status TEXT NOT NULL DEFAULT 'created',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE deployments (
    id TEXT PRIMARY KEY,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    commit_sha TEXT NOT NULL,
    image_tag TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending',
    build_log TEXT DEFAULT '',
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at TEXT
);

CREATE TABLE env_vars (
    id TEXT PRIMARY KEY,
    app_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (app_id) REFERENCES apps(id) ON DELETE CASCADE,
    UNIQUE(app_id, key)
);

CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    app_id TEXT,
    action TEXT NOT NULL,
    details TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_deployments_app_id ON deployments(app_id);
CREATE INDEX idx_env_vars_app_id ON env_vars(app_id);
CREATE INDEX idx_audit_log_app_id ON audit_log(app_id);
CREATE INDEX idx_audit_log_created_at ON audit_log(created_at);
```

## Migration 002: Cloudflare State

```sql
CREATE TABLE cloudflare_dns_records (
    id TEXT PRIMARY KEY,
    app_id TEXT NOT NULL UNIQUE,
    cf_record_id TEXT NOT NULL,
    hostname TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (app_id) REFERENCES apps(id) ON DELETE CASCADE
);
```

## Table Descriptions

### apps
The central table. Each row is a deployed application.

| Column | Type | Description |
|--------|------|-------------|
| id | TEXT (UUID) | Primary key |
| name | TEXT | URL-safe slug, e.g. "my-app". Must be unique. |
| domain | TEXT | Full domain, e.g. "my-app.lab.example.com" |
| git_repo_path | TEXT | Path to bare git repo, e.g. "/git-repos/my-app.git" |
| docker_image | TEXT | Current image tag, e.g. "homelab/my-app:abc12345" |
| port | INTEGER | Internal container port the app listens on |
| status | TEXT | One of: created, building, running, stopped, failed |
| created_at | TEXT | ISO 8601 timestamp |
| updated_at | TEXT | ISO 8601 timestamp |

### deployments
History of every deploy attempt for an app.

| Column | Type | Description |
|--------|------|-------------|
| id | TEXT (UUID) | Primary key |
| app_id | TEXT | FK → apps.id |
| commit_sha | TEXT | Full git commit SHA |
| image_tag | TEXT | Docker image tag, e.g. "homelab/my-app:abc12345" |
| status | TEXT | One of: pending, building, deploying, succeeded, failed |
| build_log | TEXT | Full Docker build output |
| started_at | TEXT | When the deploy started |
| finished_at | TEXT | When it completed (null if in progress) |

### env_vars
Environment variables injected into containers at runtime.

| Column | Type | Description |
|--------|------|-------------|
| id | TEXT (UUID) | Primary key |
| app_id | TEXT | FK → apps.id |
| key | TEXT | Env var name, e.g. "DATABASE_URL" |
| value | TEXT | Env var value (plaintext; encryption is a future goal) |
| created_at | TEXT | ISO 8601 timestamp |

Unique constraint on (app_id, key) — one value per key per app.

### audit_log
Append-only log of significant actions for debugging/auditing.

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER | Auto-increment primary key |
| app_id | TEXT | FK → apps.id (nullable for system-level events) |
| action | TEXT | e.g. "app.created", "deploy.started", "container.stopped" |
| details | TEXT | JSON string with extra context |
| created_at | TEXT | ISO 8601 timestamp |

### cloudflare_dns_records
Tracks CNAME records created in Cloudflare so we can delete them on app removal.

| Column | Type | Description |
|--------|------|-------------|
| id | TEXT (UUID) | Primary key |
| app_id | TEXT | FK → apps.id (unique — one record per app) |
| cf_record_id | TEXT | Cloudflare's ID for the DNS record |
| hostname | TEXT | e.g. "my-app.lab.example.com" |
| created_at | TEXT | ISO 8601 timestamp |
