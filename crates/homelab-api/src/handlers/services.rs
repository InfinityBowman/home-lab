use axum::Json;
use axum::extract::{Path, State};
use homelab_core::{HomelabError, Service};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::{ApiError, ApiResponse};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateServiceRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct MaskedSecret {
    pub key: String,
    pub value: String,
}

fn mask_value(v: &str) -> String {
    if v.len() <= 4 {
        "*".repeat(v.len())
    } else {
        format!("*****{}", &v[v.len() - 4..])
    }
}

fn require_cipher(state: &AppState) -> Result<&homelab_core::SecretsCipher, ApiError> {
    state
        .cipher
        .as_ref()
        .ok_or_else(|| HomelabError::Internal("secrets encryption not configured".into()).into())
}

// ─── Service CRUD ───────────────────────────────────────────────────────────

pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<Service>>>, ApiError> {
    let services = homelab_db::service_repo::list(&state.db).await?;
    Ok(ApiResponse::ok(services))
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateServiceRequest>,
) -> Result<Json<ApiResponse<Service>>, ApiError> {
    let compose_path = PathBuf::from(&state.config.services_path).join(&req.name);
    let id = uuid::Uuid::new_v4().to_string();
    let service = homelab_db::service_repo::create(
        &state.db,
        &id,
        &req.name,
        &compose_path.to_string_lossy(),
    )
    .await?;

    homelab_db::audit_repo::create(
        &state.db,
        None,
        "service.created",
        Some(&format!("name={}", req.name)),
    )
    .await?;

    Ok(ApiResponse::ok(service))
}

pub async fn get(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<Service>>, ApiError> {
    let service = homelab_db::service_repo::get_by_name(&state.db, &name).await?;
    Ok(ApiResponse::ok(service))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let service = homelab_db::service_repo::get_by_name(&state.db, &name).await?;
    homelab_db::service_repo::delete(&state.db, &service.id).await?;

    homelab_db::audit_repo::create(
        &state.db,
        None,
        "service.deleted",
        Some(&format!("name={name}")),
    )
    .await?;

    Ok(ApiResponse::ok_empty())
}

// ─── Service Secrets ────────────────────────────────────────────────────────

pub async fn list_secrets(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<Vec<MaskedSecret>>>, ApiError> {
    let cipher = require_cipher(&state)?;
    let service = homelab_db::service_repo::get_by_name(&state.db, &name).await?;
    let rows = homelab_db::service_secret_repo::get_by_service(&state.db, &service.id).await?;

    let masked: Vec<MaskedSecret> = rows
        .into_iter()
        .map(|row| {
            let decrypted = cipher
                .decrypt(&row.encrypted_value, &row.nonce)
                .unwrap_or_else(|_| "[decryption error]".into());
            MaskedSecret {
                key: row.key,
                value: mask_value(&decrypted),
            }
        })
        .collect();

    Ok(ApiResponse::ok(masked))
}

pub async fn bulk_set_secrets(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(vars): Json<HashMap<String, String>>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let cipher = require_cipher(&state)?;
    let service = homelab_db::service_repo::get_by_name(&state.db, &name).await?;

    let mut entries = Vec::with_capacity(vars.len());
    for (key, value) in &vars {
        let (encrypted, nonce) = cipher.encrypt(value)?;
        entries.push((key.clone(), encrypted, nonce));
    }

    homelab_db::service_secret_repo::bulk_set(&state.db, &service.id, &entries).await?;

    // Write .env file and restart the service
    sync_and_restart(&state, &service).await?;

    homelab_db::audit_repo::create(
        &state.db,
        None,
        "service.secrets.updated",
        Some(&format!(
            "service={}, keys={}",
            name,
            vars.keys().cloned().collect::<Vec<_>>().join(",")
        )),
    )
    .await?;

    Ok(ApiResponse::ok_empty())
}

pub async fn delete_secret(
    State(state): State<AppState>,
    Path((name, key)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    require_cipher(&state)?;
    let service = homelab_db::service_repo::get_by_name(&state.db, &name).await?;
    homelab_db::service_secret_repo::delete(&state.db, &service.id, &key).await?;

    sync_and_restart(&state, &service).await?;

    homelab_db::audit_repo::create(
        &state.db,
        None,
        "service.secrets.deleted",
        Some(&format!("service={name}, key={key}")),
    )
    .await?;

    Ok(ApiResponse::ok_empty())
}

#[derive(Serialize)]
pub struct RevealedSecret {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize)]
pub struct RevealRequest {
    pub key: String,
}

pub async fn reveal_secret(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<RevealRequest>,
) -> Result<Json<ApiResponse<RevealedSecret>>, ApiError> {
    let cipher = require_cipher(&state)?;
    let service = homelab_db::service_repo::get_by_name(&state.db, &name).await?;
    let rows = homelab_db::service_secret_repo::get_by_service(&state.db, &service.id).await?;

    let row = rows
        .into_iter()
        .find(|r| r.key == req.key)
        .ok_or_else(|| HomelabError::NotFound(format!("secret '{}' not found", req.key)))?;

    let value = cipher.decrypt(&row.encrypted_value, &row.nonce)?;

    homelab_db::audit_repo::create(
        &state.db,
        None,
        "service.secrets.revealed",
        Some(&format!("service={name}, key={}", req.key)),
    )
    .await?;

    Ok(ApiResponse::ok(RevealedSecret {
        key: row.key,
        value,
    }))
}

// ─── Helpers ────────────────────────────────────────────────────────────────

async fn sync_and_restart(state: &AppState, service: &Service) -> Result<(), HomelabError> {
    let cipher = state
        .cipher
        .as_ref()
        .ok_or_else(|| HomelabError::Internal("cipher not configured".into()))?;

    let rows = homelab_db::service_secret_repo::get_by_service(&state.db, &service.id).await?;

    let vars: Vec<(String, String)> = rows
        .into_iter()
        .map(|row| {
            let value = cipher.decrypt(&row.encrypted_value, &row.nonce)?;
            Ok((row.key, value))
        })
        .collect::<Result<_, HomelabError>>()?;

    let compose_dir = PathBuf::from(&service.compose_path);

    homelab_docker::compose::write_env_file(&compose_dir, &vars).await?;
    homelab_docker::compose::restart_compose(&compose_dir).await?;

    Ok(())
}
