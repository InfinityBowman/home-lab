use homelab_core::HomelabError;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::fmt;

const CF_API_BASE: &str = "https://api.cloudflare.com/client/v4";

/// Configuration for the Cloudflare API client.
#[derive(Clone)]
pub struct CloudflareConfig {
    pub api_token: String,
    pub account_id: String,
    pub tunnel_id: String,
    pub zone_id: String,
}

// Manual Debug impl to redact the API token from logs.
impl fmt::Debug for CloudflareConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CloudflareConfig")
            .field("api_token", &"[REDACTED]")
            .field("account_id", &self.account_id)
            .field("tunnel_id", &self.tunnel_id)
            .field("zone_id", &self.zone_id)
            .finish()
    }
}

/// Cloudflare API client wrapping reqwest with auth headers.
#[derive(Clone)]
pub struct CloudflareClient {
    pub(crate) http: Client,
    pub(crate) config: CloudflareConfig,
}

impl CloudflareClient {
    pub fn new(config: CloudflareConfig) -> Result<Self, HomelabError> {
        let http = Client::builder()
            .build()
            .map_err(|e| HomelabError::Internal(format!("cloudflare http client: {e}")))?;

        Ok(Self { http, config })
    }

    /// Build the full URL for an account-scoped API path.
    pub(crate) fn account_url(&self, path: &str) -> String {
        format!("{CF_API_BASE}/accounts/{}{path}", self.config.account_id)
    }

    /// Build the full URL for a zone-scoped API path.
    pub(crate) fn zone_url(&self, path: &str) -> String {
        format!("{CF_API_BASE}/zones/{}{path}", self.config.zone_id)
    }

    /// Get the Authorization header value.
    pub(crate) fn auth_header(&self) -> String {
        format!("Bearer {}", self.config.api_token)
    }

    /// Get the tunnel ID.
    pub fn tunnel_id(&self) -> &str {
        &self.config.tunnel_id
    }
}

// ─── Shared Cloudflare API response types ──────────────────────────────────

/// Standard Cloudflare API response envelope.
#[derive(Debug, Deserialize)]
pub(crate) struct CfApiResponse<T = serde_json::Value> {
    pub success: bool,
    pub result: Option<T>,
    pub errors: Vec<CfApiError>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CfApiError {
    pub message: String,
}

/// Build a `HomelabError::Cloudflare` from a failed Cloudflare API response.
pub(crate) fn cf_error(context: &str, status: StatusCode, errors: Vec<CfApiError>) -> HomelabError {
    let msgs: Vec<String> = errors.into_iter().map(|e| e.message).collect();
    HomelabError::Cloudflare(format!("{context} ({status}): {}", msgs.join("; ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CloudflareConfig {
        CloudflareConfig {
            api_token: "test-token".into(),
            account_id: "acc-123".into(),
            tunnel_id: "tun-456".into(),
            zone_id: "zone-789".into(),
        }
    }

    #[test]
    fn account_url_builds_correctly() {
        let client = CloudflareClient::new(test_config()).unwrap();
        assert_eq!(
            client.account_url("/cfd_tunnel/tun-456/configurations"),
            "https://api.cloudflare.com/client/v4/accounts/acc-123/cfd_tunnel/tun-456/configurations"
        );
    }

    #[test]
    fn zone_url_builds_correctly() {
        let client = CloudflareClient::new(test_config()).unwrap();
        assert_eq!(
            client.zone_url("/dns_records"),
            "https://api.cloudflare.com/client/v4/zones/zone-789/dns_records"
        );
    }

    #[test]
    fn auth_header_format() {
        let client = CloudflareClient::new(test_config()).unwrap();
        assert_eq!(client.auth_header(), "Bearer test-token");
    }

    #[test]
    fn debug_redacts_api_token() {
        let config = test_config();
        let debug = format!("{config:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("test-token"));
    }
}
