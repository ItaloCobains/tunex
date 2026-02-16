use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("[SQLX] error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("[SQLX] migration error: {0}")]
    Migration(String),

    #[error("[SQLX] query builder error: {0}")]
    Builder(String),
}

pub type Result<T> = std::result::Result<T, Error>;
