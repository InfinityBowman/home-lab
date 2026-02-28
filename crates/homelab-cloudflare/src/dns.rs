use homelab_core::HomelabError;
use serde::Deserialize;

use crate::client::{CfApiResponse, CloudflareClient, cf_error};

#[derive(Debug, Deserialize)]
struct DnsRecord {
    id: String,
}

/// Ensure a proxied CNAME record exists for the given hostname.
/// Points to `{tunnel_id}.cfargotunnel.com`.
///
/// If the record already exists, this is a no-op.
pub async fn ensure_cname(client: &CloudflareClient, hostname: &str) -> Result<(), HomelabError> {
    let tunnel_target = format!("{}.cfargotunnel.com", client.tunnel_id());

    // Check if CNAME already exists
    let existing = find_cname(client, hostname).await?;
    if existing.is_some() {
        tracing::debug!(hostname, "CNAME already exists");
        return Ok(());
    }

    // Create the CNAME record
    let url = client.zone_url("/dns_records");

    let body = serde_json::json!({
        "type": "CNAME",
        "name": hostname,
        "content": tunnel_target,
        "proxied": true
    });

    let resp = client
        .http
        .post(&url)
        .header("Authorization", client.auth_header())
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| HomelabError::Cloudflare(format!("DNS create request: {e}")))?;

    let status = resp.status();
    let api_resp: CfApiResponse<DnsRecord> = resp.json().await.map_err(|e| {
        HomelabError::Cloudflare(format!("DNS create response parse ({status}): {e}"))
    })?;

    if !api_resp.success {
        return Err(cf_error("DNS create failed", status, api_resp.errors));
    }

    tracing::info!(hostname, target = %tunnel_target, "CNAME created");
    Ok(())
}

/// Delete the CNAME record for the given hostname if it exists.
pub async fn delete_cname(client: &CloudflareClient, hostname: &str) -> Result<(), HomelabError> {
    let record_id = match find_cname(client, hostname).await? {
        Some(id) => id,
        None => {
            tracing::debug!(hostname, "no CNAME to delete");
            return Ok(());
        }
    };

    let url = client.zone_url(&format!("/dns_records/{record_id}"));

    let resp = client
        .http
        .delete(&url)
        .header("Authorization", client.auth_header())
        .send()
        .await
        .map_err(|e| HomelabError::Cloudflare(format!("DNS delete request: {e}")))?;

    let status = resp.status();
    let api_resp: CfApiResponse = resp.json().await.map_err(|e| {
        HomelabError::Cloudflare(format!("DNS delete response parse ({status}): {e}"))
    })?;

    if !api_resp.success {
        return Err(cf_error("DNS delete failed", status, api_resp.errors));
    }

    tracing::info!(hostname, "CNAME deleted");
    Ok(())
}

/// Find a CNAME record by name. Returns the record ID if found.
async fn find_cname(
    client: &CloudflareClient,
    hostname: &str,
) -> Result<Option<String>, HomelabError> {
    let url = client.zone_url("/dns_records");

    let resp = client
        .http
        .get(&url)
        .header("Authorization", client.auth_header())
        .query(&[("type", "CNAME"), ("name", hostname)])
        .send()
        .await
        .map_err(|e| HomelabError::Cloudflare(format!("DNS list request: {e}")))?;

    let status = resp.status();
    let api_resp: CfApiResponse<Vec<DnsRecord>> = resp.json().await.map_err(|e| {
        HomelabError::Cloudflare(format!("DNS list response parse ({status}): {e}"))
    })?;

    if !api_resp.success {
        return Err(cf_error("DNS list failed", status, api_resp.errors));
    }

    Ok(api_resp
        .result
        .and_then(|records| records.into_iter().next().map(|r| r.id)))
}
