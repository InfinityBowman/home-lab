use homelab_core::HomelabError;
use serde::{Deserialize, Serialize};

use crate::client::{CfApiResponse, CloudflareClient, cf_error};

/// A single tunnel ingress rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressRule {
    /// Hostname to match (e.g., "my-app.lab.example.com"). Omit for the catch-all.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    /// Backend service URL (e.g., "http://homelab-traefik:80") or special like "http_status:404".
    pub service: String,
}

/// Full tunnel configuration payload.
#[derive(Debug, Serialize)]
struct TunnelConfigPayload {
    config: TunnelConfig,
}

#[derive(Debug, Serialize)]
struct TunnelConfig {
    ingress: Vec<IngressRule>,
}

/// Build ingress rules from a list of (hostname, service) pairs.
/// Automatically appends the required catch-all rule at the end.
pub fn build_ingress_rules(routes: &[(String, String)]) -> Vec<IngressRule> {
    let mut rules: Vec<IngressRule> = routes
        .iter()
        .map(|(hostname, service)| IngressRule {
            hostname: Some(hostname.clone()),
            service: service.clone(),
        })
        .collect();

    // Cloudflare requires a catch-all rule with no hostname as the last entry
    rules.push(IngressRule {
        hostname: None,
        service: "http_status:404".into(),
    });

    rules
}

/// PUT the full tunnel ingress configuration to the Cloudflare API.
/// This replaces the entire ingress config — callers must include all routes.
pub async fn put_ingress(
    client: &CloudflareClient,
    rules: &[IngressRule],
) -> Result<(), HomelabError> {
    let tunnel_id = client.tunnel_id();
    let url = client.account_url(&format!("/cfd_tunnel/{tunnel_id}/configurations"));

    let payload = TunnelConfigPayload {
        config: TunnelConfig {
            ingress: rules.to_vec(),
        },
    };

    let resp = client
        .http
        .put(&url)
        .header("Authorization", client.auth_header())
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| HomelabError::Cloudflare(format!("tunnel PUT request: {e}")))?;

    let status = resp.status();
    let body: CfApiResponse = resp
        .json()
        .await
        .map_err(|e| HomelabError::Cloudflare(format!("tunnel response parse ({status}): {e}")))?;

    if !body.success {
        return Err(cf_error("tunnel config failed", status, body.errors));
    }

    tracing::info!(rules = rules.len(), "tunnel ingress updated");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_ingress_rules_appends_catchall() {
        let routes = vec![("app.lab.dev".into(), "http://homelab-traefik:80".into())];
        let rules = build_ingress_rules(&routes);
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].hostname.as_deref(), Some("app.lab.dev"));
        assert_eq!(rules[0].service, "http://homelab-traefik:80");
        assert!(rules[1].hostname.is_none());
        assert_eq!(rules[1].service, "http_status:404");
    }

    #[test]
    fn build_ingress_rules_empty_routes_has_catchall() {
        let rules = build_ingress_rules(&[]);
        assert_eq!(rules.len(), 1);
        assert!(rules[0].hostname.is_none());
        assert_eq!(rules[0].service, "http_status:404");
    }

    #[test]
    fn build_ingress_rules_multiple_routes() {
        let routes = vec![
            ("a.lab.dev".into(), "http://homelab-traefik:80".into()),
            ("b.lab.dev".into(), "http://homelab-traefik:80".into()),
            ("paas.lab.dev".into(), "http://homelab-traefik:80".into()),
        ];
        let rules = build_ingress_rules(&routes);
        assert_eq!(rules.len(), 4); // 3 routes + 1 catch-all
        // Last rule is always the catch-all
        assert!(rules[3].hostname.is_none());
    }

    #[test]
    fn ingress_rule_serializes_without_hostname_when_none() {
        let rule = IngressRule {
            hostname: None,
            service: "http_status:404".into(),
        };
        let json = serde_json::to_value(&rule).unwrap();
        assert!(!json.as_object().unwrap().contains_key("hostname"));
        assert_eq!(json["service"], "http_status:404");
    }
}
