use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::error::{ApiError, ApiResponse};
use crate::state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

pub async fn health() -> Json<ApiResponse<HealthResponse>> {
    ApiResponse::ok(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

#[derive(Serialize)]
pub struct SystemInfo {
    pub version: String,
    pub docker_version: String,
    pub app_count: i64,
    pub container_count: usize,
}

pub async fn info(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SystemInfo>>, ApiError> {
    let apps = homelab_db::app_repo::list(&state.db).await?;
    let containers = homelab_docker::containers::list_homelab(&state.docker).await?;

    let docker_version = state
        .docker
        .version()
        .await
        .ok()
        .and_then(|v| v.version)
        .unwrap_or_else(|| "unknown".into());

    Ok(ApiResponse::ok(SystemInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        docker_version,
        app_count: apps.len() as i64,
        container_count: containers.len(),
    }))
}
