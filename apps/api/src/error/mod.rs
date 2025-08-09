use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt;

pub type Result<T> = std::result::Result<T, ApiError>;

#[derive(Debug)]
pub enum ApiError {
    InvalidInput(String),
    NotFound(String),
    DatabaseError(String),
    SerializationError(String),
    ExternalServiceError(String),
    Unauthorized(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            ApiError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ApiError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            ApiError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            ApiError::ExternalServiceError(msg) => write!(f, "External service error: {}", msg),
            ApiError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::SerializationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::ExternalServiceError(_) => StatusCode::BAD_GATEWAY,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        let response = ErrorResponse {
            error: self.to_string(),
        };
        HttpResponse::build(status).json(response)
    }
}

// Implement From for common error types
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => ApiError::NotFound("Resource not found".to_string()),
            _ => ApiError::DatabaseError(err.to_string()),
        }
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
        ApiError::ExternalServiceError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::body::MessageBody;

    #[test]
    fn test_error_response() {
        let error = ApiError::InvalidInput("Test error".to_string());
        let response = error.error_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        if let Ok(bytes) = response.into_body().try_into_bytes() {
            let body: ErrorResponse = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(body.error, "Invalid input: Test error");
        }
    }

    #[test]
    fn test_database_error_conversion() {
        let db_error = sqlx::Error::RowNotFound;
        let api_error: ApiError = db_error.into();

        match api_error {
            ApiError::NotFound(_) => (),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_serialization_error_conversion() {
        let json_error = serde_json::from_str::<String>("{invalid}").unwrap_err();
        let api_error: ApiError = json_error.into();

        match api_error {
            ApiError::SerializationError(_) => (),
            _ => panic!("Expected SerializationError"),
        }
    }
}
