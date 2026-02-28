use homelab_core::{AuditEntry, HomelabError};
use sqlx::SqlitePool;

pub async fn create(
    pool: &SqlitePool,
    app_id: Option<&str>,
    action: &str,
    details: Option<&str>,
) -> Result<(), HomelabError> {
    sqlx::query("INSERT INTO audit_log (app_id, action, details) VALUES (?, ?, ?)")
        .bind(app_id)
        .bind(action)
        .bind(details)
        .execute(pool)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?;

    Ok(())
}

pub async fn list_by_app(
    pool: &SqlitePool,
    app_id: &str,
    limit: i64,
) -> Result<Vec<AuditEntry>, HomelabError> {
    let rows = sqlx::query_as::<_, AuditRow>(
        "SELECT * FROM audit_log WHERE app_id = ? ORDER BY created_at DESC LIMIT ?",
    )
    .bind(app_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| HomelabError::Database(e.to_string()))?;

    Ok(rows.into_iter().map(Into::into).collect())
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: i64,
    app_id: Option<String>,
    action: String,
    details: Option<String>,
    created_at: String,
}

impl From<AuditRow> for AuditEntry {
    fn from(row: AuditRow) -> Self {
        Self {
            id: row.id,
            app_id: row.app_id,
            action: row.action,
            details: row.details,
            created_at: row.created_at,
        }
    }
}
