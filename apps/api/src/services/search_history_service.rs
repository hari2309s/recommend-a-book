use crate::error::AppError;
use crate::models::{Book, SearchHistory};
use anyhow::Result;
use serde_json::json;
use supabase_rs::SupabaseClient;

pub struct SearchHistoryService {
    client: SupabaseClient,
}

impl SearchHistoryService {
    pub fn new(client: SupabaseClient) -> Self {
        Self { client }
    }

    pub async fn save_search(
        &self,
        user_id: &str,
        query: &str,
        recommendations: &[Book],
    ) -> Result<SearchHistory> {
        let insert_data = json!({
            "user_id": user_id,
            "query": query,
            "recommendations": recommendations
        });

        let response = self
            .client
            .from("search_history")
            .insert(&insert_data)
            .select("*")
            .single()
            .execute()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let search_history: SearchHistory = serde_json::from_value(response.data)
            .map_err(|e| AppError::SerializationError(e.to_string()))?;

        Ok(search_history)
    }

    pub async fn get_search_history(&self, user_id: &str) -> Result<Vec<SearchHistory>> {
        let response = self
            .client
            .from("search_history")
            .select("*")
            .eq("user_id", user_id)
            .order("created_at", Some("desc".to_string()))
            .execute()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let history: Vec<SearchHistory> = serde_json::from_value(response.data)
            .map_err(|e| AppError::SerializationError(e.to_string()))?;

        Ok(history)
    }
}
