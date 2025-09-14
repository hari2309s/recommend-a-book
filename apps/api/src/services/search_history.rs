use crate::{error::ApiError, models::SearchHistory, services::supabase::SupabaseClient};
use chrono::Utc;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SearchHistoryService {
    supabase: SupabaseClient,
}

#[allow(dead_code)]
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
            "user_id": history.user_id.unwrap_or_else(Uuid::new_v4).to_string(),
            "query": history.query,
            "recommendations": history.recommendations,
            "created_at": history.created_at.unwrap_or_else(Utc::now).to_rfc3339(),
        });

        self.supabase
            .insert("search_history", &data)
            .await
            .map_err(|e| ApiError::DatabaseError(format!("Failed to save search history: {}", e)))
    }
}
