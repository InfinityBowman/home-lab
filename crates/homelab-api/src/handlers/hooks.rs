use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiError, ApiResponse};
use crate::handlers::deploy::{self, DeployContext};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct GitPushPayload {
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub commit_sha: String,
}

/// Called by the post-receive hook when code is pushed to a bare repo.
pub async fn git_push(
    State(state): State<AppState>,
    Path(app_name): Path<String>,
    Json(payload): Json<GitPushPayload>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    // Only deploy pushes to main
    if payload.git_ref != "refs/heads/main" {
        return Ok(ApiResponse::ok(serde_json::json!({
            "message": "skipped — not main branch"
        })));
    }

    // Validate commit SHA before trusting it
    deploy::validate_sha(&payload.commit_sha)?;

    let app = homelab_db::app_repo::get_by_name(&state.db, &app_name).await?;

    let deployment_id = Uuid::new_v4().to_string();
    let deployment = homelab_db::deployment_repo::create(
        &state.db,
        &deployment_id,
        &app.id,
        &payload.commit_sha,
    )
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
        commit_sha: payload.commit_sha,
    };

    tokio::spawn(deploy::execute(ctx));

    Ok(ApiResponse::ok(serde_json::json!({
        "deployment_id": deployment.id,
        "status": "pending"
    })))
}
