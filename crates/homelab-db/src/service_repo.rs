use homelab_core::{HomelabError, Service};
use sqlx::SqlitePool;

pub async fn create(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    compose_path: &str,
) -> Result<Service, HomelabError> {
    sqlx::query("INSERT INTO services (id, name, compose_path) VALUES (?, ?, ?)")
        .bind(id)
        .bind(name)
        .bind(compose_path)
        .execute(pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                HomelabError::AlreadyExists(format!("service '{name}' already exists"))
            } else {
                HomelabError::Database(e.to_string())
            }
        })?;

    get_by_name(pool, name).await
}

pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Service, HomelabError> {
    let row = sqlx::query_as::<_, ServiceRow>("SELECT * FROM services WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?
        .ok_or_else(|| HomelabError::NotFound(format!("service not found: {id}")))?;

    Ok(row.into())
}

pub async fn get_by_name(pool: &SqlitePool, name: &str) -> Result<Service, HomelabError> {
    let row = sqlx::query_as::<_, ServiceRow>("SELECT * FROM services WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?
        .ok_or_else(|| HomelabError::NotFound(format!("service not found: {name}")))?;

    Ok(row.into())
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Service>, HomelabError> {
    let rows = sqlx::query_as::<_, ServiceRow>("SELECT * FROM services ORDER BY name")
        .fetch_all(pool)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), HomelabError> {
    let result = sqlx::query("DELETE FROM services WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(HomelabError::NotFound(format!("service not found: {id}")));
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct ServiceRow {
    id: String,
    name: String,
    compose_path: String,
    created_at: String,
}

impl From<ServiceRow> for Service {
    fn from(row: ServiceRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            compose_path: row.compose_path,
            created_at: row.created_at,
        }
    }
}
