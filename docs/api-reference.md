# PaaS API Reference

Base URL: `http://localhost:3000` (or `https://paas.lab.yourdomain.com` via tunnel)

All responses use the envelope:
```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

Authentication: `Authorization: Bearer <API_KEY>` (Phase 6)

---

## Apps

### List Apps

```
GET /api/v1/apps
```

Response:
```json
{
  "success": true,
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "my-app",
      "domain": "my-app.lab.example.com",
      "port": 3000,
      "status": "running",
      "created_at": "2026-02-26T10:00:00Z",
      "updated_at": "2026-02-26T12:30:00Z"
    }
  ]
}
```

### Create App

```
POST /api/v1/apps
Content-Type: application/json

{
  "name": "my-app",
  "domain": "my-app.lab.example.com",
  "port": 3000,
  "env_vars": {
    "NODE_ENV": "production"
  }
}
```

This creates:
- A record in SQLite
- A bare git repo at `/git-repos/my-app.git`
- A `post-receive` hook in the repo

Response includes the git remote URL:
```json
{
  "success": true,
  "data": {
    "id": "...",
    "name": "my-app",
    "domain": "my-app.lab.example.com",
    "port": 3000,
    "status": "created",
    "git_remote": "ssh://paas@homelab/git-repos/my-app.git"
  }
}
```

### Get App

```
GET /api/v1/apps/:name
```

### Update App

```
PUT /api/v1/apps/:name
Content-Type: application/json

{
  "domain": "new-domain.lab.example.com",
  "port": 8080
}
```

### Delete App

```
DELETE /api/v1/apps/:name
```

Stops the container, removes the Docker image, deletes the git repo, removes Cloudflare DNS record.

---

## Container Lifecycle

### Start

```
POST /api/v1/apps/:name/start
```

### Stop

```
POST /api/v1/apps/:name/stop
```

### Restart

```
POST /api/v1/apps/:name/restart
```

### Get Status

```
GET /api/v1/apps/:name/status
```

Response:
```json
{
  "success": true,
  "data": {
    "container_id": "abc123...",
    "state": "running",
    "uptime": "2h 30m",
    "cpu_percent": 2.5,
    "memory_mb": 128,
    "memory_limit_mb": 512
  }
}
```

### Stream Logs (SSE)

```
GET /api/v1/apps/:name/logs
Accept: text/event-stream
```

Returns a Server-Sent Events stream of container stdout/stderr.

### Get Recent Logs

```
GET /api/v1/apps/:name/logs?tail=100
```

Returns the last N lines as JSON.

---

## Deployments

### Trigger Deploy

```
POST /api/v1/apps/:name/deploy
```

Deploys from HEAD of the main branch in the git repo.

### List Deployments

```
GET /api/v1/apps/:name/deployments
```

Response:
```json
{
  "success": true,
  "data": [
    {
      "id": "...",
      "commit_sha": "abc1234567890",
      "image_tag": "homelab/my-app:abc12345",
      "status": "succeeded",
      "started_at": "2026-02-26T12:00:00Z",
      "finished_at": "2026-02-26T12:02:30Z"
    }
  ]
}
```

### Get Deployment Detail

```
GET /api/v1/apps/:name/deployments/:id
```

Includes the full `build_log`.

### Rollback

```
POST /api/v1/apps/:name/deployments/:id/rollback
```

Re-deploys the Docker image from a previous deployment (no rebuild needed).

---

## Environment Variables

### List Env Vars

```
GET /api/v1/apps/:name/env
```

Values are masked in the response:
```json
{
  "success": true,
  "data": [
    { "key": "NODE_ENV", "value": "produ*****" },
    { "key": "DATABASE_URL", "value": "sqlit*****" }
  ]
}
```

### Bulk Set Env Vars

```
PUT /api/v1/apps/:name/env
Content-Type: application/json

{
  "NODE_ENV": "production",
  "DATABASE_URL": "sqlite:///data/app.db",
  "SECRET_KEY": "supersecret"
}
```

This replaces all env vars and triggers a container restart.

### Delete Env Var

```
DELETE /api/v1/apps/:name/env/:key
```

---

## Git Webhook (Internal)

```
POST /hooks/git/:app_name
Authorization: Bearer <INTERNAL_HOOK_SECRET>
Content-Type: application/json

{
  "ref": "refs/heads/main",
  "commit_sha": "abc1234567890...",
  "repo_path": "/git-repos/my-app.git"
}
```

Called by the `post-receive` hook. Only deploys pushes to `refs/heads/main`.

---

## System

### Health Check

```
GET /api/v1/system/health
```

Response:
```json
{
  "success": true,
  "data": {
    "status": "healthy",
    "uptime": "5d 3h 20m"
  }
}
```

### System Info

```
GET /api/v1/system/info
```

Response:
```json
{
  "success": true,
  "data": {
    "docker_version": "27.5.0",
    "total_apps": 5,
    "running_apps": 3,
    "disk_total_gb": 256,
    "disk_used_gb": 45,
    "disk_percent": 17.6,
    "memory_total_mb": 8192,
    "memory_used_mb": 3200
  }
}
```
