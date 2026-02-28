use homelab_core::{App, AppStatus, HomelabError};
use sqlx::SqlitePool;

pub async fn create(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    domain: &str,
    git_repo_path: &str,
    port: i64,
) -> Result<App, HomelabError> {
    sqlx::query("INSERT INTO apps (id, name, domain, git_repo_path, port) VALUES (?, ?, ?, ?, ?)")
        .bind(id)
        .bind(name)
        .bind(domain)
        .bind(git_repo_path)
        .bind(port)
        .execute(pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                HomelabError::AlreadyExists(format!("app '{name}' already exists"))
            } else {
                HomelabError::Database(e.to_string())
            }
        })?;

    get_by_id(pool, id).await
}

pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<App, HomelabError> {
    let row = sqlx::query_as::<_, AppRow>("SELECT * FROM apps WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?
        .ok_or_else(|| HomelabError::NotFound(format!("app not found: {id}")))?;

    Ok(row.into())
}

pub async fn get_by_name(pool: &SqlitePool, name: &str) -> Result<App, HomelabError> {
    let row = sqlx::query_as::<_, AppRow>("SELECT * FROM apps WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?
        .ok_or_else(|| HomelabError::NotFound(format!("app not found: {name}")))?;

    Ok(row.into())
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<App>, HomelabError> {
    let rows = sqlx::query_as::<_, AppRow>("SELECT * FROM apps ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn update_status(
    pool: &SqlitePool,
    id: &str,
    status: &AppStatus,
) -> Result<(), HomelabError> {
    let result =
        sqlx::query("UPDATE apps SET status = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(status.to_string())
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| HomelabError::Database(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(HomelabError::NotFound(format!("app not found: {id}")));
    }
    Ok(())
}

pub async fn update_image(
    pool: &SqlitePool,
    id: &str,
    docker_image: &str,
) -> Result<(), HomelabError> {
    let result =
        sqlx::query("UPDATE apps SET docker_image = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(docker_image)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| HomelabError::Database(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(HomelabError::NotFound(format!("app not found: {id}")));
    }
    Ok(())
}

pub async fn update(
    pool: &SqlitePool,
    id: &str,
    port: Option<i64>,
    domain: Option<&str>,
) -> Result<App, HomelabError> {
    if port.is_none() && domain.is_none() {
        return Err(HomelabError::InvalidInput(
            "at least one field (port or domain) must be provided".into(),
        ));
    }

    let result = sqlx::query(
        "UPDATE apps SET port = COALESCE(?, port), domain = COALESCE(?, domain), \
         updated_at = datetime('now') WHERE id = ?",
    )
    .bind(port)
    .bind(domain)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| HomelabError::Database(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(HomelabError::NotFound(format!("app not found: {id}")));
    }
    get_by_id(pool, id).await
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), HomelabError> {
    let result = sqlx::query("DELETE FROM apps WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(HomelabError::NotFound(format!("app not found: {id}")));
    }
    Ok(())
}

// Internal row type for sqlx
#[derive(sqlx::FromRow)]
struct AppRow {
    id: String,
    name: String,
    domain: String,
    git_repo_path: String,
    docker_image: String,
    port: i64,
    status: String,
    created_at: String,
    updated_at: String,
}

impl From<AppRow> for App {
    fn from(row: AppRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            domain: row.domain,
            git_repo_path: row.git_repo_path,
            docker_image: row.docker_image,
            port: row.port,
            status: row.status.parse().unwrap_or(AppStatus::Created),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
