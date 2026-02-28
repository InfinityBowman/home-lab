use homelab_core::{EnvVar, HomelabError};
use sqlx::SqlitePool;
use std::collections::HashMap;

pub async fn set(
    pool: &SqlitePool,
    app_id: &str,
    key: &str,
    value: &str,
) -> Result<(), HomelabError> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO env_vars (id, app_id, key, value) VALUES (?, ?, ?, ?) \
         ON CONFLICT(app_id, key) DO UPDATE SET value = excluded.value",
    )
    .bind(&id)
    .bind(app_id)
    .bind(key)
    .bind(value)
    .execute(pool)
    .await
    .map_err(|e| HomelabError::Database(e.to_string()))?;

    Ok(())
}

/// Set multiple env vars in a single transaction (atomic upsert).
pub async fn bulk_set(
    pool: &SqlitePool,
    app_id: &str,
    vars: &HashMap<String, String>,
) -> Result<(), HomelabError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?;

    for (key, value) in vars {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO env_vars (id, app_id, key, value) VALUES (?, ?, ?, ?) \
             ON CONFLICT(app_id, key) DO UPDATE SET value = excluded.value",
        )
        .bind(&id)
        .bind(app_id)
        .bind(key)
        .bind(value)
        .execute(&mut *tx)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?;

    Ok(())
}

pub async fn get_by_app(pool: &SqlitePool, app_id: &str) -> Result<Vec<EnvVar>, HomelabError> {
    let rows =
        sqlx::query_as::<_, EnvVarRow>("SELECT * FROM env_vars WHERE app_id = ? ORDER BY key")
            .bind(app_id)
            .fetch_all(pool)
            .await
            .map_err(|e| HomelabError::Database(e.to_string()))?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn delete(pool: &SqlitePool, app_id: &str, key: &str) -> Result<(), HomelabError> {
    let result = sqlx::query("DELETE FROM env_vars WHERE app_id = ? AND key = ?")
        .bind(app_id)
        .bind(key)
        .execute(pool)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(HomelabError::NotFound(format!(
            "env var '{key}' not found for app"
        )));
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct EnvVarRow {
    id: String,
    app_id: String,
    key: String,
    value: String,
    created_at: String,
}

impl From<EnvVarRow> for EnvVar {
    fn from(row: EnvVarRow) -> Self {
        Self {
            id: row.id,
            app_id: row.app_id,
            key: row.key,
            value: row.value,
            created_at: row.created_at,
        }
    }
}
