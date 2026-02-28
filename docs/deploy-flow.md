# Deploy Flow — Detailed Walkthrough

How code goes from `git push` to a running, publicly-accessible container.

## Overview

```
Developer                    Server
   │                           │
   │  git push deploy main     │
   │ ─────────────────────────>│
   │                           │  post-receive hook fires
   │                           │  curl POST /hooks/git/my-app
   │                           │
   │                           │  ┌─ PaaS API ──────────────┐
   │                           │  │ 1. Record deployment     │
   │                           │  │ 2. Checkout code         │
   │                           │  │ 3. Build Docker image    │
   │                           │  │ 4. Stop old container    │
   │                           │  │ 5. Start new container   │
   │                           │  │ 6. Update CF tunnel      │
   │                           │  │ 7. Update DNS            │
   │                           │  │ 8. Finalize              │
   │                           │  └──────────────────────────┘
   │                           │
   │  push complete            │
   │ <─────────────────────────│
```

## Step-by-Step

### Setup: Creating an App

When you `POST /api/v1/apps` with `{ "name": "my-app" }`:

1. **SQLite:** Insert app record with status `created`
2. **Git:** Create bare repo at `/git-repos/my-app.git`:
   ```bash
   git init --bare /git-repos/my-app.git
   ```
3. **Hook:** Write `/git-repos/my-app.git/hooks/post-receive`:
   ```bash
   #!/bin/bash
   while read oldrev newrev ref; do
     if [ "$ref" = "refs/heads/main" ]; then
       curl -s -X POST \
         -H "Content-Type: application/json" \
         -H "Authorization: Bearer ${INTERNAL_HOOK_SECRET}" \
         -d "{\"ref\":\"$ref\",\"commit_sha\":\"$newrev\",\"repo_path\":\"$(pwd)\"}" \
         http://localhost:3000/hooks/git/my-app
     fi
   done
   ```
4. **Hook permissions:** `chmod +x` the hook

The developer then adds the remote:
```bash
git remote add deploy ssh://paas@192.168.1.100/git-repos/my-app.git
```

### Deploy: Pushing Code

When the developer runs `git push deploy main`:

#### Step 1: Record Deployment
- Insert `Deployment` row: status = `pending`
- Update `App.status` to `building`

#### Step 2: Checkout Code
- Create temp directory: `/tmp/build/my-app-{sha}`
- Checkout from bare repo:
  ```bash
  git --work-tree=/tmp/build/my-app-abc123 \
      --git-dir=/git-repos/my-app.git \
      checkout -f abc123
  ```
- Verify a `Dockerfile` exists in the checkout

#### Step 3: Build Docker Image
- Create a tar archive of the checkout directory (Docker build API expects tar)
- Call `bollard::Docker::build_image()`:
  - Dockerfile: `Dockerfile` (from the app's repo)
  - Tag: `homelab/my-app:abc12345` (first 8 chars of commit SHA)
  - Also tag as `homelab/my-app:latest`
- Stream build output into `Deployment.build_log`
- If build fails → update deployment to `failed`, update app to `failed`, stop

#### Step 4: Stop Old Container
- If container `homelab-my-app` exists:
  ```
  docker.stop_container("homelab-my-app", timeout=10)
  docker.remove_container("homelab-my-app")
  ```
- If no existing container, skip this step

#### Step 5: Start New Container
- Load env vars from SQLite for this app
- Generate Traefik Docker labels:
  ```
  traefik.enable=true
  traefik.http.routers.my-app.rule=Host(`my-app.lab.example.com`)
  traefik.http.routers.my-app.entrypoints=websecure
  traefik.http.services.my-app.loadbalancer.server.port=3000
  ```
- Create container:
  - Image: `homelab/my-app:abc12345`
  - Name: `homelab-my-app`
  - Network: `homelab`
  - Env vars from DB
  - Labels for Traefik
  - Restart policy: `unless-stopped`
- Start container

#### Step 6: Update Cloudflare Tunnel Ingress
- Fetch all apps with status `running` from SQLite
- Build ingress rules array (all pointing to `http://homelab-traefik:80`)
- PUT to Cloudflare Tunnel Configuration API
- cloudflared picks up the new config automatically

#### Step 7: Ensure DNS Record Exists
- Check if CNAME `my-app.lab.example.com` exists
- If not, create it pointing to `{TUNNEL_ID}.cfargotunnel.com`
- (Skip if using wildcard CNAME)

#### Step 8: Finalize
- Update `Deployment.status` to `succeeded`
- Set `Deployment.finished_at`
- Update `App.status` to `running`
- Clean up temp build directory

### Rollback

When you `POST /api/v1/apps/my-app/deployments/{old-id}/rollback`:

1. Look up the old deployment's `image_tag` (e.g., `homelab/my-app:def45678`)
2. The image should still be in local Docker cache
3. Run Steps 4-8 above using that image tag
4. **No rebuild needed** — that's the key advantage of tagging by commit SHA

### Build Requirements for Apps

Each app needs a `Dockerfile` in its root. Example for a Node.js app:

```dockerfile
FROM node:20-slim
WORKDIR /app
COPY package*.json ./
RUN npm ci --production
COPY . .
EXPOSE 3000
CMD ["node", "server.js"]
```

Example for a Go app:
```dockerfile
FROM golang:1.23 AS builder
WORKDIR /app
COPY go.* ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 go build -o server .

FROM alpine:3.19
COPY --from=builder /app/server /server
EXPOSE 8080
CMD ["/server"]
```

The PaaS doesn't care what language — if it has a Dockerfile, it can be deployed.
