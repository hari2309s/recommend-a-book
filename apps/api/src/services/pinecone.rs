use crate::error::{ApiError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error};

#[derive(Clone)]
pub struct Pinecone {
    client: Client,
    api_key: String,
    host: String,
    dimension: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueryMatch {
    pub id: String,
    pub score: f32,
    pub values: Option<Vec<f32>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct QueryResponse {
    pub matches: Option<Vec<QueryMatch>>,
    pub namespace: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QueryRequest {
    pub vector: Vec<f32>,
    #[serde(rename = "topK")]
    pub top_k: u32,
    #[serde(rename = "includeValues", skip_serializing_if = "Option::is_none")]
    pub include_values: Option<bool>,
    #[serde(rename = "includeMetadata", skip_serializing_if = "Option::is_none")]
    pub include_metadata: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

impl Pinecone {
    pub async fn new(api_key: &str, environment: &str, index_name: &str) -> Result<Self> {
        debug!(
            "Initializing Pinecone client with index: '{}', environment: '{}', API key: {}...{}",
            index_name,
            environment,
            &api_key[..5],
            &api_key[api_key.len().saturating_sub(5)..]
        );

        // Create HTTP client with better configuration
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| ApiError::PineconeError(format!("Failed to create HTTP client: {}", e)))?;

        // Construct the host URL using modern Pinecone URL format
        // Try to detect project ID from API key if possible, otherwise use simplified format
        let host = if environment.contains("-") {
            // Modern format: https://index-name.svc.environment.pinecone.io
            format!("https://{}.svc.{}.pinecone.io", index_name, environment)
        } else {
            // Legacy format fallback
            format!(
                "https://{}-project.svc.{}.pinecone.io",
                index_name, environment
            )
        };

        debug!("Pinecone host URL: {}", host);

        // For now, use a default dimension of 512 (Universal Sentence Encoder)
        let dimension = 512;

        Ok(Self {
            client,
            api_key: api_key.to_string(),
            host,
            dimension,
        })
    }

    pub async fn query_metadata(
        &self,
        field: &str,
        value: &str,
        _exact_match: bool,
        top_k: usize,
    ) -> Result<Vec<crate::models::Book>> {
        // Create a metadata filter
        let filter = json!({
            field: {"$eq": value}
        });

        // Create query request with dummy vector and metadata filter
        let query_request = QueryRequest {
            vector: vec![0.0; self.dimension],
            top_k: top_k as u32,
            include_values: Some(false),
            include_metadata: Some(true),
            filter: Some(filter),
            namespace: None,
        };

        self.execute_query(query_request).await
    }

    pub async fn query_vector(
        &self,
        embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<crate::models::Book>> {
        // Create query request
        let query_request = QueryRequest {
            vector: embedding.to_vec(),
            top_k: top_k as u32,
            include_values: Some(false),
            include_metadata: Some(true),
            filter: None,
            namespace: None,
        };

        self.execute_query(query_request).await
    }

    async fn execute_query(&self, request: QueryRequest) -> Result<Vec<crate::models::Book>> {
        let url = format!("{}/query", self.host);

        debug!(
            "Making Pinecone query to: {} with top_k: {}",
            url, request.top_k
        );

        // Retry logic with exponential backoff
        let mut attempts = 0;
        const MAX_RETRIES: u32 = 3;

        loop {
            attempts += 1;

            let response = self
                .client
                .post(&url)
                .header("Api-Key", &self.api_key)
                .header("Content-Type", "application/json")
                .header("X-Pinecone-API-Version", "2025-01")
                .header("User-Agent", "recommend-a-book-rust-api/1.0")
                .json(&request)
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let query_result: QueryResponse = resp.json().await.map_err(|e| {
                        error!("Failed to parse Pinecone response: {}", e);
                        ApiError::PineconeError(format!("Response parsing failed: {}", e))
                    })?;

                    debug!(
                        "Pinecone query returned {} matches",
                        query_result.matches.as_ref().map_or(0, |m| m.len())
                    );

                    return self.process_pinecone_results(query_result);
                }
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();

                    if status.as_u16() >= 500 && attempts < MAX_RETRIES {
                        // Retry on server errors
                        let delay = std::time::Duration::from_millis(100 * 2u64.pow(attempts - 1));
                        debug!(
                            "Retrying Pinecone request in {:?} (attempt {}/{})",
                            delay, attempts, MAX_RETRIES
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }

                    error!("Pinecone API error: {} - {}", status, text);
                    return Err(ApiError::PineconeError(format!(
                        "API returned {}: {}",
                        status, text
                    )));
                }
                Err(e) if attempts < MAX_RETRIES => {
                    // Retry on network errors
                    let delay = std::time::Duration::from_millis(100 * 2u64.pow(attempts - 1));
                    debug!(
                        "Retrying Pinecone request after network error in {:?} (attempt {}/{}): {}",
                        delay, attempts, MAX_RETRIES, e
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                }
                Err(e) => {
                    error!(
                        "Failed to send request to Pinecone after {} attempts: {}",
                        MAX_RETRIES, e
                    );
                    return Err(ApiError::PineconeError(format!("Request failed: {}", e)));
                }
            }
        }
    }

    fn process_pinecone_results(
        &self,
        query_result: QueryResponse,
    ) -> Result<Vec<crate::models::Book>> {
        let mut books = Vec::new();
        let matches = query_result.matches.unwrap_or_default();

        debug!("Processing {} matches from Pinecone", matches.len());

        // Process each match in the query results
        let matches_len = matches.len();
        for (index, match_) in matches.into_iter().enumerate() {
            // Convert the match's metadata to a serde_json::Value
            let mut metadata_value = serde_json::Map::new();

            // Add the match ID
            metadata_value.insert("id".to_string(), serde_json::json!(match_.id));

            // Add the score as rating (ensure it's a valid float)
            let score = match_.score.max(0.0).min(1.0); // Clamp between 0 and 1
            metadata_value.insert("rating".to_string(), serde_json::json!(score));

            // Add other metadata fields if they exist
            if let Some(metadata) = &match_.metadata {
                match metadata {
                    serde_json::Value::Object(ref meta_obj) => {
                        for (key, value) in meta_obj {
                            // Skip internal Pinecone fields
                            if !key.starts_with("_") {
                                metadata_value.insert(key.clone(), value.clone());
                            }
                        }
                    }
                    _ => {
                        debug!(
                            "Unexpected metadata format for match {}: {:?}",
                            match_.id, metadata
                        );
                    }
                }
            }

            // Ensure required fields have defaults if missing
            if !metadata_value.contains_key("title") {
                metadata_value.insert("title".to_string(), serde_json::json!("Unknown Title"));
            }
            if !metadata_value.contains_key("author") {
                metadata_value.insert("author".to_string(), serde_json::json!("Unknown Author"));
            }
            if !metadata_value.contains_key("categories") {
                metadata_value.insert(
                    "categories".to_string(),
                    serde_json::json!(Vec::<String>::new()),
                );
            }

            // Try to deserialize the metadata into a Book
            match serde_json::from_value::<crate::models::Book>(serde_json::Value::Object(
                metadata_value.clone(),
            )) {
                Ok(book) => {
                    debug!(
                        "Successfully processed book: {} by {}",
                        book.title.as_deref().unwrap_or("Unknown"),
                        book.author.as_deref().unwrap_or("Unknown")
                    );
                    books.push(book);
                }
                Err(e) => {
                    error!(
                        "Failed to deserialize book metadata for match {} (ID: {}): {}. Metadata: {:?}",
                        index, match_.id, e, metadata_value
                    );
                    // Continue processing other matches instead of failing completely
                    continue;
                }
            }
        }

        if books.is_empty() && matches_len > 0 {
            return Err(ApiError::PineconeError(
                "No books could be processed from Pinecone results".to_string(),
            ));
        }

        debug!(
            "Successfully processed {}/{} books from Pinecone results",
            books.len(),
            matches_len
        );
        Ok(books)
    }
}
