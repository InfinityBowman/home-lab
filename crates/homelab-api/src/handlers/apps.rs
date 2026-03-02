use axum::Json;
use axum::extract::{Path, State};
use homelab_core::{App, CreateAppRequest, HomelabError, UpdateAppRequest};
use uuid::Uuid;

use crate::error::{ApiError, ApiResponse};
use crate::state::AppState;

pub async fn list(State(state): State<AppState>) -> Result<Json<ApiResponse<Vec<App>>>, ApiError> {
    let apps = homelab_db::app_repo::list(&state.db).await?;
    Ok(ApiResponse::ok(apps))
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateAppRequest>,
) -> Result<Json<ApiResponse<App>>, ApiError> {
    // Validate name (DNS label rules)
    let name = &req.name;
    if name.is_empty() || name.len() > 63 {
        return Err(HomelabError::InvalidInput("app name must be 1-63 characters".into()).into());
    }
    if !name.starts_with(|c: char| c.is_ascii_lowercase()) {
        return Err(HomelabError::InvalidInput(
            "app name must start with a lowercase letter".into(),
        )
        .into());
    }
    if name.ends_with('-') {
        return Err(
            HomelabError::InvalidInput("app name must not end with a hyphen".into()).into(),
        );
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(HomelabError::InvalidInput(
            "app name must be lowercase alphanumeric with hyphens only".into(),
        )
        .into());
    }

    // Validate port
    if req.port < 1 || req.port > 65535 {
        return Err(HomelabError::InvalidInput("port must be between 1 and 65535".into()).into());
    }

    let id = Uuid::new_v4().to_string();
    let domain = format!("{}.{}", req.name, state.config.base_domain);
    let git_repo_path = format!("{}/{}.git", state.config.git_repos_path, req.name);

    let app =
        homelab_db::app_repo::create(&state.db, &id, &req.name, &domain, &git_repo_path, req.port)
            .await?;

    // Create bare git repo — clean up DB record on failure
    if let Err(e) = homelab_git::repo::init_bare(&git_repo_path).await {
        let _ = homelab_db::app_repo::delete(&state.db, &id).await;
        return Err(e.into());
    }

    // Write post-receive hook — clean up both git repo and DB on failure
    if let Err(e) = homelab_git::hooks::write_post_receive(
        &git_repo_path,
        &req.name,
        &state.config.internal_hook_secret,
        state.config.api_port,
    )
    .await
    {
        let _ = homelab_git::repo::remove(&git_repo_path).await;
        let _ = homelab_db::app_repo::delete(&state.db, &id).await;
        return Err(e.into());
    }

    // Seed the repo with a starter Dockerfile and app (non-fatal)
    if let Err(e) =
        homelab_git::repo::seed_initial_commit(&git_repo_path, &req.name, req.port).await
    {
        tracing::warn!(app = %req.name, error = %e, "failed to seed initial commit");
    }

    homelab_db::audit_repo::create(
        &state.db,
        Some(&id),
        "app.created",
        Some(&format!("name={}, port={}", req.name, req.port)),
    )
    .await?;

    tracing::info!(app = %req.name, "app created");
    Ok(ApiResponse::ok(app))
}

pub async fn get(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<App>>, ApiError> {
    let app = homelab_db::app_repo::get_by_name(&state.db, &name).await?;
    Ok(ApiResponse::ok(app))
}

pub async fn update(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<UpdateAppRequest>,
) -> Result<Json<ApiResponse<App>>, ApiError> {
    let existing = homelab_db::app_repo::get_by_name(&state.db, &name).await?;
    let app =
        homelab_db::app_repo::update(&state.db, &existing.id, req.port, req.domain.as_deref())
            .await?;
    Ok(ApiResponse::ok(app))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let app = homelab_db::app_repo::get_by_name(&state.db, &name).await?;

    // Try to remove the container (ignore errors if it doesn't exist)
    let _ = homelab_docker::containers::remove(&state.docker, &name).await;

    // Remove the bare git repo
    let _ = homelab_git::repo::remove(&app.git_repo_path).await;

    // Remove Cloudflare DNS record (non-fatal)
    if let Some(cf) = &state.cloudflare
        && let Err(e) = homelab_cloudflare::remove_dns(cf, &app.domain).await
    {
        tracing::warn!(app = %name, error = %e, "cloudflare DNS cleanup failed");
    }

    homelab_db::app_repo::delete(&state.db, &app.id).await?;

    // Resync tunnel ingress now that app is removed from DB
    if let Some(cf) = &state.cloudflare
        && let Err(e) = crate::handlers::deploy::sync_cloudflare(cf, &state.db).await
    {
        tracing::warn!(app = %name, error = %e, "cloudflare ingress resync failed");
    }

    homelab_db::audit_repo::create(
        &state.db,
        Some(&app.id),
        "app.deleted",
        Some(&format!("name={name}")),
    )
    .await?;

    tracing::info!(app = %name, "app deleted");
    Ok(ApiResponse::ok_empty())
}
