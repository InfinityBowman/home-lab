# Cloudflare Tunnel Setup

How to create and configure a Cloudflare Tunnel for the PaaS.

## Prerequisites

- A domain on Cloudflare (free plan is fine)
- Cloudflare account

## 1. Create an API Token

1. Go to https://dash.cloudflare.com/profile/api-tokens
2. Click "Create Token"
3. Use "Custom token" template
4. Permissions needed:
   - **Account** → Cloudflare Tunnel → Edit
   - **Zone** → DNS → Edit
   - **Zone** → Zone → Read
5. Zone Resources: Include → your domain
6. Create the token and save it — you'll need it for `.env`

## 2. Get Your Account ID and Zone ID

- **Account ID:** Visible on the main Cloudflare dashboard sidebar, or at `dash.cloudflare.com` → select your domain → right sidebar under "API"
- **Zone ID:** Same page, right sidebar under "API"

## 3. Create a Tunnel

### Option A: Via Cloudflare Dashboard (Easier)

1. Go to https://one.dash.cloudflare.com/
2. Networks → Tunnels → Create a tunnel
3. Choose "Cloudflared" connector
4. Name it `homelab`
5. Copy the tunnel token — this goes in your `.env` as `CLOUDFLARE_TUNNEL_TOKEN`
6. Skip the "Route tunnel" step for now (the PaaS API will manage routes)
7. Note the Tunnel ID from the URL or tunnel details page

### Option B: Via CLI

```bash
# Install cloudflared on your Mac (for initial setup only)
brew install cloudflared

# Login to Cloudflare
cloudflared tunnel login

# Create a tunnel
cloudflared tunnel create homelab

# This creates a credentials file and prints the tunnel ID
# Note the Tunnel ID

# Get the tunnel token
cloudflared tunnel token homelab
# Copy this token — it goes in .env as CLOUDFLARE_TUNNEL_TOKEN
```

### Option C: Via API

```bash
# Create tunnel
curl -X POST "https://api.cloudflare.com/client/v4/accounts/{account_id}/cfd_tunnel" \
  -H "Authorization: Bearer {api_token}" \
  -H "Content-Type: application/json" \
  --data '{
    "name": "homelab",
    "tunnel_secret": "'$(openssl rand -base64 32)'"
  }'

# Response includes the tunnel ID and token
```

## 4. Set Up DNS

You have two options:

### Option A: Wildcard CNAME (Recommended)

Create one wildcard record that covers all apps:

1. Go to your domain's DNS settings on Cloudflare
2. Add a CNAME record:
   - **Name:** `*.lab` (or whatever subdomain prefix you want)
   - **Target:** `{TUNNEL_ID}.cfargotunnel.com`
   - **Proxy:** ON (orange cloud)

This means `anything.lab.yourdomain.com` will route through the tunnel.

### Option B: Per-App CNAMEs (API-Managed)

The PaaS API will create individual CNAME records for each app. This is more precise but requires API calls on each deploy. The `homelab-cloudflare` crate handles this automatically.

```bash
# Manual example for testing:
curl -X POST "https://api.cloudflare.com/client/v4/zones/{zone_id}/dns_records" \
  -H "Authorization: Bearer {api_token}" \
  -H "Content-Type: application/json" \
  --data '{
    "type": "CNAME",
    "name": "my-app.lab.yourdomain.com",
    "content": "{TUNNEL_ID}.cfargotunnel.com",
    "proxied": true
  }'
```

## 5. How Tunnel Ingress Works

The PaaS API manages tunnel routing via the Cloudflare API. When an app is deployed:

1. API builds the full ingress rule set from all running apps
2. PUTs to the Cloudflare Tunnel Configuration API:

```
PUT https://api.cloudflare.com/client/v4/accounts/{account_id}/cfd_tunnel/{tunnel_id}/configurations
```

Payload:
```json
{
  "config": {
    "ingress": [
      {
        "hostname": "my-app.lab.yourdomain.com",
        "service": "http://homelab-traefik:80"
      },
      {
        "hostname": "paas.lab.yourdomain.com",
        "service": "http://homelab-traefik:80"
      },
      {
        "service": "http_status:404"
      }
    ]
  }
}
```

Key points:
- **All hostnames route to Traefik**, not directly to app containers
- Traefik uses the `Host` header (preserved by cloudflared) to route to the correct container
- The last rule MUST be a catch-all with no hostname (returns 404)
- `cloudflared` picks up config changes automatically (no restart needed for remote-managed tunnels)

## 6. Environment Variables Summary

After setup, your `.env` should have:

```bash
CLOUDFLARE_API_TOKEN=xxxxx          # From step 1
CLOUDFLARE_ACCOUNT_ID=xxxxx        # From step 2
CLOUDFLARE_ZONE_ID=xxxxx           # From step 2
CLOUDFLARE_TUNNEL_ID=xxxxx         # From step 3
CLOUDFLARE_TUNNEL_TOKEN=xxxxx      # From step 3
BASE_DOMAIN=lab.yourdomain.com     # Your chosen subdomain
```

## 7. Testing the Tunnel

Before the PaaS is built, you can test the tunnel manually:

```bash
# On the laptop, start cloudflared with a simple test
docker run -d --name test-tunnel \
  cloudflare/cloudflared:latest \
  tunnel run --token YOUR_TUNNEL_TOKEN

# Run a test web server
docker run -d --name test-web -p 8888:80 nginx

# Add a temporary ingress rule via the Cloudflare dashboard:
# hostname: test.lab.yourdomain.com → http://localhost:8888

# From your phone or another network, visit:
# https://test.lab.yourdomain.com
# Should see the nginx welcome page

# Clean up
docker rm -f test-tunnel test-web
```

## Troubleshooting

| Issue | Fix |
|-------|-----|
| Tunnel shows "inactive" | Check `docker logs homelab-cloudflared` — token may be wrong |
| 502 Bad Gateway | cloudflared can't reach Traefik — check they're on the same Docker network |
| DNS not resolving | CNAME may need a few minutes to propagate. Check with `dig my-app.lab.yourdomain.com` |
| "No ingress rules" error | Ensure the catch-all rule (no hostname, `http_status:404`) is last |
| SSL errors | Make sure DNS records have the orange cloud (proxied) enabled |
