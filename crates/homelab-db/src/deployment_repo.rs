use homelab_core::{DeployStatus, Deployment, HomelabError};
use sqlx::SqlitePool;

pub async fn create(
    pool: &SqlitePool,
    id: &str,
    app_id: &str,
    commit_sha: &str,
) -> Result<Deployment, HomelabError> {
    sqlx::query("INSERT INTO deployments (id, app_id, commit_sha) VALUES (?, ?, ?)")
        .bind(id)
        .bind(app_id)
        .bind(commit_sha)
        .execute(pool)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?;

    get_by_id(pool, id).await
}

pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Deployment, HomelabError> {
    let row = sqlx::query_as::<_, DeploymentRow>("SELECT * FROM deployments WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?
        .ok_or_else(|| HomelabError::NotFound(format!("deployment not found: {id}")))?;

    Ok(row.into())
}

pub async fn list_by_app(pool: &SqlitePool, app_id: &str) -> Result<Vec<Deployment>, HomelabError> {
    let rows = sqlx::query_as::<_, DeploymentRow>(
        "SELECT * FROM deployments WHERE app_id = ? ORDER BY started_at DESC",
    )
    .bind(app_id)
    .fetch_all(pool)
    .await
    .map_err(|e| HomelabError::Database(e.to_string()))?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn update_status(
    pool: &SqlitePool,
    id: &str,
    status: &DeployStatus,
    image_tag: Option<&str>,
    build_log: Option<&str>,
) -> Result<(), HomelabError> {
    let finished = matches!(status, DeployStatus::Succeeded | DeployStatus::Failed);

    let result = if finished {
        sqlx::query(
            "UPDATE deployments SET status = ?, image_tag = COALESCE(?, image_tag), \
             build_log = COALESCE(?, build_log), finished_at = datetime('now') WHERE id = ?",
        )
        .bind(status.to_string())
        .bind(image_tag)
        .bind(build_log)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?
    } else {
        sqlx::query(
            "UPDATE deployments SET status = ?, image_tag = COALESCE(?, image_tag), \
             build_log = COALESCE(?, build_log) WHERE id = ?",
        )
        .bind(status.to_string())
        .bind(image_tag)
        .bind(build_log)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?
    };

    if result.rows_affected() == 0 {
        return Err(HomelabError::NotFound(format!(
            "deployment not found: {id}"
        )));
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct DeploymentRow {
    id: String,
    app_id: String,
    commit_sha: String,
    image_tag: String,
    status: String,
    build_log: Option<String>,
    started_at: String,
    finished_at: Option<String>,
}

impl From<DeploymentRow> for Deployment {
    fn from(row: DeploymentRow) -> Self {
        Self {
            id: row.id,
            app_id: row.app_id,
            commit_sha: row.commit_sha,
            image_tag: row.image_tag,
            status: row.status.parse().unwrap_or(DeployStatus::Pending),
            build_log: row.build_log,
            started_at: row.started_at,
            finished_at: row.finished_at,
        }
    }
}
