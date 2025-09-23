use crate::error::{ApiError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, warn};

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
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct QueryResponse {
    pub matches: Option<Vec<QueryMatch>>,
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

        // Validate that we have actual values and not placeholders
        if api_key.is_empty() || api_key.contains("your") || api_key.len() < 10 {
            return Err(ApiError::PineconeError(
                "Invalid Pinecone API key. Please set a valid APP_PINECONE_API_KEY environment variable.".to_string(),
            ));
        }

        if environment.is_empty()
            || environment.contains("your")
            || environment == "your_environment_name"
        {
            return Err(ApiError::PineconeError(
                "Invalid Pinecone environment. Please set a valid APP_PINECONE_ENVIRONMENT environment variable.".to_string(),
            ));
        }

        if index_name.is_empty() || index_name.contains("your") || index_name == "your_index_name" {
            return Err(ApiError::PineconeError(
                "Invalid Pinecone index name. Please set a valid APP_PINECONE_INDEX environment variable.".to_string(),
            ));
        }

        let host = if environment.contains("-") {
            format!("https://{}.svc.{}.pinecone.io", index_name, environment)
        } else {
            // Legacy format fallback
            format!(
                "https://{}-project.svc.{}.pinecone.io",
                index_name, environment
            )
        };

        debug!("Pinecone host URL: {}", host);

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
        exact_match: bool,
        top_k: usize,
    ) -> Result<Vec<crate::models::Book>> {
        let filter = if exact_match {
            json!({
                field: {"$eq": value}
            })
        } else {
            let variations = vec![
                value.to_lowercase(),
                value.to_uppercase(),
                format!("{}", value),
            ];

            json!({
                field: {"$in": variations}
            })
        };

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

                    // Check for specific errors that might indicate configuration issues
                    if text.contains("unauthorized")
                        || text.contains("forbidden")
                        || text.contains("not found")
                    {
                        return Err(ApiError::PineconeError(format!(
                            "Pinecone authentication failed ({}). Please verify your API key, index name, and environment are correct.",
                            status
                        )));
                    }

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
                    // Provide more helpful error for DNS issues
                    if e.to_string().contains("dns error")
                        || e.to_string().contains("lookup address")
                    {
                        return Err(ApiError::PineconeError(format!(
                            "Connection to Pinecone failed: DNS error. Please verify your APP_PINECONE_ENVIRONMENT and APP_PINECONE_INDEX environment variables. Error details: {}",
                            e
                        )));
                    }

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

        let matches_len = matches.len();
        for (index, match_) in matches.into_iter().enumerate() {
            // Extract metadata from the match
            let metadata = match_.metadata.and_then(|m| {
                if let serde_json::Value::Object(obj) = m {
                    Some(obj)
                } else {
                    None
                }
            });

            if let Some(mut metadata_map) = metadata {
                // Add the ID to metadata for Book deserialization
                metadata_map.insert("id".to_string(), serde_json::json!(match_.id));

                // Try to deserialize the metadata into a Book using serde
                // The Book model already has the correct aliases set up
                match serde_json::from_value::<crate::models::Book>(serde_json::Value::Object(
                    metadata_map.clone(),
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
                        warn!(
                            "Failed to deserialize book metadata for match {} (ID: {}): {}",
                            index, match_.id, e
                        );

                        // Log the problematic metadata for debugging
                        debug!("Problematic metadata: {:?}", metadata_map);

                        // Create minimal fallback book
                        let minimal_book = crate::models::Book {
                            id: Some(match_.id.clone()),
                            title: metadata_map
                                .get("title")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            author: metadata_map
                                .get("author")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            description: metadata_map
                                .get("description")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            categories: metadata_map
                                .get("categories")
                                .and_then(|v| v.as_str())
                                .map(|s| vec![s.to_string()])
                                .unwrap_or_else(|| vec!["Unknown".to_string()]),
                            thumbnail: metadata_map
                                .get("thumbnail")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            rating: metadata_map
                                .get("rating")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(0.0),
                            year: metadata_map
                                .get("publishedYear")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse().ok()),
                            isbn: Some(match_.id.clone()),
                            page_count: metadata_map
                                .get("pageCount")
                                .or_else(|| metadata_map.get("page_count"))
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse().ok()),
                            ratings_count: metadata_map
                                .get("ratingsCount")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse().ok()),
                            language: metadata_map
                                .get("language")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            publisher: metadata_map
                                .get("publisher")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                        };

                        debug!("Created minimal book fallback for ID: {}", match_.id);
                        books.push(minimal_book);
                    }
                }
            } else {
                warn!("Match {} has no metadata", match_.id);

                // Create minimal fallback book
                let minimal_book = crate::models::Book {
                    id: Some(match_.id.clone()),
                    title: Some("Unknown Title".to_string()),
                    author: Some("Unknown Author".to_string()),
                    description: None,
                    categories: vec!["Unknown".to_string()],
                    thumbnail: None,
                    rating: 0.0,
                    year: None,
                    isbn: Some(match_.id.clone()),
                    page_count: None,
                    ratings_count: None,
                    language: None,
                    publisher: None,
                };

                debug!("Created minimal book fallback for ID: {}", match_.id);
                books.push(minimal_book);
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
