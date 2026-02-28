pub mod client;
pub mod dns;
pub mod tunnel;

use homelab_core::HomelabError;

use client::CloudflareClient;

const TRAEFIK_SERVICE: &str = "http://homelab-traefik:80";

/// Sync all tunnel ingress rules from the current set of running apps.
///
/// Takes a list of hostnames (e.g., `["my-app.lab.dev", "other.lab.dev"]`).
/// Builds the full ingress config and PUTs it to the Cloudflare API.
/// Also ensures CNAME records exist for each hostname.
pub async fn sync_routes(
    client: &CloudflareClient,
    hostnames: &[String],
) -> Result<(), HomelabError> {
    // Build ingress rules — all hostnames route to Traefik
    let routes: Vec<(String, String)> = hostnames
        .iter()
        .map(|h| (h.clone(), TRAEFIK_SERVICE.to_string()))
        .collect();

    let rules = tunnel::build_ingress_rules(&routes);

    // PUT full tunnel config
    tunnel::put_ingress(client, &rules).await?;

    // Ensure DNS records exist for each hostname
    for hostname in hostnames {
        dns::ensure_cname(client, hostname).await?;
    }

    tracing::info!(count = hostnames.len(), "cloudflare routes synced");
    Ok(())
}

/// Remove a single hostname's CNAME record from Cloudflare DNS.
/// Call this when an app is deleted.
///
/// Note: This only removes the DNS record. Tunnel ingress rules are
/// updated separately via `sync_routes` (which reads active apps from DB).
pub async fn remove_dns(client: &CloudflareClient, hostname: &str) -> Result<(), HomelabError> {
    dns::delete_cname(client, hostname).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traefik_service_is_correct() {
        assert_eq!(TRAEFIK_SERVICE, "http://homelab-traefik:80");
    }
}
