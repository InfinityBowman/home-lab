use axum::Json;
use axum::extract::{Path, Query, State};
use homelab_core::{AppStatus, HomelabError};
use serde::Deserialize;

use crate::error::{ApiError, ApiResponse};
use crate::state::AppState;

pub async fn start(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let app = homelab_db::app_repo::get_by_name(&state.db, &name).await?;

    if app.docker_image.is_empty() {
        return Err(
            HomelabError::InvalidInput("app has no docker image — deploy first".into()).into(),
        );
    }

    let env_vars = homelab_db::env_var_repo::get_by_app(&state.db, &app.id).await?;
    let env: Vec<(String, String)> = env_vars.into_iter().map(|e| (e.key, e.value)).collect();

    let config = homelab_docker::containers::ContainerConfig {
        app_name: app.name.clone(),
        image: app.docker_image.clone(),
        port: app.port,
        domain: app.domain.clone(),
        env_vars: env,
    };

    homelab_docker::containers::create_and_start(&state.docker, &config).await?;
    homelab_db::app_repo::update_status(&state.db, &app.id, &AppStatus::Running).await?;

    homelab_db::audit_repo::create(&state.db, Some(&app.id), "container.started", None).await?;

    Ok(ApiResponse::ok_empty())
}

pub async fn stop(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let app = homelab_db::app_repo::get_by_name(&state.db, &name).await?;

    homelab_docker::containers::stop(&state.docker, &app.name).await?;
    homelab_db::app_repo::update_status(&state.db, &app.id, &AppStatus::Stopped).await?;

    homelab_db::audit_repo::create(&state.db, Some(&app.id), "container.stopped", None).await?;

    Ok(ApiResponse::ok_empty())
}

pub async fn restart(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let app = homelab_db::app_repo::get_by_name(&state.db, &name).await?;

    homelab_docker::containers::restart(&state.docker, &app.name).await?;
    homelab_db::app_repo::update_status(&state.db, &app.id, &AppStatus::Running).await?;

    homelab_db::audit_repo::create(&state.db, Some(&app.id), "container.restarted", None).await?;

    Ok(ApiResponse::ok_empty())
}

pub async fn status(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<homelab_docker::containers::ContainerStatus>>, ApiError> {
    let app = homelab_db::app_repo::get_by_name(&state.db, &name).await?;
    let status = homelab_docker::containers::status(&state.docker, &app.name).await?;
    Ok(ApiResponse::ok(status))
}

#[derive(Deserialize)]
pub struct LogsQuery {
    #[serde(default = "default_tail")]
    pub tail: u64,
}

fn default_tail() -> u64 {
    100
}

pub async fn logs(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<ApiResponse<Vec<String>>>, ApiError> {
    let app = homelab_db::app_repo::get_by_name(&state.db, &name).await?;
    let tail = query.tail.min(5_000);
    let lines = homelab_docker::logs::get_logs(&state.docker, &app.name, tail).await?;
    Ok(ApiResponse::ok(lines))
}
