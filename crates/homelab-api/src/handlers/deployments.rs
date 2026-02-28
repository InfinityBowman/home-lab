use axum::extract::{Path, State};
use axum::Json;
use bollard::Docker;
use homelab_cloudflare::client::CloudflareClient;
use homelab_core::{AppStatus, Deployment, DeployStatus, HomelabError};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{ApiError, ApiResponse};
use crate::handlers::deploy;
use crate::state::AppState;

pub async fn list(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<Vec<Deployment>>>, ApiError> {
    let app = homelab_db::app_repo::get_by_name(&state.db, &name).await?;
    let deployments = homelab_db::deployment_repo::list_by_app(&state.db, &app.id).await?;
    Ok(ApiResponse::ok(deployments))
}

pub async fn get(
    State(state): State<AppState>,
    Path((name, id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Deployment>>, ApiError> {
    let app = homelab_db::app_repo::get_by_name(&state.db, &name).await?;
    let deployment = homelab_db::deployment_repo::get_by_id(&state.db, &id).await?;

    // Verify deployment belongs to this app
    if deployment.app_id != app.id {
        return Err(
            HomelabError::NotFound(format!("deployment {id} not found for app {name}")).into(),
        );
    }

    Ok(ApiResponse::ok(deployment))
}

/// Rollback to a previous deployment's image (no rebuild needed).
pub async fn rollback(
    State(state): State<AppState>,
    Path((name, id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let app = homelab_db::app_repo::get_by_name(&state.db, &name).await?;
    let old_deployment = homelab_db::deployment_repo::get_by_id(&state.db, &id).await?;

    // Verify deployment belongs to this app
    if old_deployment.app_id != app.id {
        return Err(
            HomelabError::NotFound(format!("deployment {id} not found for app {name}")).into(),
        );
    }

    if old_deployment.image_tag.is_empty() {
        return Err(
            HomelabError::InvalidInput("deployment has no image to rollback to".into()).into(),
        );
    }

    // Verify the Docker image still exists locally before returning success
    state
        .docker
        .inspect_image(&old_deployment.image_tag)
        .await
        .map_err(|_| {
            HomelabError::InvalidInput(format!(
                "image {} no longer exists locally",
                old_deployment.image_tag
            ))
        })?;

    // Create a new deployment record for the rollback
    let deployment_id = Uuid::new_v4().to_string();
    let deployment = homelab_db::deployment_repo::create(
        &state.db,
        &deployment_id,
        &app.id,
        &old_deployment.commit_sha,
    )
    .await?;

    let ctx = RollbackContext {
        db: state.db.clone(),
        docker: state.docker.clone(),
        cloudflare: state.cloudflare.clone(),
        app_id: app.id,
        app_name: app.name,
        domain: app.domain,
        port: app.port,
        deployment_id: deployment.id.clone(),
        image_tag: old_deployment.image_tag,
        commit_sha: old_deployment.commit_sha,
    };

    tokio::spawn(execute_rollback(ctx));

    Ok(ApiResponse::ok(serde_json::json!({
        "deployment_id": deployment.id,
        "status": "pending",
        "message": "rollback initiated"
    })))
}

struct RollbackContext {
    db: SqlitePool,
    docker: Docker,
    cloudflare: Option<CloudflareClient>,
    app_id: String,
    app_name: String,
    domain: String,
    port: i64,
    deployment_id: String,
    image_tag: String,
    commit_sha: String,
}

async fn execute_rollback(ctx: RollbackContext) {
    if let Err(e) = execute_rollback_inner(&ctx).await {
        tracing::error!(app = %ctx.app_name, error = %e, "rollback failed");
        let _ = homelab_db::deployment_repo::update_status(
            &ctx.db,
            &ctx.deployment_id,
            &DeployStatus::Failed,
            None,
            Some(&e.to_string()),
        )
        .await;
        let _ =
            homelab_db::app_repo::update_status(&ctx.db, &ctx.app_id, &AppStatus::Failed).await;
    }
}

async fn execute_rollback_inner(ctx: &RollbackContext) -> Result<(), HomelabError> {
    deploy::swap_container(
        &ctx.db,
        &ctx.docker,
        &ctx.app_id,
        &ctx.app_name,
        &ctx.domain,
        ctx.port,
        &ctx.deployment_id,
        &ctx.image_tag,
        Some("rollback — no rebuild"),
        "app.rolled_back",
        &format!("sha={}, image={}", ctx.commit_sha, ctx.image_tag),
    )
    .await?;

    // Sync Cloudflare routes (non-fatal)
    if let Some(cf) = &ctx.cloudflare
        && let Err(e) = deploy::sync_cloudflare(cf, &ctx.db).await
    {
        tracing::warn!(app = %ctx.app_name, error = %e, "cloudflare sync failed (non-fatal)");
    }

    tracing::info!(app = %ctx.app_name, image = %ctx.image_tag, "rollback succeeded");
    Ok(())
}
