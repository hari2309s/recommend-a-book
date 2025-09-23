use crate::error::{ApiError, Result};
use log::{debug, error, info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

// Cache entry for Pinecone results to improve performance
#[derive(Debug, Clone)]
struct PineconeCacheEntry {
    results: Vec<crate::models::Book>,
    timestamp: Instant,
}

// Cache and connection configuration
// Cache TTL in seconds
const CACHE_TTL_SECONDS: u64 = 3600; // 1 hour
const CACHE_CAPACITY: usize = 100;
const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;
const DEFAULT_CONNECTION_TIMEOUT_SECS: u64 = 10;
const DEFAULT_MAX_RETRIES: u32 = 3;
const DEFAULT_RETRY_DELAY_MS: u64 = 500; // Base delay for exponential backoff

#[derive(Clone)]
pub struct Pinecone {
    client: Client,
    api_key: String,
    host: String,
    dimension: usize,
    // Cache for query results to improve performance
    vector_cache: Arc<RwLock<HashMap<String, PineconeCacheEntry>>>,
    metadata_cache: Arc<RwLock<HashMap<String, PineconeCacheEntry>>>,
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
        info!(
            "Initializing Pinecone client with index: '{}', environment: '{}', API key: {}...{}",
            index_name,
            environment,
            &api_key[..std::cmp::min(5, api_key.len())],
            &api_key[api_key.len().saturating_sub(5)..]
        );

        // Get configuration from environment variables with fallbacks
        let request_timeout = env::var("APP_PINECONE_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_REQUEST_TIMEOUT_SECS);

        let connection_timeout = env::var("APP_PINECONE_CONNECTION_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_CONNECTION_TIMEOUT_SECS);

        // Log connection settings for debugging
        info!(
            "Pinecone connection settings: request_timeout={}s, connection_timeout={}s",
            request_timeout, connection_timeout
        );

        // Create HTTP client with optimized settings for cold starts
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(request_timeout))
            .connect_timeout(std::time::Duration::from_secs(connection_timeout))
            // Increase connection pool for better concurrent request handling
            .pool_max_idle_per_host(10)
            // Enable TCP keepalive for better connection reuse
            .tcp_keepalive(Some(std::time::Duration::from_secs(60)))
            // Set a reasonable timeout for TLS handshakes
            .https_only(true)
            // Enable automatic HTTP/2 support where available
            .http2_adaptive_window(true)
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
            vector_cache: Arc::new(RwLock::new(HashMap::with_capacity(CACHE_CAPACITY))),
            metadata_cache: Arc::new(RwLock::new(HashMap::with_capacity(CACHE_CAPACITY))),
        })
    }

    // Check if a result is in the metadata cache
    fn check_metadata_cache(&self, key: &str) -> Option<Vec<crate::models::Book>> {
        if let Ok(cache) = self.metadata_cache.read() {
            if let Some(entry) = cache.get(key) {
                if entry.timestamp.elapsed() < Duration::from_secs(CACHE_TTL_SECONDS) {
                    return Some(entry.results.clone());
                }
            }
        }
        None
    }

    // Update the metadata cache with new results
    fn update_metadata_cache(&self, key: String, results: Vec<crate::models::Book>) {
        if let Ok(mut cache) = self.metadata_cache.write() {
            // Clean up expired cache entries if we're at capacity
            if cache.len() >= CACHE_CAPACITY {
                // Clean up expired entries
                let before_cleanup = cache.len();
                cache.retain(|_, entry| {
                    entry.timestamp.elapsed() < Duration::from_secs(CACHE_TTL_SECONDS)
                });
                let removed = before_cleanup - cache.len();
                if removed > 0 {
                    debug!("Cleaned up {} expired vector cache entries", removed);
                }
            }

            cache.insert(
                key,
                PineconeCacheEntry {
                    results,
                    timestamp: Instant::now(),
                },
            );
        }
    }

    // Check if a result is in the vector cache
    fn check_vector_cache(&self, key: &str) -> Option<Vec<crate::models::Book>> {
        if let Ok(cache) = self.vector_cache.read() {
            if let Some(entry) = cache.get(key) {
                if entry.timestamp.elapsed() < Duration::from_secs(CACHE_TTL_SECONDS) {
                    return Some(entry.results.clone());
                }
            }
        }
        None
    }

    // Update the vector cache with new results
    fn update_vector_cache(&self, key: String, results: Vec<crate::models::Book>) {
        if let Ok(mut cache) = self.vector_cache.write() {
            // Clean up expired cache entries if we're at capacity
            if cache.len() >= CACHE_CAPACITY {
                // Clean up expired entries
                let before_cleanup = cache.len();
                cache.retain(|_, entry| {
                    entry.timestamp.elapsed() < Duration::from_secs(CACHE_TTL_SECONDS)
                });
                let removed = before_cleanup - cache.len();
                if removed > 0 {
                    debug!("Cleaned up {} expired metadata cache entries", removed);
                }
            }

            cache.insert(
                key,
                PineconeCacheEntry {
                    results,
                    timestamp: Instant::now(),
                },
            );
        }
    }

    pub async fn query_metadata(
        &self,
        field: &str,
        value: &str,
        exact_match: bool,
        top_k: usize,
    ) -> Result<Vec<crate::models::Book>> {
        // Generate a cache key
        let cache_key = format!("md_{}_{}_{}_{}", field, value, exact_match, top_k);

        // Check cache first
        if let Some(results) = self.check_metadata_cache(&cache_key) {
            debug!("Metadata query cache hit for: {} = {}", field, value);
            return Ok(results);
        }

        // Build filter based on match type
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

        let results = self.execute_query(query_request).await?;

        // Cache the results
        self.update_metadata_cache(cache_key, results.clone());

        Ok(results)
    }

    pub async fn query_vector(
        &self,
        embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<crate::models::Book>> {
        // Generate a cache key based on a few dimensions to allow some fuzzy matching
        // Using first, middle, and last dimensions to create a reasonable fingerprint
        let cache_key = format!(
            "vec_{:.3}_{:.3}_{:.3}_{:.3}_{:.3}_{:.3}_{}",
            embedding[0],
            embedding[1],
            embedding[embedding.len() / 2 - 1],
            embedding[embedding.len() / 2],
            embedding[embedding.len() - 2],
            embedding[embedding.len() - 1],
            top_k
        );

        // Check cache first
        if let Some(results) = self.check_vector_cache(&cache_key) {
            debug!("Vector query cache hit");
            return Ok(results);
        }

        // Create query request
        let query_request = QueryRequest {
            vector: embedding.to_vec(),
            top_k: top_k as u32,
            include_values: Some(false),
            include_metadata: Some(true),
            filter: None,
            namespace: None,
        };

        let results = self.execute_query(query_request).await?;

        // Cache the results
        self.update_vector_cache(cache_key, results.clone());

        Ok(results)
    }

    async fn execute_query(&self, request: QueryRequest) -> Result<Vec<crate::models::Book>> {
        let url = format!("{}/query", self.host);

        debug!(
            "Making Pinecone query to: {} with top_k: {}",
            url, request.top_k
        );

        // Enhanced retry logic with exponential backoff
        let mut attempts = 0;
        // Get max retries from environment with fallback
        let max_retries = env::var("APP_PINECONE_MAX_RETRIES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_MAX_RETRIES);

        loop {
            attempts += 1;

            // Log detailed information on retry attempts
            if attempts > 1 {
                info!(
                    "Retrying Pinecone query (attempt {}/{})",
                    attempts, max_retries
                );
            }

            let response = self
                .client
                .post(&url)
                .timeout(std::time::Duration::from_secs(60)) // Extended timeout for cold starts
                .header("Api-Key", &self.api_key)
                .header("Content-Type", "application/json")
                .header("X-Pinecone-API-Version", "2025-01")
                .header("User-Agent", "recommend-a-book-rust-api/1.0")
                .json(&request)
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let query_result: QueryResponse = match resp.json().await {
                        Ok(result) => result,
                        Err(e) => {
                            error!("Failed to parse Pinecone response: {}", e);

                            // Retry parsing errors as they might be due to partial responses
                            if attempts < max_retries {
                                let delay = std::time::Duration::from_millis(
                                    DEFAULT_RETRY_DELAY_MS * 2u64.pow(attempts - 1),
                                );
                                warn!(
                                    "Response parsing error, retrying in {:?} (attempt {}/{})",
                                    delay, attempts, max_retries
                                );
                                tokio::time::sleep(delay).await;
                                continue;
                            }

                            return Err(ApiError::PineconeError(format!(
                                "Response parsing failed after {} attempts: {}",
                                max_retries, e
                            )));
                        }
                    };

                    info!(
                        "Pinecone query successful: {} matches returned",
                        query_result.matches.as_ref().map_or(0, |m| m.len())
                    );

                    return self.process_pinecone_results(query_result);
                }
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();

                    // Enhanced retry logic with better status code handling
                    let should_retry = (status.as_u16() >= 500 || // Server errors
                                      status.as_u16() == 429 ||   // Rate limiting
                                      status.as_u16() == 408) &&  // Request timeout
                                      attempts < max_retries;

                    if should_retry {
                        // Exponential backoff with jitter for better retry behavior
                        let base_delay = DEFAULT_RETRY_DELAY_MS * 2u64.pow(attempts - 1);
                        // Add jitter (±20% of base delay)
                        let jitter = base_delay / 5;
                        let jitter_factor = (std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .subsec_nanos()
                            % 100) as u64;

                        let delay = std::time::Duration::from_millis(
                            base_delay.saturating_add(jitter_factor * jitter / 100),
                        );

                        warn!(
                            "Pinecone API returned status {}, retrying in {:?} (attempt {}/{})",
                            status, delay, attempts, max_retries
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }

                    error!("Pinecone API error: {} - {}", status, text);

                    // Improved error handling with more specific error messages
                    if text.contains("unauthorized") || text.contains("forbidden") {
                        return Err(ApiError::PineconeError(
                            format!("Pinecone authentication failed ({}). Please verify your API key is correct.", status)
                        ));
                    } else if text.contains("not found") || status.as_u16() == 404 {
                        return Err(ApiError::PineconeError(
                            format!("Pinecone index or endpoint not found ({}). Please verify your index name '{}' and environment '{}' are correct.",
                                status, self.host.split('/').nth(2).unwrap_or("unknown"),
                                self.host.split('.').nth(2).unwrap_or("unknown"))
                        ));
                    } else if status.as_u16() == 429 {
                        return Err(ApiError::PineconeError(
                            format!("Pinecone rate limit exceeded ({}). Consider reducing query frequency or upgrading your Pinecone plan.", status)
                        ));
                    }

                    return Err(ApiError::PineconeError(format!(
                        "Pinecone API returned unexpected status {}: {}",
                        status, text
                    )));
                }
                Err(e) if attempts < max_retries => {
                    // Enhanced error categorization and retry for network errors
                    let is_timeout = e.is_timeout() || e.is_connect();
                    let is_dns_error =
                        e.to_string().contains("dns") || e.to_string().contains("lookup");

                    // More informative error logging based on error type
                    if is_timeout {
                        warn!(
                            "Pinecone connection timed out, retrying (attempt {}/{}): {}",
                            attempts, max_retries, e
                        );
                    } else if is_dns_error {
                        warn!(
                            "Pinecone DNS resolution error, retrying (attempt {}/{}): {}",
                            attempts, max_retries, e
                        );
                    } else {
                        warn!(
                            "Pinecone network error, retrying (attempt {}/{}): {}",
                            attempts, max_retries, e
                        );
                    }

                    // Exponential backoff with longer delays for connection issues
                    let delay = std::time::Duration::from_millis(
                        DEFAULT_RETRY_DELAY_MS * 4u64.pow(attempts - 1),
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                }
                Err(e) => {
                    error!(
                        "Failed to send request to Pinecone after {} attempts: {}",
                        max_retries, e
                    );

                    // More comprehensive error diagnosis
                    if e.is_timeout() {
                        return Err(ApiError::PineconeError(format!(
                            "Connection to Pinecone timed out after {} attempts. This may indicate network issues or Pinecone service disruption. Error details: {}",
                            max_retries, e
                        )));
                    } else if e.is_connect() {
                        return Err(ApiError::PineconeError(format!(
                            "Cannot connect to Pinecone after {} attempts. Please check your network configuration and Pinecone service status. Error details: {}",
                            max_retries, e
                        )));
                    } else if e.to_string().contains("dns error")
                        || e.to_string().contains("lookup")
                    {
                        return Err(ApiError::PineconeError(format!(
                            "DNS resolution failed for Pinecone host: '{}'. Please verify your APP_PINECONE_ENVIRONMENT ('{}') and APP_PINECONE_INDEX ('{}') environment variables are correct. Error details: {}",
                            self.host,
                            self.host.split('.').nth(2).unwrap_or("unknown"),
                            self.host.split('/').nth(2).unwrap_or("unknown").split('.').next().unwrap_or("unknown"),
                            e
                        )));
                    }

                    return Err(ApiError::PineconeError(format!(
                        "Failed to communicate with Pinecone after {} attempts: {}",
                        max_retries, e
                    )));
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
