use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// Re-export types from book.rs
pub use book::Book;

mod book;

/// Request structure for book recommendations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RecommendationRequest {
    /// The search query or description to find book recommendations
    #[schema(example = "fantasy books with dragons and magic")]
    pub query: String,
    /// Optional number of recommendations to return (default: 100)
    #[serde(default = "default_top_k")]
    #[schema(example = 50, minimum = 1, maximum = 200)]
    pub top_k: usize,
}

/// Response structure for book recommendations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RecommendationResponse {
    /// List of recommended books
    pub recommendations: Vec<Book>,
    /// Semantic tags extracted from the query
    #[schema(example = json!(["Fantasy", "Magic", "Adventure"]))]
    pub semantic_tags: Vec<String>,
}

/// Health check response structure
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Status of the service
    #[schema(example = "ok")]
    pub status: String,
    /// Current timestamp in RFC3339 format
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub timestamp: String,
}

/// Error response structure
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    /// Error message
    #[schema(example = "Query cannot be empty")]
    pub error: String,
    /// HTTP status code
    #[schema(example = 400)]
    pub status: u16,
}

fn default_top_k() -> usize {
    100
}
