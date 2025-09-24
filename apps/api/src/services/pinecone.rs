use crate::error::{ApiError, Result};
use log::{debug, error, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

// Cache entry for Pinecone results to improve performance
#[derive(Debug, Clone)]
struct PineconeCacheEntry {
    results: Vec<crate::models::Book>,
    timestamp: Instant,
}

// Cache configuration
// Cache TTL in seconds
const CACHE_TTL_SECONDS: u64 = 3600; // 1 hour
const CACHE_CAPACITY: usize = 100;

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
    pub score: Option<f32>,
    pub metadata: Option<serde_json::Value>,
}

/// Search result containing ID, score, and metadata
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SearchResult {
    /// Unique identifier of the matched item
    pub id: String,
    /// Similarity score (0.0 to 1.0)
    pub score: f32,
    /// Optional metadata containing the book information
    pub metadata: Option<serde_json::Value>,
}

impl From<QueryMatch> for SearchResult {
    fn from(query_match: QueryMatch) -> Self {
        Self {
            id: query_match.id,
            score: query_match.score.unwrap_or(0.0),
            metadata: query_match.metadata,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct TextSearchRequest {
    text: String,
    top_k: u32,
    include_metadata: Option<bool>,
    filter: Option<serde_json::Value>,
    namespace: Option<String>,
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
            &api_key[..std::cmp::min(5, api_key.len())],
            &api_key[api_key.len().saturating_sub(5)..]
        );

        // Create HTTP client with optimized settings for cold starts
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            // Increase connection pool for better concurrent request handling
            .pool_max_idle_per_host(10)
            .build()
            .map_err(|e| {
                ApiError::pinecone_error(format!("Failed to create HTTP client: {}", e))
                    .with_context("pinecone")
                    .with_operation("initialization")
            })?;

        // Validate that we have actual values and not placeholders
        if api_key.is_empty() || api_key.contains("your") || api_key.len() < 10 {
            return Err(ApiError::pinecone_error(
                "Invalid Pinecone API key. Please set a valid APP_PINECONE_API_KEY environment variable.",
            )
            .with_context("pinecone")
            .with_operation("validation"));
        }

        if environment.is_empty()
            || environment.contains("your")
            || environment == "your_environment_name"
        {
            return Err(ApiError::pinecone_error(
                "Invalid Pinecone environment. Please set a valid APP_PINECONE_ENVIRONMENT environment variable.",
            )
            .with_context("pinecone")
            .with_operation("validation"));
        }

        if index_name.is_empty() || index_name.contains("your") || index_name == "your_index_name" {
            return Err(ApiError::pinecone_error(
                "Invalid Pinecone index name. Please set a valid APP_PINECONE_INDEX environment variable.",
            )
            .with_context("pinecone")
            .with_operation("validation"));
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
                cache.retain(|_, entry| {
                    entry.timestamp.elapsed() < Duration::from_secs(CACHE_TTL_SECONDS)
                });
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
                cache.retain(|_, entry| {
                    entry.timestamp.elapsed() < Duration::from_secs(CACHE_TTL_SECONDS)
                });
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

    #[allow(dead_code)]
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
                        ApiError::pinecone_error(format!("Response parsing failed: {}", e))
                            .with_context("pinecone")
                            .with_operation("parse_response")
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
                        return Err(ApiError::pinecone_error(format!(
                            "Pinecone authentication failed ({}). Please verify your API key, index name, and environment are correct.",
                            status
                        ))
                        .with_context("pinecone")
                        .with_operation("process_response"));
                    }

                    return Err(ApiError::pinecone_error(format!(
                        "API returned {}: {}",
                        status, text
                    ))
                    .with_context("pinecone")
                    .with_operation("process_response"));
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
                        return Err(ApiError::pinecone_error(format!(
                            "Connection to Pinecone failed: DNS error. Please verify your APP_PINECONE_ENVIRONMENT and APP_PINECONE_INDEX environment variables. Error details: {}",
                            e
                        ))
                        .with_context("pinecone")
                        .with_operation("request"));
                    }

                    return Err(ApiError::pinecone_error(format!("Request failed: {}", e))
                        .with_context("pinecone")
                        .with_operation("request"));
                }
            }
        }
    }

    /// Search for vectors in Pinecone by vector similarity
    ///
    /// This method performs a semantic search using vector embeddings
    ///
    /// # Arguments
    /// * `vector` - The vector embedding to search with
    /// * `filter` - Optional filter to apply (e.g., for author, category)
    /// * `top_k` - Maximum number of results to return
    ///
    /// # Returns
    /// A vector of search results with scores and metadata
    pub async fn search(
        &self,
        vector: &[f32],
        filter: Option<&serde_json::Value>,
        top_k: usize,
    ) -> std::result::Result<Vec<SearchResult>, ApiError> {
        debug!(
            "Searching Pinecone with vector of dimension {}",
            vector.len()
        );

        // Make sure the vector has the right dimension
        if vector.len() != self.dimension {
            return Err(ApiError::pinecone_error(format!(
                "Vector dimension mismatch: expected {}, got {}",
                self.dimension,
                vector.len()
            ))
            .with_context("pinecone")
            .with_operation("search"));
        }

        // Create query request
        let query_request = QueryRequest {
            vector: vector.to_vec(),
            top_k: top_k as u32,
            include_values: Some(false),
            include_metadata: Some(true),
            filter: filter.cloned(),
            namespace: None,
        };

        // Generate a cache key
        let cache_key = format!(
            "vec_{:x}_{}",
            md5::compute(
                serde_json::to_string(&query_request)
                    .unwrap_or_default()
                    .as_bytes()
            ),
            top_k
        );

        // Check cache first
        if let Some(results) = self.check_vector_cache(&cache_key) {
            debug!("Vector query cache hit for query");
            return Ok(results
                .iter()
                .enumerate()
                .map(|(i, book)| {
                    let score = 1.0 - (i as f32 * 0.01).min(0.9);
                    SearchResult {
                        id: book.id.clone().unwrap_or_else(|| format!("id_{}", i)),
                        score,
                        metadata: Some(serde_json::to_value(book).unwrap_or_default()),
                    }
                })
                .collect());
        }

        // Make API call to Pinecone
        let books = self.execute_query(query_request).await?;

        // Convert books to search results
        let results: Vec<SearchResult> = books
            .iter()
            .enumerate()
            .map(|(i, book)| {
                // Create a synthetic score based on position
                let score = 1.0 - (i as f32 * 0.01).min(0.9);
                SearchResult {
                    id: book.id.clone().unwrap_or_else(|| format!("id_{}", i)),
                    score,
                    metadata: Some(serde_json::to_value(book).unwrap_or_default()),
                }
            })
            .collect();

        // Cache the book results
        let books: Vec<crate::models::Book> = results
            .iter()
            .filter_map(|result| {
                result
                    .metadata
                    .as_ref()
                    .and_then(|m| serde_json::from_value::<crate::models::Book>(m.clone()).ok())
            })
            .collect();

        if !books.is_empty() {
            self.update_vector_cache(cache_key, books);
        }

        Ok(results)
    }

    /// Search for vectors in Pinecone using text
    ///
    /// This method performs a text-based search (if supported by your Pinecone index)
    /// or falls back to a metadata search
    ///
    /// # Arguments
    /// * `query` - The text query to search for
    /// * `top_k` - Maximum number of results to return
    ///
    /// # Returns
    /// A vector of search results with scores and metadata
    pub async fn text_search(
        &self,
        query: &str,
        top_k: usize,
    ) -> std::result::Result<Vec<SearchResult>, ApiError> {
        debug!("Text searching Pinecone with query: '{}'", query);

        // Generate a cache key
        let cache_key = format!("text_{}", query);

        // Check cache first
        if let Some(results) = self.check_vector_cache(&cache_key) {
            debug!("Text query cache hit for: {}", query);
            return Ok(results
                .iter()
                .enumerate()
                .map(|(i, book)| {
                    let score = 1.0 - (i as f32 * 0.01).min(0.9);
                    SearchResult {
                        id: book.id.clone().unwrap_or_else(|| format!("id_{}", i)),
                        score,
                        metadata: Some(serde_json::to_value(book).unwrap_or_default()),
                    }
                })
                .collect());
        }

        // Try to do a metadata search across title, author and description
        let title_results = self
            .query_metadata("title", query, false, top_k)
            .await
            .unwrap_or_default();
        let author_results = self
            .query_metadata("author", query, false, top_k)
            .await
            .unwrap_or_default();
        let desc_results = self
            .query_metadata("description", query, false, top_k)
            .await
            .unwrap_or_default();

        // Combine results with deduplication
        let mut combined_results = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for book in title_results
            .iter()
            .chain(author_results.iter())
            .chain(desc_results.iter())
        {
            if let Some(id) = &book.id {
                if !seen_ids.contains(id) {
                    seen_ids.insert(id.clone());
                    combined_results.push(book.clone());
                }
            }
        }

        // Sort by a relevance score (for now just use rating as a proxy)
        combined_results.sort_by(|a, b| {
            b.rating
                .partial_cmp(&a.rating)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit to top_k
        combined_results.truncate(top_k);

        // Convert to search results
        let results: Vec<SearchResult> = combined_results
            .iter()
            .enumerate()
            .map(|(i, book)| {
                // Calculate a synthetic score based on position
                let score = 1.0 - (i as f32 * 0.1).min(0.9);

                SearchResult {
                    id: book.id.clone().unwrap_or_else(|| format!("id_{}", i)),
                    score,
                    metadata: Some(serde_json::to_value(book).unwrap_or_default()),
                }
            })
            .collect();

        // Cache the book results
        if !combined_results.is_empty() {
            self.update_vector_cache(cache_key, combined_results);
        }

        Ok(results)
    }

    /// Process Pinecone query results into Book objects
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
            return Err(ApiError::pinecone_error(
                "No books could be processed from Pinecone results",
            )
            .with_context("pinecone")
            .with_operation("process_results"));
        }

        debug!(
            "Successfully processed {}/{} books from Pinecone results",
            books.len(),
            matches_len
        );
        Ok(books)
    }
}
