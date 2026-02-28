use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use std::collections::HashMap;

use crate::error::{ApiError, ApiResponse};
use crate::state::AppState;

#[derive(Serialize)]
pub struct MaskedEnvVar {
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

/// List env vars with masked values.
pub async fn list(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<Vec<MaskedEnvVar>>>, ApiError> {
    let app = homelab_db::app_repo::get_by_name(&state.db, &name).await?;
    let vars = homelab_db::env_var_repo::get_by_app(&state.db, &app.id).await?;

    let masked: Vec<MaskedEnvVar> = vars
        .into_iter()
        .map(|v| MaskedEnvVar {
            key: v.key,
            value: mask_value(&v.value),
        })
        .collect();

    Ok(ApiResponse::ok(masked))
}

/// Bulk set env vars (upsert). Expects a JSON object of key-value pairs.
pub async fn bulk_set(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(vars): Json<HashMap<String, String>>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let app = homelab_db::app_repo::get_by_name(&state.db, &name).await?;

    homelab_db::env_var_repo::bulk_set(&state.db, &app.id, &vars).await?;

    homelab_db::audit_repo::create(
        &state.db,
        Some(&app.id),
        "env.updated",
        Some(&format!(
            "keys={}",
            vars.keys().cloned().collect::<Vec<_>>().join(",")
        )),
    )
    .await?;

    Ok(ApiResponse::ok_empty())
}

/// Delete a single env var by key.
pub async fn delete(
    State(state): State<AppState>,
    Path((name, key)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let app = homelab_db::app_repo::get_by_name(&state.db, &name).await?;
    homelab_db::env_var_repo::delete(&state.db, &app.id, &key).await?;

    homelab_db::audit_repo::create(
        &state.db,
        Some(&app.id),
        "env.deleted",
        Some(&format!("key={key}")),
    )
    .await?;

    Ok(ApiResponse::ok_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_short_value() {
        assert_eq!(mask_value("ab"), "**");
        assert_eq!(mask_value("abcd"), "****");
    }

    #[test]
    fn mask_long_value_shows_suffix() {
        assert_eq!(mask_value("supersecretkey"), "*****tkey");
        assert_eq!(mask_value("12345"), "*****2345");
    }

    #[test]
    fn mask_empty_value() {
        assert_eq!(mask_value(""), "");
    }
}
