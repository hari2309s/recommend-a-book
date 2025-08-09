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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_book_serialization() {
        let book = Book {
            id: "123".to_string(),
            title: "Test Book".to_string(),
            author: "Test Author".to_string(),
            description: "A test book".to_string(),
            rating: 4.5,
            thumbnail: "https://example.com/thumbnail.jpg".to_string(),
            categories: vec!["Fiction".to_string(), "Mystery".to_string()],
            published_year: 2023,
            ratings_count: 100,
        };

        let json = serde_json::to_string(&book).unwrap();
        let deserialized: Book = serde_json::from_str(&json).unwrap();

        assert_eq!(book.title, deserialized.title);
        assert_eq!(book.author, deserialized.author);
        assert_eq!(book.published_year, deserialized.published_year);
    }

    #[test]
    fn test_recommendation_request_serialization() {
        let request = RecommendationRequest {
            query: "test query".to_string(),
            top_k: Some(5),
            user_id: Some(Uuid::new_v4()),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: RecommendationRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.query, deserialized.query);
        assert_eq!(request.top_k, deserialized.top_k);
    }

    #[test]
    fn test_search_history_serialization() {
        let history = SearchHistory {
            id: Some(Uuid::new_v4()),
            user_id: Uuid::new_v4(),
            query: "test query".to_string(),
            recommendations: vec![],
            created_at: Some(Utc::now()),
        };

        let json = serde_json::to_string(&history).unwrap();
        let deserialized: SearchHistory = serde_json::from_str(&json).unwrap();

        assert_eq!(history.user_id, deserialized.user_id);
        assert_eq!(history.query, deserialized.query);
    }
}
