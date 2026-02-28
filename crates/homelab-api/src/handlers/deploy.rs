use axum::Json;
use axum::extract::{Path, State};
use bollard::Docker;
use homelab_cloudflare::client::CloudflareClient;
use homelab_core::{AppStatus, DeployStatus, HomelabError};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{ApiError, ApiResponse};
use crate::state::AppState;

/// Validate that a string looks like a 40-character hex git SHA.
pub(crate) fn validate_sha(sha: &str) -> Result<(), HomelabError> {
    if sha.len() != 40 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(HomelabError::InvalidInput("invalid commit SHA".into()));
    }
    Ok(())
}

/// Manual deploy trigger — deploys from HEAD of main.
pub async fn trigger(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let app = homelab_db::app_repo::get_by_name(&state.db, &name).await?;

    let commit_sha = homelab_git::repo::get_head_sha(&app.git_repo_path).await?;
    validate_sha(&commit_sha)?;

    let deployment_id = Uuid::new_v4().to_string();
    let deployment =
        homelab_db::deployment_repo::create(&state.db, &deployment_id, &app.id, &commit_sha)
            .await?;

    let ctx = DeployContext {
        db: state.db.clone(),
        docker: state.docker.clone(),
        cloudflare: state.cloudflare.clone(),
        app_id: app.id,
        app_name: app.name,
        git_repo_path: app.git_repo_path,
        domain: app.domain,
        port: app.port,
        deployment_id: deployment.id.clone(),
        commit_sha,
    };

    tokio::spawn(execute(ctx));

    Ok(ApiResponse::ok(serde_json::json!({
        "deployment_id": deployment.id,
        "status": "pending"
    })))
}

// ─── Deploy pipeline ────────────────────────────────────────────────────────

pub struct DeployContext {
    pub db: SqlitePool,
    pub docker: Docker,
    pub cloudflare: Option<CloudflareClient>,
    pub app_id: String,
    pub app_name: String,
    pub git_repo_path: String,
    pub domain: String,
    pub port: i64,
    pub deployment_id: String,
    pub commit_sha: String,
}

/// Execute the full deploy pipeline. Intended to run via `tokio::spawn`.
pub async fn execute(ctx: DeployContext) {
    if let Err(e) = execute_inner(&ctx).await {
        tracing::error!(app = %ctx.app_name, error = %e, "deploy failed");
        let _ = homelab_db::deployment_repo::update_status(
            &ctx.db,
            &ctx.deployment_id,
            &DeployStatus::Failed,
            None,
            Some(&e.to_string()),
        )
        .await;
        let _ = homelab_db::app_repo::update_status(&ctx.db, &ctx.app_id, &AppStatus::Failed).await;
    }
}

async fn execute_inner(ctx: &DeployContext) -> Result<(), HomelabError> {
    let short_sha: String = ctx.commit_sha.chars().take(8).collect();
    let build_dir = format!("/tmp/homelab-build/{}-{short_sha}", ctx.app_name);

    // Step 1: Mark as building
    homelab_db::app_repo::update_status(&ctx.db, &ctx.app_id, &AppStatus::Building).await?;
    homelab_db::deployment_repo::update_status(
        &ctx.db,
        &ctx.deployment_id,
        &DeployStatus::Building,
        None,
        None,
    )
    .await?;

    // Step 2: Checkout code from bare repo
    if let Err(e) =
        homelab_git::repo::checkout(&ctx.git_repo_path, &ctx.commit_sha, &build_dir).await
    {
        let _ = tokio::fs::remove_dir_all(&build_dir).await;
        return Err(e);
    }

    // Step 3: Build Docker image
    let (image_tag, build_log) = match homelab_docker::builder::build_image(
        &ctx.docker,
        &build_dir,
        &ctx.app_name,
        &ctx.commit_sha,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            let _ = tokio::fs::remove_dir_all(&build_dir).await;
            return Err(e);
        }
    };

    // Clean up build dir now that image is built
    let _ = tokio::fs::remove_dir_all(&build_dir).await;

    // Steps 4-8: Swap container and finalize
    swap_container(
        &ctx.db,
        &ctx.docker,
        &ctx.app_id,
        &ctx.app_name,
        &ctx.domain,
        ctx.port,
        &ctx.deployment_id,
        &image_tag,
        Some(&build_log),
        "app.deployed",
        &format!("sha={}, image={image_tag}", ctx.commit_sha),
    )
    .await?;

    // Sync Cloudflare tunnel + DNS (non-fatal — container is already running)
    if let Some(cf) = &ctx.cloudflare
        && let Err(e) = sync_cloudflare(cf, &ctx.db).await
    {
        tracing::warn!(app = %ctx.app_name, error = %e, "cloudflare sync failed (non-fatal)");
    }

    tracing::info!(
        app = %ctx.app_name,
        sha = %ctx.commit_sha,
        image = %image_tag,
        "deploy succeeded"
    );

    Ok(())
}

/// Shared logic for swapping a container to a (new or existing) image.
/// Used by both fresh deploys and rollbacks.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn swap_container(
    db: &SqlitePool,
    docker: &Docker,
    app_id: &str,
    app_name: &str,
    domain: &str,
    port: i64,
    deployment_id: &str,
    image_tag: &str,
    build_log: Option<&str>,
    audit_action: &str,
    audit_details: &str,
) -> Result<(), HomelabError> {
    // Mark deployment as deploying
    homelab_db::deployment_repo::update_status(
        db,
        deployment_id,
        &DeployStatus::Deploying,
        Some(image_tag),
        build_log,
    )
    .await?;

    // Stop old container (ignore errors if not running)
    let _ = homelab_docker::containers::remove(docker, app_name).await;

    // Start new container with env vars + Traefik labels
    let env_vars = homelab_db::env_var_repo::get_by_app(db, app_id).await?;
    let env: Vec<(String, String)> = env_vars.into_iter().map(|e| (e.key, e.value)).collect();

    let container_config = homelab_docker::containers::ContainerConfig {
        app_name: app_name.to_string(),
        image: image_tag.to_string(),
        port,
        domain: domain.to_string(),
        env_vars: env,
    };

    homelab_docker::containers::create_and_start(docker, &container_config).await?;

    // Update app image + status
    homelab_db::app_repo::update_image(db, app_id, image_tag).await?;
    homelab_db::app_repo::update_status(db, app_id, &AppStatus::Running).await?;

    // Finalize deployment
    homelab_db::deployment_repo::update_status(
        db,
        deployment_id,
        &DeployStatus::Succeeded,
        Some(image_tag),
        build_log,
    )
    .await?;

    // Audit
    homelab_db::audit_repo::create(db, Some(app_id), audit_action, Some(audit_details)).await?;

    Ok(())
}

/// Sync all running app routes to Cloudflare Tunnel + DNS.
/// Fetches all running apps from DB and PUTs the full ingress config.
pub(crate) async fn sync_cloudflare(
    cf: &CloudflareClient,
    db: &SqlitePool,
) -> Result<(), HomelabError> {
    let apps = homelab_db::app_repo::list(db).await?;
    let hostnames: Vec<String> = apps
        .into_iter()
        .filter(|a| a.status == AppStatus::Running)
        .map(|a| a.domain)
        .collect();

    homelab_cloudflare::sync_routes(cf, &hostnames).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_sha_accepts_valid() {
        assert!(validate_sha("a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2").is_ok());
    }

    #[test]
    fn validate_sha_accepts_all_hex_digits() {
        assert!(validate_sha("0123456789abcdef0123456789abcdef01234567").is_ok());
        assert!(validate_sha("AABBCCDDEE0011223344AABBCCDDEE0011223344").is_ok());
    }

    #[test]
    fn validate_sha_rejects_too_short() {
        assert!(validate_sha("abc123").is_err());
    }

    #[test]
    fn validate_sha_rejects_too_long() {
        assert!(validate_sha("a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2ff").is_err());
    }

    #[test]
    fn validate_sha_rejects_non_hex() {
        assert!(validate_sha("g1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2").is_err());
    }

    #[test]
    fn validate_sha_rejects_empty() {
        assert!(validate_sha("").is_err());
    }

    #[test]
    fn validate_sha_accepts_null_sha() {
        // Git sends all-zeros for various operations — it's valid hex
        assert!(validate_sha("0000000000000000000000000000000000000000").is_ok());
    }
}
