use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub id: String,
    pub title: String,
    pub author: String,
    pub description: String,
    pub rating: f32,
    pub thumbnail: String,
    pub categories: Vec<String>,
    #[serde(rename = "publishedYear")]
    pub published_year: i32,
    #[serde(rename = "ratingsCount")]
    pub ratings_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookRecommendation {
    pub id: Uuid,
    pub book: Book,
    pub similarity_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistory {
    pub id: Option<Uuid>,
    pub user_id: Uuid,
    pub query: String,
    pub recommendations: Vec<Book>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationRequest {
    pub query: String,
    pub top_k: Option<usize>,
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryRequest {
    pub user_id: String,
    pub limit: Option<i32>,
}
