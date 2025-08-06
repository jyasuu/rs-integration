use async_graphql::{Error, ErrorExtensions};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Internal server error")]
    #[allow(dead_code)]
    Internal,
}

impl ErrorExtensions for AppError {
    fn extend(&self) -> Error {
        Error::new(format!("{}", self)).extend_with(|_, e| match self {
            AppError::Database(_) => {
                e.set("code", "DATABASE_ERROR");
            }
            AppError::NotFound(_) => {
                e.set("code", "NOT_FOUND");
            }
            AppError::Validation(_) => {
                e.set("code", "VALIDATION_ERROR");
            }
            AppError::Internal => {
                e.set("code", "INTERNAL_ERROR");
            }
        })
    }
}

pub type AppResult<T> = Result<T, AppError>;