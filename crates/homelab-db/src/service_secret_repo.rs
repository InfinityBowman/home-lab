use homelab_core::HomelabError;
use sqlx::SqlitePool;

pub struct EncryptedSecret {
    pub id: String,
    pub service_id: String,
    pub key: String,
    pub encrypted_value: Vec<u8>,
    pub nonce: Vec<u8>,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn set(
    pool: &SqlitePool,
    service_id: &str,
    key: &str,
    encrypted_value: &[u8],
    nonce: &[u8],
) -> Result<(), HomelabError> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO service_secrets (id, service_id, key, encrypted_value, nonce) \
         VALUES (?, ?, ?, ?, ?) \
         ON CONFLICT(service_id, key) DO UPDATE SET \
         encrypted_value = excluded.encrypted_value, \
         nonce = excluded.nonce, \
         updated_at = datetime('now')",
    )
    .bind(&id)
    .bind(service_id)
    .bind(key)
    .bind(encrypted_value)
    .bind(nonce)
    .execute(pool)
    .await
    .map_err(|e| HomelabError::Database(e.to_string()))?;

    Ok(())
}

pub async fn bulk_set(
    pool: &SqlitePool,
    service_id: &str,
    entries: &[(String, Vec<u8>, Vec<u8>)],
) -> Result<(), HomelabError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?;

    for (key, encrypted_value, nonce) in entries {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO service_secrets (id, service_id, key, encrypted_value, nonce) \
             VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(service_id, key) DO UPDATE SET \
             encrypted_value = excluded.encrypted_value, \
             nonce = excluded.nonce, \
             updated_at = datetime('now')",
        )
        .bind(&id)
        .bind(service_id)
        .bind(key)
        .bind(encrypted_value.as_slice())
        .bind(nonce.as_slice())
        .execute(&mut *tx)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?;

    Ok(())
}

pub async fn get_by_service(
    pool: &SqlitePool,
    service_id: &str,
) -> Result<Vec<EncryptedSecret>, HomelabError> {
    let rows = sqlx::query_as::<_, SecretRow>(
        "SELECT * FROM service_secrets WHERE service_id = ? ORDER BY key",
    )
    .bind(service_id)
    .fetch_all(pool)
    .await
    .map_err(|e| HomelabError::Database(e.to_string()))?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn delete(pool: &SqlitePool, service_id: &str, key: &str) -> Result<(), HomelabError> {
    let result = sqlx::query("DELETE FROM service_secrets WHERE service_id = ? AND key = ?")
        .bind(service_id)
        .bind(key)
        .execute(pool)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(HomelabError::NotFound(format!(
            "secret '{key}' not found for service"
        )));
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct SecretRow {
    id: String,
    service_id: String,
    key: String,
    encrypted_value: Vec<u8>,
    nonce: Vec<u8>,
    created_at: String,
    updated_at: String,
}

impl From<SecretRow> for EncryptedSecret {
    fn from(row: SecretRow) -> Self {
        Self {
            id: row.id,
            service_id: row.service_id,
            key: row.key,
            encrypted_value: row.encrypted_value,
            nonce: row.nonce,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
