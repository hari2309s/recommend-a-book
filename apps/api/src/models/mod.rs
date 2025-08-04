use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub title: String,
    pub author: String,
    pub description: String,
    pub rating: String,
    pub thumbnail: String,
    pub categories: String,
    #[serde(rename = "publishedYear")]
    pub published_year: String,
    #[serde(rename = "ratingsCount")]
    pub ratings_count: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistory {
    pub id: Option<String>,
    pub user_id: String,
    pub query: String,
    pub recommendations: Vec<Book>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PineconeRecord {
    pub id: String,
    pub values: Vec<f32>,
    pub metadata: PineconeMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PineconeMetadata {
    pub title: String,
    pub author: String,
    pub description: String,
    pub rating: String,
    pub thumbnail: String,
    pub categories: String,
    #[serde(rename = "publishedYear")]
    pub published_year: String,
    #[serde(rename = "ratingsCount")]
    pub ratings_count: String,
}

#[derive(Debug, Deserialize)]
pub struct RecommendationRequest {
    pub query: String,
    pub user_id: Option<String>,
    #[serde(rename = "topK")]
    pub top_k: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct RecommendationResponse {
    pub recommendations: Vec<Book>,
    pub user_id: String,
}

#[derive(Debug, Serialize)]
pub struct SearchHistoryResponse {
    pub history: Vec<SearchHistory>,
}
