# Services Setup Checklist

Step-by-step guide to getting n8n and NanoClaw running on your homelab. Each step is marked with who does it.

- **You** = requires your browser, accounts, or decisions
- **Claude** = I can do this over SSH

---

## Part 1: n8n

### Step 1 — Copy files to server

**Claude** can do this.

- Copy `services/n8n/` to `/opt/homelab/services/n8n/` on the server
- Create the `.env` file from the template

### Step 2 — Generate secrets

**Claude** can do this.

- Generate `N8N_ENCRYPTION_KEY` with `openssl rand -base64 32`
- Generate `N8N_POSTGRES_PASSWORD` with `openssl rand -base64 16`
- Write both to `/opt/homelab/services/n8n/.env`

> **Important:** Save the encryption key somewhere safe (password manager). If it's ever lost, all credentials stored in n8n become permanently unrecoverable.

### Step 3 — Set your BASE_DOMAIN

**You** need to decide this.

- Your domain is `lab.jacobmaynard.dev`
- n8n will be accessible at `n8n.lab.jacobmaynard.dev`
- This must match the `BASE_DOMAIN` in your main infrastructure `.env`

### Step 4 — Cloudflare DNS record

**You** need to do this (or Claude can via API if your Cloudflare token is set up).

- Add a CNAME record: `n8n.lab.yourdomain.com` → `<TUNNEL_ID>.cfargotunnel.com`
- Or if you already have a wildcard `*.lab.yourdomain.com` CNAME, this is already handled

### Step 5 — Cloudflare Tunnel ingress rule

**You** need to do this in Cloudflare dashboard (or Claude can via API).

- Add a public hostname in your tunnel config:
  - Subdomain: `n8n`
  - Domain: `lab.yourdomain.com`
  - Service: `http://homelab-traefik:80`

### Step 6 — Start n8n

**Claude** can do this.

- Make sure the main infrastructure stack is running (creates the `homelab` network)
- Run `docker compose up -d` in the n8n directory
- Verify containers are healthy

### Step 7 — Create your n8n account

**You** must do this.

- Open `https://n8n.<BASE_DOMAIN>` in your browser
- Create your owner account (email + password)
- This is the admin account — there is no default login

### n8n is done after step 7.

---

## Part 2: NanoClaw

### Step 8 — Install Node.js on the server

**Claude** can do this.

- Install Node.js 20 via NodeSource apt repo
- Verify `node --version` and `npm --version`

### Step 9 — Create a Discord Bot

**You** must do this — requires your Discord account.

1. Go to https://discord.com/developers/applications
2. Click **New Application**, give it a name (e.g., "NanoClaw")
3. Go to **Bot** in the left sidebar
4. Click **Reset Token** and **copy the Bot Token** — you'll need this later
5. Scroll down to **Privileged Gateway Intents** and enable:
   - **Message Content Intent** (required)
   - **Server Members Intent** (optional)
6. Go to **OAuth2 → URL Generator** in the left sidebar
7. Under **Scopes**, check `bot`
8. Under **Bot Permissions**, check:
   - Send Messages
   - Read Message History
   - Read Messages/View Channels
9. Copy the generated URL at the bottom
10. Open that URL in your browser and invite the bot to your Discord server

**Save the Bot Token** — you'll give it to Claude for the NanoClaw setup.

### Step 10 — Get an Anthropic API Key

**You** must do this — requires your Anthropic account.

1. Go to https://console.anthropic.com/settings/keys
2. Create a new API key
3. Copy it — you'll give it to Claude for the NanoClaw setup

> **Note:** NanoClaw calls the Anthropic API for every message your Discord bot receives. Monitor usage at https://console.anthropic.com/settings/billing. Set a usage limit to avoid surprises.

### Step 11 — Clone and set up NanoClaw

**Claude** can do this (once you provide the tokens from steps 9-10).

- Clone the NanoClaw repo to `/opt/homelab/services/nanoclaw/`
- Install npm dependencies
- Configure with your Discord Bot Token and Anthropic API Key

### Step 12 — Run NanoClaw setup

**This is a grey area.** NanoClaw is designed to be configured via Claude Code's `/setup` command interactively. Two options:

- **Option A:** You SSH into the server, run `claude` in the nanoclaw directory, and run `/setup` yourself
- **Option B:** Claude sets it up manually by writing the config files directly (less tested but doable)

### Step 13 — Set up auto-start

**Claude** can do this.

- Create a systemd service so NanoClaw starts on boot
- Enable and start the service

### NanoClaw is done after step 13.

---

## Summary: What You Need to Provide

Before we start, gather these:

| Item | Where to get it | For |
|------|----------------|-----|
| `BASE_DOMAIN` | Your decision | n8n routing |
| Cloudflare DNS/Tunnel config | Cloudflare dashboard | n8n public access |
| Discord Bot Token | Discord Developer Portal (step 9) | NanoClaw |
| Anthropic API Key | console.anthropic.com (step 10) | NanoClaw |

Everything else I can handle over SSH.

---

## What Claude Will Do (Once You're Ready)

Once you provide the items above, I can run through all the "Claude" steps in one go:

1. Copy n8n compose files to the server
2. Generate and write secrets
3. Start n8n containers
4. Install Node.js
5. Clone and configure NanoClaw
6. Set up systemd service

Total time: ~10-15 minutes.
