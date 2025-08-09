use crate::{
    error::ApiError,
    models::{Book, SearchHistory},
    services::supabase::SupabaseClient,
};
use chrono::Utc;
use serde::de::DeserializeOwned;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SearchHistoryService {
    supabase: SupabaseClient,
}

impl SearchHistoryService {
    pub fn new(supabase: SupabaseClient) -> Self {
        Self { supabase }
    }

    pub async fn get_search_history(
        &self,
        user_id: Uuid,
        limit: Option<i32>,
    ) -> Result<Vec<SearchHistory>, ApiError> {
        self.supabase
            .select_by_user_id::<SearchHistory>("search_history", &user_id.to_string(), limit)
            .await
    }

    pub async fn save_search(&self, history: &SearchHistory) -> Result<(), ApiError> {
        let data = serde_json::json!({
            "id": history.id.unwrap_or_else(Uuid::new_v4).to_string(),
            "user_id": history.user_id.to_string(),
            "query": history.query,
            "recommendations": history.recommendations,
            "created_at": history.created_at.unwrap_or_else(Utc::now).to_rfc3339(),
        });

        self.supabase
            .insert("search_history", &data)
            .await
            .map_err(|e| ApiError::DatabaseError(format!("Failed to save search history: {}", e)))
    }

    #[cfg(test)]
    pub fn mock() -> Self {
        Self {
            supabase: SupabaseClient::mock(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_save_search() {
        let service = SearchHistoryService::mock();
        let history = SearchHistory {
            id: None,
            user_id: Uuid::new_v4(),
            query: "test query".to_string(),
            recommendations: vec![],
            created_at: None,
        };

        let result = service.save_search(&history).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_search_history() {
        let service = SearchHistoryService::mock();
        let user_id = Uuid::new_v4();
        let history = service.get_search_history(user_id, Some(10)).await;
        assert!(history.is_ok());
    }
}
