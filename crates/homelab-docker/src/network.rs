use bollard::network::{CreateNetworkOptions, ListNetworksOptions};
use bollard::Docker;
use homelab_core::HomelabError;
use std::collections::HashMap;

use crate::HOMELAB_NETWORK;

/// Ensure the `homelab` bridge network exists. Creates it if missing.
/// Safe to call multiple times (idempotent).
pub async fn ensure_network(docker: &Docker) -> Result<(), HomelabError> {
    let mut filters = HashMap::new();
    filters.insert("name".to_string(), vec![HOMELAB_NETWORK.to_string()]);

    let networks = docker
        .list_networks(Some(ListNetworksOptions { filters }))
        .await
        .map_err(|e| HomelabError::Docker(format!("list networks: {e}")))?;

    // Check for exact name match (Docker's filter is a substring match)
    let exists = networks
        .iter()
        .any(|n| n.name.as_deref() == Some(HOMELAB_NETWORK));

    if exists {
        tracing::debug!("network '{HOMELAB_NETWORK}' already exists");
        return Ok(());
    }

    let options = CreateNetworkOptions {
        name: HOMELAB_NETWORK,
        driver: "bridge",
        ..Default::default()
    };

    docker
        .create_network(options)
        .await
        .map_err(|e| HomelabError::Docker(format!("create network: {e}")))?;

    tracing::info!("created Docker network '{HOMELAB_NETWORK}'");
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::HOMELAB_NETWORK;

    #[test]
    fn network_name_is_homelab() {
        assert_eq!(HOMELAB_NETWORK, "homelab");
    }
}
