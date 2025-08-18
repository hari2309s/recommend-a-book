use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-export Book and BookRecommendation from book.rs
pub use book::Book;

// Re-export SearchHistory from search.rs
pub use search::SearchHistory;

mod book;
mod search;

/// Request structure for book recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationRequest {
    /// The search query or description to find book recommendations
    pub query: String,
    /// Optional number of recommendations to return
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// Optional user ID for tracking search history
    pub user_id: Option<Uuid>,
}

fn default_top_k() -> usize {
    51
}

/// Request structure for fetching search history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryRequest {
    /// User ID to fetch history for
    pub user_id: String,
    /// Optional limit on number of history items to return
    #[serde(default = "default_history_limit")]
    pub limit: i32,
}

fn default_history_limit() -> i32 {
    20
}
