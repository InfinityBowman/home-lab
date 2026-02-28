use bollard::Docker;
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, RemoveContainerOptions,
    StartContainerOptions, StopContainerOptions,
};
use bollard::models::{ContainerSummary, HostConfig};
use homelab_core::HomelabError;
use std::collections::HashMap;

use crate::HOMELAB_NETWORK;
use crate::labels;

pub struct ContainerConfig {
    pub app_name: String,
    pub image: String,
    pub port: i64,
    pub domain: String,
    pub env_vars: Vec<(String, String)>,
}

pub(crate) fn container_name(app_name: &str) -> String {
    format!("homelab-{app_name}")
}

pub async fn create_and_start(
    docker: &Docker,
    config: &ContainerConfig,
) -> Result<String, HomelabError> {
    let name = container_name(&config.app_name);

    // Remove existing container if present
    let _ = remove(docker, &config.app_name).await;

    let env: Vec<String> = config
        .env_vars
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect();

    let traefik_labels = labels::traefik_labels(&config.app_name, &config.domain, config.port);

    let port_str = format!("{}/tcp", config.port);
    let mut exposed_ports: HashMap<String, HashMap<(), ()>> = HashMap::new();
    exposed_ports.insert(port_str, HashMap::new());

    let host_config = HostConfig {
        network_mode: Some(HOMELAB_NETWORK.to_string()),
        ..Default::default()
    };

    let container_config = Config {
        image: Some(config.image.clone()),
        env: Some(env),
        labels: Some(traefik_labels),
        exposed_ports: Some(exposed_ports),
        host_config: Some(host_config),
        ..Default::default()
    };

    let opts = CreateContainerOptions {
        name: name.clone(),
        ..Default::default()
    };

    let response = docker
        .create_container(Some(opts), container_config)
        .await
        .map_err(|e| HomelabError::Docker(format!("create container: {e}")))?;

    docker
        .start_container(&name, None::<StartContainerOptions<String>>)
        .await
        .map_err(|e| HomelabError::Docker(format!("start container: {e}")))?;

    tracing::info!(app = %config.app_name, id = %response.id, "container started");
    Ok(response.id)
}

pub async fn stop(docker: &Docker, app_name: &str) -> Result<(), HomelabError> {
    let name = container_name(app_name);
    docker
        .stop_container(&name, Some(StopContainerOptions { t: 10 }))
        .await
        .map_err(|e| HomelabError::Docker(format!("stop container: {e}")))?;

    tracing::info!(app = %app_name, "container stopped");
    Ok(())
}

pub async fn start(docker: &Docker, app_name: &str) -> Result<(), HomelabError> {
    let name = container_name(app_name);
    docker
        .start_container(&name, None::<StartContainerOptions<String>>)
        .await
        .map_err(|e| HomelabError::Docker(format!("start container: {e}")))?;

    tracing::info!(app = %app_name, "container started");
    Ok(())
}

pub async fn restart(docker: &Docker, app_name: &str) -> Result<(), HomelabError> {
    let name = container_name(app_name);
    docker
        .restart_container(
            &name,
            Some(bollard::container::RestartContainerOptions { t: 10 }),
        )
        .await
        .map_err(|e| HomelabError::Docker(format!("restart container: {e}")))?;

    tracing::info!(app = %app_name, "container restarted");
    Ok(())
}

pub async fn remove(docker: &Docker, app_name: &str) -> Result<(), HomelabError> {
    let name = container_name(app_name);
    docker
        .remove_container(
            &name,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await
        .map_err(|e| HomelabError::Docker(format!("remove container: {e}")))?;

    tracing::info!(app = %app_name, "container removed");
    Ok(())
}

#[derive(Debug, serde::Serialize)]
pub struct ContainerStatus {
    pub name: String,
    pub state: String,
    pub image: String,
    pub created: String,
}

pub async fn status(docker: &Docker, app_name: &str) -> Result<ContainerStatus, HomelabError> {
    let name = container_name(app_name);
    let info = docker
        .inspect_container(&name, None)
        .await
        .map_err(|e| HomelabError::Docker(format!("inspect container: {e}")))?;

    let state = info
        .state
        .as_ref()
        .and_then(|s| s.status.as_ref())
        .map(|s| format!("{s:?}"))
        .unwrap_or_else(|| "unknown".to_string());

    Ok(ContainerStatus {
        name,
        state,
        image: info.config.and_then(|c| c.image).unwrap_or_default(),
        created: info.created.unwrap_or_default(),
    })
}

pub async fn list_homelab(docker: &Docker) -> Result<Vec<ContainerSummary>, HomelabError> {
    let mut filters = HashMap::new();
    filters.insert("name", vec!["homelab-"]);

    let opts = ListContainersOptions {
        all: true,
        filters,
        ..Default::default()
    };

    docker
        .list_containers(Some(opts))
        .await
        .map_err(|e| HomelabError::Docker(format!("list containers: {e}")))
}
