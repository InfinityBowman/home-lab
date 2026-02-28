use thiserror::Error;

#[derive(Debug, Error)]
pub enum HomelabError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("already exists: {0}")]
    AlreadyExists(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("docker error: {0}")]
    Docker(String),

    #[error("cloudflare error: {0}")]
    Cloudflare(String),

    #[error("database error: {0}")]
    Database(String),

    #[error("internal error: {0}")]
    Internal(String),
}
