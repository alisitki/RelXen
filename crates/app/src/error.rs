use anyhow::Error as AnyhowError;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("history failed: {0}")]
    History(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("secure store unavailable: {0}")]
    SecureStoreUnavailable(String),
    #[error("live operation failed: {0}")]
    Live(String),
    #[error("exchange operation failed: {0}")]
    Exchange(String),
    #[error(transparent)]
    Other(#[from] AnyhowError),
}

impl From<String> for AppError {
    fn from(value: String) -> Self {
        Self::Validation(value)
    }
}
