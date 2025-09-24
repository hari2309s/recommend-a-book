use crate::error::ApiError;
use reqwest::{Client, StatusCode};
use serde::{de::DeserializeOwned, Serialize};

#[derive(Debug, Clone)]
pub struct SupabaseClient {
    client: Client,
    base_url: String,
    api_key: String,
}

#[allow(dead_code)]
impl SupabaseClient {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
        }
    }

    pub async fn get_book<T: DeserializeOwned>(&self, id: &str) -> Result<T, ApiError> {
        let url = format!("{}/rest/v1/books?id=eq.{}", self.base_url, id);
        let response = self
            .client
            .get(&url)
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                ApiError::database_error(e.to_string())
                    .with_context("supabase")
                    .with_operation("get_book")
            })?;

        match response.status() {
            StatusCode::OK => {
                let mut items: Vec<T> = response.json().await.map_err(|e| {
                    ApiError::serialization_error(e.to_string())
                        .with_context("supabase")
                        .with_operation("deserialize_response")
                })?;
                items.pop().ok_or_else(|| {
                    ApiError::not_found("Book not found")
                        .with_context("supabase")
                        .with_operation("get_book")
                })
            }
            StatusCode::NOT_FOUND => Err(ApiError::not_found("Book not found")
                .with_context("supabase")
                .with_operation("get_book")),
            status => Err(
                ApiError::database_error(format!("Unexpected status code: {}", status))
                    .with_context("supabase")
                    .with_operation("get_book"),
            ),
        }
    }

    pub async fn select_by_user_id<T: DeserializeOwned>(
        &self,
        table: &str,
        user_id: &str,
        limit: Option<i32>,
    ) -> Result<Vec<T>, ApiError> {
        let mut url = format!(
            "{}/rest/v1/{}?user_id=eq.{}&order=created_at.desc",
            self.base_url, table, user_id
        );

        if let Some(limit) = limit {
            url.push_str(&format!("&limit={}", limit));
        }

        let response = self
            .client
            .get(&url)
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                ApiError::database_error(e.to_string())
                    .with_context("supabase")
                    .with_operation("get_books")
            })?;

        match response.status() {
            StatusCode::OK => response.json().await.map_err(|e| {
                ApiError::serialization_error(e.to_string())
                    .with_context("supabase")
                    .with_operation("deserialize_books")
            }),
            status => Err(
                ApiError::database_error(format!("Unexpected status code: {}", status))
                    .with_context("supabase")
                    .with_operation("get_books"),
            ),
        }
    }

    pub async fn insert<T: Serialize>(&self, table: &str, data: &T) -> Result<(), ApiError> {
        let url = format!("{}/rest/v1/{}", self.base_url, table);
        let response = self
            .client
            .post(&url)
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Prefer", "return=minimal")
            .json(data)
            .send()
            .await
            .map_err(|e| {
                ApiError::database_error(e.to_string())
                    .with_context("supabase")
                    .with_operation("insert_book")
            })?;

        match response.status() {
            StatusCode::CREATED => Ok(()),
            status => Err(
                ApiError::database_error(format!("Failed to insert data: {}", status))
                    .with_context("supabase")
                    .with_operation("insert_book"),
            ),
        }
    }
}
