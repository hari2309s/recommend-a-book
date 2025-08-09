use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ApiError>;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("External service error: {0}")]
    ExternalServiceError(String),

    #[error("Failed to load model: {0}")]
    ModelLoadError(String),

    #[error("Model inference failed: {0}")]
    ModelInferenceError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    #[error("Internal server error: {0}")]
    InternalError(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let error = ErrorResponse {
            error: self.to_string(),
        };

        match self {
            ApiError::InvalidInput(_) => HttpResponse::BadRequest().json(error),
            ApiError::AuthenticationError(_) => HttpResponse::Unauthorized().json(error),
            ApiError::NotFound(_) => HttpResponse::NotFound().json(error),
            _ => HttpResponse::InternalServerError().json(error),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        ApiError::DatabaseError(err.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::SerializationError(err.to_string())
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        ApiError::ExternalServiceError(err.to_string())
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        ApiError::InternalError(err.to_string())
    }
}

impl From<ndarray::ShapeError> for ApiError {
    fn from(err: ndarray::ShapeError) -> Self {
        ApiError::ModelInferenceError(err.to_string())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::InternalError(err.to_string())
    }
}
