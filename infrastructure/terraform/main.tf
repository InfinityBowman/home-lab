terraform {
  required_version = ">= 1.0"

  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 5.15.0"
    }
  }
}

provider "cloudflare" {
  api_token = var.cloudflare_api_token
}

# ─── Rate Limiting ────────────────────────────────────────────────────────────

resource "cloudflare_ruleset" "rate_limiting" {
  zone_id = var.zone_id
  name    = "zone-wide rate limiting"
  kind    = "zone"
  phase   = "http_ratelimit"

  rules = [
    {
      description = "Rate limit API endpoints"
      action      = "block"
      expression  = "starts_with(http.request.uri.path, \"/api/\")"
      enabled     = true
      ratelimit = {
        characteristics     = ["cf.colo.id", "ip.src"]
        period              = 10
        requests_per_period = 20
        mitigation_timeout  = 10
      }
    },
  ]
}

# ─── WAF Custom Rules ────────────────────────────────────────────────────────

resource "cloudflare_ruleset" "waf_custom" {
  zone_id = var.zone_id
  name    = "zone WAF custom rules"
  kind    = "zone"
  phase   = "http_request_firewall_custom"

  rules = [
    {
      description = "Block sensitive file probes"
      action      = "block"
      expression  = join(" or ", [
        "http.request.uri.path contains \"/.env\"",
        "http.request.uri.path contains \"/.git\"",
        "http.request.uri.path contains \"/.svn\"",
        "http.request.uri.path contains \"/.DS_Store\"",
        "http.request.uri.path contains \"/wp-login.php\"",
        "http.request.uri.path contains \"/wp-admin\"",
        "http.request.uri.path contains \"/xmlrpc.php\"",
        "http.request.uri.path contains \"/phpmyadmin\"",
        "http.request.uri.path contains \"/adminer\"",
        "http.request.uri.path contains \"/phpinfo\"",
        "http.request.uri.path contains \"/config.php\"",
        "http.request.uri.path contains \"/config.yml\"",
        "http.request.uri.path contains \"/config.json\"",
        "http.request.uri.path contains \"/.htaccess\"",
        "http.request.uri.path contains \"/.htpasswd\"",
        "http.request.uri.path contains \"/web.config\"",
        "http.request.uri.path contains \"/backup.sql\"",
        "http.request.uri.path contains \"/dump.sql\"",
        "http.request.uri.path contains \"/database.sql\"",
      ])
      enabled = true
    },
    {
      description = "Block path traversal"
      action      = "block"
      expression  = "http.request.uri.path contains \"../\" or http.request.uri.path contains \"..\\\\\" or http.request.uri contains \"..%2f\" or http.request.uri contains \"..%5c\""
      enabled     = true
    },
    {
      description = "Block common exploit paths"
      action      = "block"
      expression  = join(" or ", [
        "http.request.uri.path contains \"/cgi-bin/\"",
        "http.request.uri.path contains \"/wp-includes/\"",
        "http.request.uri.path contains \"/wp-content/uploads/\"",
        "(http.request.uri.path contains \"/.well-known/\" and not http.request.uri.path contains \"/.well-known/acme-challenge/\")",
        "http.request.uri.path contains \"/vendor/\"",
        "http.request.uri.path contains \"/node_modules/\"",
        "http.request.uri.path contains \"/.vscode/\"",
        "http.request.uri.path contains \".php\"",
        "http.request.uri.path contains \".asp\"",
        "http.request.uri.path contains \".aspx\"",
        "http.request.uri.path contains \".jsp\"",
      ])
      enabled = true
    },
    {
      description = "Block suspicious query strings (SQLi, RCE, XSS)"
      action      = "block"
      expression  = join(" or ", [
        "lower(http.request.uri.query) contains \"union select\"",
        "lower(http.request.uri.query) contains \"or 1=1\"",
        "lower(http.request.uri.query) contains \"drop table\"",
        "lower(http.request.uri.query) contains \"/etc/passwd\"",
        "lower(http.request.uri.query) contains \"/bin/sh\"",
        "lower(http.request.uri.query) contains \"cmd.exe\"",
        "lower(http.request.uri.query) contains \"<script\"",
        "lower(http.request.uri.query) contains \"javascript:\"",
        "lower(http.request.uri.query) contains \"onerror=\"",
      ])
      enabled = true
    },
  ]
}

# ─── Cloudflare Access (Zero Trust) ──────────────────────────────────────────

resource "cloudflare_zero_trust_access_policy" "admin_only" {
  account_id       = var.account_id
  name             = "Admin only"
  decision         = "allow"
  session_duration = "24h"

  include = [
    {
      email = {
        email = var.admin_email
      }
    },
  ]
}

resource "cloudflare_zero_trust_access_application" "dozzle" {
  zone_id  = var.zone_id
  name     = "Dozzle Logs"
  domain   = "dozzle.${var.domain}"
  type     = "self_hosted"
  policies = [{ id = cloudflare_zero_trust_access_policy.admin_only.id }]
}

resource "cloudflare_zero_trust_access_application" "paas_api" {
  zone_id  = var.zone_id
  name     = "PaaS API"
  domain   = "paas.${var.domain}"
  type     = "self_hosted"
  policies = [{ id = cloudflare_zero_trust_access_policy.admin_only.id }]
}

resource "cloudflare_zero_trust_access_application" "dashboard" {
  zone_id  = var.zone_id
  name     = "PaaS Dashboard"
  domain   = "dashboard.${var.domain}"
  type     = "self_hosted"
  policies = [{ id = cloudflare_zero_trust_access_policy.admin_only.id }]
}

resource "cloudflare_zero_trust_access_application" "n8n" {
  zone_id  = var.zone_id
  name     = "n8n Workflows"
  domain   = "n8n.${var.domain}"
  type     = "self_hosted"
  policies = [{ id = cloudflare_zero_trust_access_policy.admin_only.id }]
}
