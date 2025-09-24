use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt::{self, Display};
use thiserror::Error;

/// Type alias for the API Result type
pub type Result<T> = std::result::Result<T, ApiError>;

/// Error context provides additional information about an error
#[derive(Debug, Clone, Serialize)]
pub struct ErrorContext {
    /// The component that generated the error
    pub component: String,
    /// Additional details about the error
    pub details: Option<String>,
    /// Source operation that failed
    pub operation: Option<String>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new(component: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            details: None,
            operation: None,
        }
    }

    /// Add details to the error context
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// Add the operation that failed
    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }
}

impl Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "in component '{}'", self.component)?;

        if let Some(op) = &self.operation {
            write!(f, " during operation '{}'", op)?;
        }

        if let Some(details) = &self.details {
            write!(f, ": {}", details)?;
        }

        Ok(())
    }
}

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum ApiError {
    #[error("Resource not found: {message} {context}")]
    NotFound {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Invalid input: {message} {context}")]
    InvalidInput {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Database error: {message} {context}")]
    DatabaseError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("External service error: {message} {context}")]
    ExternalServiceError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Failed to load model: {message} {context}")]
    ModelLoadError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Model inference failed: {message} {context}")]
    ModelInferenceError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Serialization error: {message} {context}")]
    SerializationError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Authentication error: {message} {context}")]
    AuthenticationError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Cache error: {message} {context}")]
    CacheError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Rate limit exceeded: {message} {context}")]
    RateLimitExceeded {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Timeout error: {message} {context}")]
    TimeoutError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Configuration error: {message} {context}")]
    ConfigurationError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Pinecone service error: {message} {context}")]
    PineconeError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Internal server error: {message} {context}")]
    InternalError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    component: Option<String>,
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::NotFound { .. } => StatusCode::NOT_FOUND,
            ApiError::InvalidInput { .. } => StatusCode::BAD_REQUEST,
            ApiError::AuthenticationError { .. } => StatusCode::UNAUTHORIZED,
            ApiError::RateLimitExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,
            ApiError::TimeoutError { .. } => StatusCode::REQUEST_TIMEOUT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let (code, context) = match self {
            ApiError::NotFound { context, .. } => ("NOT_FOUND", context),
            ApiError::InvalidInput { context, .. } => ("INVALID_INPUT", context),
            ApiError::DatabaseError { context, .. } => ("DATABASE_ERROR", context),
            ApiError::ExternalServiceError { context, .. } => ("EXTERNAL_SERVICE_ERROR", context),
            ApiError::ModelLoadError { context, .. } => ("MODEL_LOAD_ERROR", context),
            ApiError::ModelInferenceError { context, .. } => ("MODEL_INFERENCE_ERROR", context),
            ApiError::SerializationError { context, .. } => ("SERIALIZATION_ERROR", context),
            ApiError::AuthenticationError { context, .. } => ("AUTHENTICATION_ERROR", context),
            ApiError::CacheError { context, .. } => ("CACHE_ERROR", context),
            ApiError::RateLimitExceeded { context, .. } => ("RATE_LIMIT_EXCEEDED", context),
            ApiError::TimeoutError { context, .. } => ("TIMEOUT_ERROR", context),
            ApiError::ConfigurationError { context, .. } => ("CONFIGURATION_ERROR", context),
            ApiError::PineconeError { context, .. } => ("PINECONE_ERROR", context),
            ApiError::InternalError { context, .. } => ("INTERNAL_ERROR", context),
        };

        let error_response = ErrorResponse {
            error: self.to_string(),
            code: code.to_string(),
            details: context.details.clone(),
            component: Some(context.component.clone()),
        };

        HttpResponse::build(self.status_code()).json(error_response)
    }
}

// Helper function to convert old-style error constructors to the new structured format
#[allow(dead_code)]
impl ApiError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
            source: None,
            context: ErrorContext::new("api"),
        }
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: message.into(),
            source: None,
            context: ErrorContext::new("api"),
        }
    }

    pub fn database_error(message: impl Into<String>) -> Self {
        Self::DatabaseError {
            message: message.into(),
            source: None,
            context: ErrorContext::new("database"),
        }
    }

    pub fn external_service_error(message: impl Into<String>) -> Self {
        Self::ExternalServiceError {
            message: message.into(),
            source: None,
            context: ErrorContext::new("external"),
        }
    }

    pub fn model_load_error(message: impl Into<String>) -> Self {
        Self::ModelLoadError {
            message: message.into(),
            source: None,
            context: ErrorContext::new("ml"),
        }
    }

    pub fn model_inference_error(message: impl Into<String>) -> Self {
        Self::ModelInferenceError {
            message: message.into(),
            source: None,
            context: ErrorContext::new("ml"),
        }
    }

    pub fn serialization_error(message: impl Into<String>) -> Self {
        Self::SerializationError {
            message: message.into(),
            source: None,
            context: ErrorContext::new("serialization"),
        }
    }

    pub fn authentication_error(message: impl Into<String>) -> Self {
        Self::AuthenticationError {
            message: message.into(),
            source: None,
            context: ErrorContext::new("auth"),
        }
    }

    pub fn cache_error(message: impl Into<String>) -> Self {
        Self::CacheError {
            message: message.into(),
            source: None,
            context: ErrorContext::new("cache"),
        }
    }

    pub fn pinecone_error(message: impl Into<String>) -> Self {
        Self::PineconeError {
            message: message.into(),
            source: None,
            context: ErrorContext::new("pinecone"),
        }
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError {
            message: message.into(),
            source: None,
            context: ErrorContext::new("system"),
        }
    }

    pub fn with_context(mut self, component: impl Into<String>) -> Self {
        match &mut self {
            ApiError::NotFound { context, .. } => context.component = component.into(),
            ApiError::InvalidInput { context, .. } => context.component = component.into(),
            ApiError::DatabaseError { context, .. } => context.component = component.into(),
            ApiError::ExternalServiceError { context, .. } => context.component = component.into(),
            ApiError::ModelLoadError { context, .. } => context.component = component.into(),
            ApiError::ModelInferenceError { context, .. } => context.component = component.into(),
            ApiError::SerializationError { context, .. } => context.component = component.into(),
            ApiError::AuthenticationError { context, .. } => context.component = component.into(),
            ApiError::CacheError { context, .. } => context.component = component.into(),
            ApiError::RateLimitExceeded { context, .. } => context.component = component.into(),
            ApiError::TimeoutError { context, .. } => context.component = component.into(),
            ApiError::ConfigurationError { context, .. } => context.component = component.into(),
            ApiError::PineconeError { context, .. } => context.component = component.into(),
            ApiError::InternalError { context, .. } => context.component = component.into(),
        }
        self
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        match &mut self {
            ApiError::NotFound { context, .. } => context.details = Some(details.into()),
            ApiError::InvalidInput { context, .. } => context.details = Some(details.into()),
            ApiError::DatabaseError { context, .. } => context.details = Some(details.into()),
            ApiError::ExternalServiceError { context, .. } => {
                context.details = Some(details.into())
            }
            ApiError::ModelLoadError { context, .. } => context.details = Some(details.into()),
            ApiError::ModelInferenceError { context, .. } => context.details = Some(details.into()),
            ApiError::SerializationError { context, .. } => context.details = Some(details.into()),
            ApiError::AuthenticationError { context, .. } => context.details = Some(details.into()),
            ApiError::CacheError { context, .. } => context.details = Some(details.into()),
            ApiError::RateLimitExceeded { context, .. } => context.details = Some(details.into()),
            ApiError::TimeoutError { context, .. } => context.details = Some(details.into()),
            ApiError::ConfigurationError { context, .. } => context.details = Some(details.into()),
            ApiError::PineconeError { context, .. } => context.details = Some(details.into()),
            ApiError::InternalError { context, .. } => context.details = Some(details.into()),
        }
        self
    }

    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        match &mut self {
            ApiError::NotFound { context, .. } => context.operation = Some(operation.into()),
            ApiError::InvalidInput { context, .. } => context.operation = Some(operation.into()),
            ApiError::DatabaseError { context, .. } => context.operation = Some(operation.into()),
            ApiError::ExternalServiceError { context, .. } => {
                context.operation = Some(operation.into())
            }
            ApiError::ModelLoadError { context, .. } => context.operation = Some(operation.into()),
            ApiError::ModelInferenceError { context, .. } => {
                context.operation = Some(operation.into())
            }
            ApiError::SerializationError { context, .. } => {
                context.operation = Some(operation.into())
            }
            ApiError::AuthenticationError { context, .. } => {
                context.operation = Some(operation.into())
            }
            ApiError::CacheError { context, .. } => context.operation = Some(operation.into()),
            ApiError::RateLimitExceeded { context, .. } => {
                context.operation = Some(operation.into())
            }
            ApiError::TimeoutError { context, .. } => context.operation = Some(operation.into()),
            ApiError::ConfigurationError { context, .. } => {
                context.operation = Some(operation.into())
            }
            ApiError::PineconeError { context, .. } => context.operation = Some(operation.into()),
            ApiError::InternalError { context, .. } => context.operation = Some(operation.into()),
        }
        self
    }

    pub fn with_source<E>(mut self, err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        match &mut self {
            ApiError::NotFound { source, .. } => *source = Some(Box::new(err)),
            ApiError::InvalidInput { source, .. } => *source = Some(Box::new(err)),
            ApiError::DatabaseError { source, .. } => *source = Some(Box::new(err)),
            ApiError::ExternalServiceError { source, .. } => *source = Some(Box::new(err)),
            ApiError::ModelLoadError { source, .. } => *source = Some(Box::new(err)),
            ApiError::ModelInferenceError { source, .. } => *source = Some(Box::new(err)),
            ApiError::SerializationError { source, .. } => *source = Some(Box::new(err)),
            ApiError::AuthenticationError { source, .. } => *source = Some(Box::new(err)),
            ApiError::CacheError { source, .. } => *source = Some(Box::new(err)),
            ApiError::RateLimitExceeded { source, .. } => *source = Some(Box::new(err)),
            ApiError::TimeoutError { source, .. } => *source = Some(Box::new(err)),
            ApiError::ConfigurationError { source, .. } => *source = Some(Box::new(err)),
            ApiError::PineconeError { source, .. } => *source = Some(Box::new(err)),
            ApiError::InternalError { source, .. } => *source = Some(Box::new(err)),
        }
        self
    }
}

// From implementations for common error types
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        ApiError::database_error(err.to_string()).with_source(err)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::serialization_error(err.to_string()).with_source(err)
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        // Check if it's a timeout before consuming the error
        if err.is_timeout() {
            return ApiError::TimeoutError {
                message: format!("Request timed out: {}", err),
                source: Some(Box::new(err)),
                context: ErrorContext::new("http_client"),
            };
        }

        // For other types of errors
        ApiError::external_service_error(err.to_string()).with_source(err)
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        ApiError::internal_error(err.to_string()).with_source(err)
    }
}

impl From<ndarray::ShapeError> for ApiError {
    fn from(err: ndarray::ShapeError) -> Self {
        ApiError::model_inference_error(err.to_string()).with_source(err)
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::InternalError {
            message: err.to_string(),
            source: None,
            context: ErrorContext::new("system"),
        }
    }
}

// Additional From implementations for new error types in the codebase
impl<E> From<crate::services::utils::retry::RetryError<E>> for ApiError
where
    E: std::fmt::Display + std::error::Error + Send + Sync + 'static,
{
    fn from(err: crate::services::utils::retry::RetryError<E>) -> Self {
        match err {
            crate::services::utils::retry::RetryError::MaxAttemptsReached { attempts, source } => {
                ApiError::ExternalServiceError {
                    message: format!("Operation failed after {} attempts", attempts),
                    source: Some(Box::new(source)),
                    context: ErrorContext::new("retry")
                        .with_operation("max_attempts_reached")
                        .with_details(format!("Max retries: {}", attempts)),
                }
            }
            crate::services::utils::retry::RetryError::OperationFailed { attempts, source } => {
                ApiError::ExternalServiceError {
                    message: format!("Operation failed: {}", source),
                    source: Some(Box::new(source)),
                    context: ErrorContext::new("retry")
                        .with_operation("operation_failed")
                        .with_details(format!("Attempts: {}", attempts)),
                }
            }
        }
    }
}

impl From<String> for ApiError {
    fn from(err: String) -> Self {
        ApiError::internal_error(err)
    }
}

impl From<&str> for ApiError {
    fn from(err: &str) -> Self {
        ApiError::internal_error(err)
    }
}
