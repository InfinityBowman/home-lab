# Self-Hosted Services

Standalone services running alongside the PaaS. These are not deployed via git-push — they have their own Docker Compose files or setup processes and plug into the existing Traefik routing via the `homelab` network.

## n8n (Workflow Automation)

**Location:** `services/n8n/`
**URL:** `https://n8n.<BASE_DOMAIN>`
**RAM usage:** ~1-2 GB (n8n + PostgreSQL)

### Setup

```bash
cd services/n8n
cp .env.example .env

# Generate encryption key — store this somewhere safe
openssl rand -base64 32
# Paste the output as N8N_ENCRYPTION_KEY in .env

# Set your postgres password and BASE_DOMAIN in .env
nano .env
```

### Start / Stop

```bash
# The main infrastructure stack must be running first (creates the homelab network)
cd /opt/homelab/infrastructure
docker compose up -d

# Then start n8n
cd /opt/homelab/services/n8n
docker compose up -d

# Check status
docker compose ps
docker compose logs -f n8n

# Stop
docker compose down
```

### First Login

Visit `https://n8n.<BASE_DOMAIN>` and create your owner account (email + password). There is no default admin — you set it up on first visit.

### Important Notes

- **Encryption key is critical.** If `N8N_ENCRYPTION_KEY` is lost or changed, ALL saved credentials (API keys, OAuth tokens, passwords stored in n8n) become permanently unrecoverable. Back it up.
- **Pin the version.** The compose file uses `1.76.1`. Before upgrading, check the [n8n releases](https://github.com/n8n-io/n8n/releases) for breaking changes.
- **Webhook URLs.** `WEBHOOK_URL` is set to `https://n8n.<BASE_DOMAIN>/` so webhook trigger nodes generate correct public URLs. If this is wrong, webhooks will show `localhost:5678` URLs.
- **PostgreSQL passwords** must not contain special characters (`@`, `#`, `!`, `$`) — the connection string parsing will break.

### Backups

```bash
# Database dump
docker exec homelab-n8n-postgres pg_dump -U n8n n8n > n8n-backup-$(date +%Y%m%d).sql

# Export workflows (optional)
docker exec homelab-n8n n8n export:workflow --all --output=/home/node/.n8n/backups/workflows.json
```

### Traefik Integration

n8n joins the `homelab` network as an external network. Traefik discovers it via Docker labels:

| Label | Value |
|-------|-------|
| `traefik.enable` | `true` |
| `traefik.http.routers.homelab-n8n.rule` | `` Host(`n8n.<BASE_DOMAIN>`) `` |
| `traefik.http.routers.homelab-n8n.entrypoints` | `web` |
| `traefik.http.services.homelab-n8n.loadbalancer.server.port` | `5678` |

Traffic flow: `Cloudflare Tunnel → Traefik → homelab-n8n:5678`

---

## NanoClaw (AI Agent — Discord)

**Setup method:** Claude Code `/setup` command (not Docker Compose)
**RAM usage:** ~500 MB - 1 GB
**Requires:** Node.js 20+, Docker (already installed), Anthropic API key, Discord Bot Token

NanoClaw is a lightweight AI agent platform that connects to Discord and routes messages to Claude. Agents run inside containers for isolation.

### Prerequisites

Install Node.js 20+ on the server:

```bash
# Install via NodeSource
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs

# Verify
node --version  # should be v20.x+
npm --version
```

### Setup

```bash
# Clone the repo
cd /opt/homelab/services
git clone https://github.com/qwibitai/NanoClaw.git nanoclaw
cd nanoclaw

# Install dependencies
npm install

# Run Claude Code and use /setup
claude
# Inside Claude Code, run:
# /setup
# Follow the prompts to configure:
#   - Anthropic API key
#   - Discord Bot Token
#   - Agent preferences
```

### Discord Bot Setup

1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Create a new application
3. Go to Bot → create a bot
4. Copy the Bot Token (this goes into NanoClaw config)
5. Enable these Privileged Gateway Intents:
   - Message Content Intent
   - Server Members Intent (optional)
6. Go to OAuth2 → URL Generator:
   - Scopes: `bot`
   - Bot Permissions: `Send Messages`, `Read Message History`, `Read Messages/View Channels`
7. Copy the generated URL and open it to invite the bot to your server

### Run as a systemd Service

To keep NanoClaw running after SSH disconnect and auto-start on boot:

```bash
sudo tee /etc/systemd/system/nanoclaw.service > /dev/null <<'EOF'
[Unit]
Description=NanoClaw AI Agent
After=network.target docker.service

[Service]
Type=simple
User=jacob
WorkingDirectory=/opt/homelab/services/nanoclaw
ExecStart=/usr/bin/node index.js
Restart=on-failure
RestartSec=10
Environment=NODE_ENV=production

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable nanoclaw
sudo systemctl start nanoclaw

# Check status
sudo systemctl status nanoclaw
journalctl -u nanoclaw -f
```

### Important Notes

- **API costs.** NanoClaw calls the Anthropic API for every message. Monitor your usage at [console.anthropic.com](https://console.anthropic.com).
- **Container isolation.** Agents execute bash commands inside Docker containers, not on the host. This is a security feature — don't disable it.
- **Updates.** Pull the latest code with `cd /opt/homelab/services/nanoclaw && git pull && npm install`, then restart the service.

---

## Resource Budget

All services running on the HP Spectre x360 (16 GB RAM):

| Service | Est. RAM | Status |
|---------|----------|--------|
| Ubuntu OS + overhead | ~1 GB | Always |
| PaaS (Traefik + API + apps) | ~1-2 GB | Core infrastructure |
| n8n + PostgreSQL | ~1-2 GB | `services/n8n/` |
| NanoClaw | ~0.5-1 GB | systemd service |
| **Headroom** | **~10-12 GB free** | Available for deployed apps |

This leaves plenty of room. If you add Twenty CRM later (~4-6 GB), headroom drops to ~5-7 GB.
