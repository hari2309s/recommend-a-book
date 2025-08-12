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

#[derive(Debug, Deserialize)]
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

        // Create HTTP client
        let client = Client::new();

        // Construct the host URL for the index
        let host = format!(
            "https://{}-{}.svc.{}.pinecone.io",
            index_name, "project-id", environment
        );
        debug!("Pinecone host: {}", host);

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

        debug!("Making Pinecone query to: {}", url);

        let response = self
            .client
            .post(&url)
            .header("Api-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send request to Pinecone: {}", e);
                ApiError::PineconeError(format!("Request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Pinecone API error: {} - {}", status, text);
            return Err(ApiError::PineconeError(format!(
                "API returned {}: {}",
                status, text
            )));
        }

        let query_result: QueryResponse = response.json().await.map_err(|e| {
            error!("Failed to parse Pinecone response: {}", e);
            ApiError::PineconeError(format!("Response parsing failed: {}", e))
        })?;

        debug!(
            "Pinecone query returned {} matches",
            query_result.matches.as_ref().map_or(0, |m| m.len())
        );

        self.process_pinecone_results(query_result)
    }

    fn process_pinecone_results(
        &self,
        query_result: QueryResponse,
    ) -> Result<Vec<crate::models::Book>> {
        let mut books = Vec::new();

        // Process each match in the query results
        for match_ in query_result.matches.unwrap_or_default() {
            // Convert the match's metadata to a serde_json::Value
            let mut metadata_value = serde_json::Map::new();

            // Add the match ID
            metadata_value.insert("id".to_string(), serde_json::json!(match_.id));

            // Add the score as rating
            metadata_value.insert("rating".to_string(), serde_json::json!(match_.score));

            // Add other metadata fields if they exist
            if let Some(metadata) = &match_.metadata {
                if let serde_json::Value::Object(ref meta_obj) = metadata {
                    for (key, value) in meta_obj {
                        metadata_value.insert(key.clone(), value.clone());
                    }
                }
            }

            // Try to deserialize the metadata into a Book
            match serde_json::from_value(serde_json::Value::Object(metadata_value)) {
                Ok(book) => books.push(book),
                Err(e) => {
                    error!(
                        "Failed to deserialize book metadata for ID {}: {}",
                        match_.id, e
                    );
                    continue;
                }
            }
        }

        debug!(
            "Successfully processed {} books from Pinecone results",
            books.len()
        );
        Ok(books)
    }
}
