use crate::error::ApiError;
use ndarray::{Array1, Array2};
use reqwest::Client;
use serde_json::json;
use std::env;
use tracing::{debug, error, info, warn};

/// Target dimension for Pinecone index
const TARGET_EMBEDDING_SIZE: usize = 512;

/// Default model configuration
const DEFAULT_MODEL_NAME: &str = "BAAI/bge-large-en-v1.5";
const DEFAULT_BASE_URL: &str = "https://api-inference.huggingface.co";
const DEFAULT_TIMEOUT_SECONDS: u64 = 120; // Increased timeout
const DEFAULT_RETRY_ATTEMPTS: u32 = 3; // Number of retry attempts for API calls
const DEFAULT_RETRY_DELAY_MS: u64 = 1000; // Delay between retries in milliseconds
const BATCH_SIZE_LIMIT: usize = 20; // Maximum number of texts to process in a single batch

/// Text processing limits
const MAX_TEXT_PREVIEW_LENGTH: usize = 100;
// LRU cache size for embedding results
#[allow(dead_code)]
const EMBEDDING_CACHE_SIZE: usize = 100;

use lazy_static::lazy_static;
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};

lazy_static! {
    // Global embedding cache to improve performance and reduce API calls
    static ref EMBEDDING_CACHE: RwLock<lru::LruCache<String, Vec<f32>>> =
        RwLock::new(lru::LruCache::new(NonZeroUsize::new(EMBEDDING_CACHE_SIZE).unwrap()));

    // Flag to track if prewarming has been done
    static ref PREWARM_COMPLETED: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);
}

#[derive(Clone)]

pub struct HuggingFaceEmbedder {
    client: Client,
    api_key: String,
    model_url: String,
    model_name: String,
    initialized: Arc<std::sync::atomic::AtomicBool>,
}

impl HuggingFaceEmbedder {
    /// Creates a new instance of the sentence encoder using HuggingFace API
    /// Uses a 512-dimensional model for better quality embeddings
    /// # Returns
    /// * `Result<HuggingFaceEmbedder, ApiError>` - A new instance of the encoder or an error
    ///
    /// Create a new HuggingFaceEmbedder with optimized initialization for cold starts
    ///
    /// This constructor uses a more efficient initialization pattern that:
    /// 1. Initializes client and configuration immediately
    /// 2. Defers the actual API connection until first use
    /// 3. Supports prewarming to prepare the encoder before the first user request
    pub async fn new() -> Result<Self, ApiError> {
        info!("Creating HuggingFace API sentence encoder (lazy initialization)...");

        let api_key = env::var("APP_HUGGINGFACE_API_KEY").map_err(|_| {
            ApiError::ModelLoadError(
                "Missing APP_HUGGINGFACE_API_KEY environment variable".to_string(),
            )
        })?;

        if api_key.trim().is_empty() {
            return Err(ApiError::ModelLoadError(
                "APP_HUGGINGFACE_API_KEY is empty".to_string(),
            ));
        }

        // Load configuration from environment with defaults
        let timeout_seconds = env::var("APP_HUGGINGFACE_TIMEOUT_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECONDS);

        let base_url =
            env::var("APP_HUGGINGFACE_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());

        let model_name = env::var("APP_HUGGINGFACE_MODEL_NAME")
            .unwrap_or_else(|_| DEFAULT_MODEL_NAME.to_string());

        // Create client with optimized connection settings for better cold starts
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_seconds))
            // Increase connection pool to handle concurrent requests better
            .pool_max_idle_per_host(10)
            // Add connection timeout separate from request timeout
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| ApiError::InternalError(format!("Failed to create HTTP client: {}", e)))?;

        let model_url = format!("{}/models/{}", base_url, model_name);

        info!("Using model: {}", model_name);
        info!("Model URL: {}", model_url);
        info!("Timeout: {}s", timeout_seconds);

        let encoder = Self {
            client,
            api_key,
            model_url,
            model_name,
            initialized: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        info!("HuggingFace encoder created (not yet initialized)");
        Ok(encoder)
    }

    /// Prewarm the encoder to avoid cold start delays
    ///
    /// This method:
    /// 1. Tests the connection to the HuggingFace API
    /// 2. Performs a simple embedding operation to initialize the model
    /// 3. Caches the result for future use
    ///
    /// Returns a boolean indicating if this was the first prewarm operation
    pub async fn prewarm(&self) -> Result<bool, ApiError> {
        // Check if already prewarmed to avoid duplicate work
        let was_first = !PREWARM_COMPLETED.load(std::sync::atomic::Ordering::Acquire);

        if was_first {
            info!("Prewarming HuggingFace encoder...");

            // Test with a simple embedding
            let test_text = "This is a test sentence for prewarming the embeddings model.";
            let _embedding = self.encode(test_text).await?;

            // Mark as initialized
            self.initialized
                .store(true, std::sync::atomic::Ordering::Release);
            PREWARM_COMPLETED.store(true, std::sync::atomic::Ordering::Release);

            info!("HuggingFace encoder successfully prewarmed");
        } else {
            debug!("HuggingFace encoder already prewarmed, skipping");
        }

        Ok(was_first)
    }

    /// Encodes a single text string into a 512-dimensional vector embedding
    /// # Arguments
    /// * `text` - The text to encode
    /// # Returns
    /// * `Result<Vec<f32>, ApiError>` - The encoded embedding or an error
    pub async fn encode(&self, text: &str) -> Result<Vec<f32>, ApiError> {
        info!(
            "Encoding text with {}: '{}' (text length: {})",
            self.model_name,
            if text.len() > MAX_TEXT_PREVIEW_LENGTH {
                &text[..MAX_TEXT_PREVIEW_LENGTH]
            } else {
                text
            },
            text.len()
        );

        let processed_text = self.preprocess_text(text);
        if processed_text.is_empty() {
            error!("Text preprocessing resulted in empty text");
            return Err(ApiError::InvalidInput(
                "Empty text after preprocessing".to_string(),
            ));
        }

        // Load retry configuration
        let retry_config = self.get_retry_config();
        let retry_attempts = retry_config.0;
        let retry_delay_ms = retry_config.1;

        let request_json = json!({
            "inputs": processed_text,
            "options": {
                "wait_for_model": true,
                "use_cache": true
            }
        });

        // Use a retry mechanism for API calls with exponential backoff
        let mut last_error = None;
        for attempt in 1..=retry_attempts {
            info!(
                "HuggingFace API request attempt {}/{}",
                attempt, retry_attempts
            );

            match self.make_api_request(&request_json).await {
                Ok(response) => {
                    // If successful, process the response
                    info!(
                        "HuggingFace API request for '{}' succeeded after {} attempt(s) with status: {}",
                        if text.len() > MAX_TEXT_PREVIEW_LENGTH {
                            &text[..MAX_TEXT_PREVIEW_LENGTH]
                        } else {
                            text
                        },
                        attempt,
                        response.status()
                    );
                    return self.process_api_response(response).await;
                }
                Err(e) => {
                    // Store the error and retry if it's retryable
                    error!("Attempt {}/{} failed: {}", attempt, retry_attempts, e);

                    if attempt < retry_attempts {
                        info!("Waiting {}ms before retry...", retry_delay_ms);
                        tokio::time::sleep(std::time::Duration::from_millis(retry_delay_ms)).await;
                    }
                    last_error = Some(e);
                }
            }
        }

        // If we've exhausted all retries, return the last error
        Err(last_error.unwrap_or_else(|| {
            ApiError::ExternalServiceError("Maximum retry attempts reached".to_string())
        }))
    }

    /// Make a single API request to HuggingFace
    async fn make_api_request(
        &self,
        payload: &serde_json::Value,
    ) -> Result<reqwest::Response, ApiError> {
        self.client
            .post(&self.model_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(payload)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to call HuggingFace API: {}", e);
                ApiError::ExternalServiceError(format!("HuggingFace API request failed: {}", e))
            })
    }

    /// Process the API response
    /// Get retry configuration from environment variables
    fn get_retry_config(&self) -> (u32, u64) {
        let retry_attempts = env::var("APP_HUGGINGFACE_RETRY_ATTEMPTS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_RETRY_ATTEMPTS);

        let retry_delay_ms = env::var("APP_HUGGINGFACE_RETRY_DELAY_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_RETRY_DELAY_MS);

        (retry_attempts, retry_delay_ms)
    }

    async fn process_api_response(
        &self,
        response: reqwest::Response,
    ) -> Result<Vec<f32>, ApiError> {
        let status = response.status();
        info!(
            "Processing HuggingFace API response with status: {}",
            status
        );

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("HuggingFace API error {}: {}", status, error_text);

            return match status.as_u16() {
                401 => Err(ApiError::AuthenticationError(
                    "Invalid or expired HuggingFace API key".to_string(),
                )),
                403 => Err(ApiError::AuthenticationError(
                    "HuggingFace API access forbidden".to_string(),
                )),
                429 => Err(ApiError::ExternalServiceError(
                    "HuggingFace API rate limit exceeded".to_string(),
                )),
                503 => Err(ApiError::ExternalServiceError(
                    "HuggingFace model is currently loading".to_string(),
                )),
                _ => Err(ApiError::ExternalServiceError(format!(
                    "HuggingFace API error {}: {}",
                    status, error_text
                ))),
            };
        }

        // Get the response body as bytes first for logging
        let response_body = response.text().await.map_err(|e| {
            error!("Failed to get HuggingFace response body: {}", e);
            ApiError::SerializationError(format!("Failed to get response body: {}", e))
        })?;

        info!(
            "HuggingFace API response body (preview): {}",
            if response_body.len() > 100 {
                &response_body[..100]
            } else {
                &response_body
            }
        );

        let embeddings: Vec<f32> = serde_json::from_str(&response_body).map_err(|e| {
            error!("Failed to parse HuggingFace response: {}", e);
            ApiError::SerializationError(format!("Failed to parse HuggingFace response: {}", e))
        })?;

        if embeddings.is_empty() {
            error!("HuggingFace API returned empty embeddings");
            return Err(ApiError::ModelInferenceError(
                "Empty embeddings returned from HuggingFace API".to_string(),
            ));
        }

        // Calculate stats for logging
        let _min_value = embeddings.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let _max_value = embeddings.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let sum: f32 = embeddings.iter().sum();
        let _avg = sum / embeddings.len() as f32;

        info!(
            "Raw embedding dimensions: {} - First 5 values: {:?}",
            embeddings.len(),
            embeddings.iter().take(5).collect::<Vec<_>>()
        );

        // Map to exactly 512 dimensions using intelligent dimensionality reduction
        let mapped_embeddings = self.map_to_512_dimensions(&embeddings);

        debug!("Successfully generated 512D embedding for text");
        Ok(mapped_embeddings)
    }

    /// Encodes multiple texts in a batch
    /// # Arguments
    /// * `texts` - A slice of strings to encode
    /// # Returns
    /// * `Result<Array2<f32>, ApiError>` - A 2D array of embeddings or an error
    #[allow(dead_code)]
    pub async fn encode_batch(&self, texts: &[String]) -> Result<Array2<f32>, ApiError> {
        if texts.is_empty() {
            return Err(ApiError::InvalidInput("Empty batch provided".to_string()));
        }

        let processed_texts: Vec<String> = texts
            .iter()
            .map(|text| self.preprocess_text(text))
            .collect();

        if processed_texts.iter().any(|text| text.is_empty()) {
            return Err(ApiError::InvalidInput(
                "One or more texts were empty after preprocessing".to_string(),
            ));
        }

        debug!(
            "Encoding batch of {} texts with {}",
            texts.len(),
            self.model_name
        );

        // Get retry attempts from env or use default
        let retry_attempts = env::var("APP_HUGGINGFACE_RETRY_ATTEMPTS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_RETRY_ATTEMPTS);

        let retry_delay_ms = env::var("APP_HUGGINGFACE_RETRY_DELAY_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_RETRY_DELAY_MS);

        // Prepare request payload
        let request_json = json!({
            "inputs": processed_texts,
            "options": {
                "wait_for_model": true,
                "use_cache": true
            }
        });

        // Use a retry mechanism for batch API calls
        let mut last_error = None;
        for attempt in 1..=retry_attempts {
            info!(
                "HuggingFace batch API request attempt {}/{}",
                attempt, retry_attempts
            );

            match self.make_api_request(&request_json).await {
                Ok(response) => {
                    if attempt > 1 {
                        info!(
                            "HuggingFace batch API request succeeded after {} attempts",
                            attempt
                        );
                    }

                    // Process successful response
                    return self.process_batch_response(response, texts.len()).await;
                }
                Err(e) => {
                    error!("Batch attempt {}/{} failed: {}", attempt, retry_attempts, e);

                    if attempt < retry_attempts {
                        let backoff_ms = retry_delay_ms * (2_u64.pow(attempt - 1));
                        info!("Waiting {}ms before retry...", backoff_ms);
                        tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                    }
                    last_error = Some(e);
                }
            }
        }

        // If we've exhausted all retries, return the last error
        Err(last_error.unwrap_or_else(|| {
            ApiError::ExternalServiceError("Maximum batch retry attempts reached".to_string())
        }))
    }

    /// Process the batch API response
    async fn process_batch_response(
        &self,
        response: reqwest::Response,
        num_texts: usize,
    ) -> Result<Array2<f32>, ApiError> {
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("HuggingFace API batch error {}: {}", status, error_text);

            return match status.as_u16() {
                401 => Err(ApiError::AuthenticationError(
                    "Invalid HuggingFace API key".to_string(),
                )),
                403 => Err(ApiError::AuthenticationError(
                    "HuggingFace API access forbidden".to_string(),
                )),
                429 => Err(ApiError::ExternalServiceError(
                    "HuggingFace API rate limit exceeded".to_string(),
                )),
                503 => Err(ApiError::ExternalServiceError(
                    "HuggingFace model is currently loading".to_string(),
                )),
                _ => Err(ApiError::ExternalServiceError(format!(
                    "HuggingFace API batch error {}: {}",
                    status, error_text
                ))),
            };
        }

        let embeddings: Vec<Vec<f32>> = response.json().await.map_err(|e| {
            error!("Failed to parse HuggingFace batch response: {}", e);
            ApiError::SerializationError(format!(
                "Failed to parse HuggingFace batch response: {}",
                e
            ))
        })?;

        if embeddings.is_empty() {
            error!("HuggingFace API returned empty batch embeddings");
            return Err(ApiError::ModelInferenceError(
                "No embeddings generated for batch".to_string(),
            ));
        }

        // Verify we got the expected number of embeddings
        if embeddings.len() != num_texts {
            warn!(
                "Expected {} embeddings but received {}",
                num_texts,
                embeddings.len()
            );
        }

        // Process each embedding and collect into flat vector
        let mut flat_embeddings = Vec::with_capacity(num_texts * TARGET_EMBEDDING_SIZE);
        for (i, embedding) in embeddings.iter().enumerate() {
            debug!(
                "Processing embedding {} with {} dimensions",
                i,
                embedding.len()
            );
            let mapped = self.map_to_512_dimensions(embedding);
            flat_embeddings.extend(mapped);
        }

        debug!(
            "Successfully generated {} 512D embeddings",
            embeddings.len()
        );

        // Reshape into 2D array
        Array2::from_shape_vec((embeddings.len(), TARGET_EMBEDDING_SIZE), flat_embeddings).map_err(
            |e| {
                error!("Failed to reshape batch embeddings: {}", e);
                ApiError::ModelInferenceError(format!("Failed to reshape batch embeddings: {}", e))
            },
        )
    }

    /// Improved text preprocessing that preserves more semantic information
    fn preprocess_text(&self, text: &str) -> String {
        text.trim()
            .chars()
            .filter(|c| !c.is_control()) // Remove control characters but keep punctuation
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ")
    }

    /// Intelligent mapping to exactly 512 dimensions
    /// Uses PCA-like dimensionality reduction for larger embeddings
    /// and strategic padding for smaller ones
    fn map_to_512_dimensions(&self, original: &[f32]) -> Vec<f32> {
        let original_size = original.len();
        debug!(
            "Mapping from {} to {} dimensions",
            original_size, TARGET_EMBEDDING_SIZE
        );

        if original_size == TARGET_EMBEDDING_SIZE {
            return original.to_vec();
        }

        if original_size < TARGET_EMBEDDING_SIZE {
            // For smaller embeddings, pad with zeros
            let mut result = original.to_vec();
            result.resize(TARGET_EMBEDDING_SIZE, 0.0);
            info!(
                "Padded {} dimensions to {} dimensions with zeros",
                original_size, TARGET_EMBEDDING_SIZE
            );
            return result;
        }

        // For larger embeddings (like 1024D from bge-large), use intelligent reduction
        let result = self.reduce_dimensions_intelligently(original);

        // Calculate statistics for logging
        let min_value = result.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_value = result.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let sum: f32 = result.iter().sum();
        let avg = sum / result.len() as f32;

        info!(
            "Generated {}D embedding - Stats: avg={:.4}, min={:.4}, max={:.4}, sum={:.4}",
            TARGET_EMBEDDING_SIZE, avg, min_value, max_value, sum
        );

        result
    }

    /// Reduce dimensions while preserving maximum information
    /// Uses a simple but effective averaging strategy that maintains vector properties
    fn reduce_dimensions_intelligently(&self, original: &[f32]) -> Vec<f32> {
        let original_size = original.len();
        let original_array = Array1::from_vec(original.to_vec());

        // Calculate how many original dimensions to average for each target dimension
        let scale_factor = original_size as f32 / TARGET_EMBEDDING_SIZE as f32;

        let mut result = Vec::with_capacity(TARGET_EMBEDDING_SIZE);

        for i in 0..TARGET_EMBEDDING_SIZE {
            let start_idx = (i as f32 * scale_factor) as usize;
            let end_idx = ((i + 1) as f32 * scale_factor) as usize;
            let end_idx = end_idx.min(original_size);

            if start_idx < original_size {
                let slice = original_array.slice(ndarray::s![start_idx..end_idx]);
                let mean = slice.mean().unwrap_or(0.0);
                result.push(mean);
            } else {
                result.push(0.0);
            }
        }

        // Normalize the result to maintain vector magnitude properties
        self.normalize_vector(&result)
    }

    /// Normalize vector to unit length (L2 normalization)
    fn normalize_vector(&self, vector: &[f32]) -> Vec<f32> {
        let magnitude: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();

        if magnitude == 0.0 {
            return vector.to_vec();
        }

        vector.iter().map(|x| x / magnitude).collect()
    }

    /// Get information about the current model
    ///
    /// # Returns
    /// * `(String, usize)` - The model name and the target embedding size
    #[allow(dead_code)]
    pub fn model_info(&self) -> (String, usize) {
        (self.model_name.clone(), TARGET_EMBEDDING_SIZE)
    }

    /// Encodes a large batch by splitting into smaller batches to avoid timeouts
    /// # Arguments
    /// * `texts` - A slice of strings to encode
    /// * `batch_size` - The maximum size of each batch
    /// # Returns
    /// * `Result<Array2<f32>, ApiError>` - A 2D array of all embeddings
    #[allow(dead_code)]
    pub async fn encode_large_batch(
        &self,
        texts: &[String],
        batch_size: Option<usize>,
    ) -> Result<Array2<f32>, ApiError> {
        if texts.is_empty() {
            return Err(ApiError::InvalidInput("Empty batch provided".to_string()));
        }

        let batch_size = batch_size.unwrap_or(BATCH_SIZE_LIMIT);
        if batch_size == 0 {
            return Err(ApiError::InvalidInput(
                "Batch size cannot be zero".to_string(),
            ));
        }

        // If batch is small enough, use regular batch encoding
        if texts.len() <= batch_size {
            return self.encode_batch(texts).await;
        }

        // Process in smaller batches
        info!(
            "Large batch of {} texts detected, splitting into batches of {}",
            texts.len(),
            batch_size
        );

        let mut all_embeddings = Vec::new();
        for (i, chunk) in texts.chunks(batch_size).enumerate() {
            info!(
                "Processing batch {}/{}",
                i + 1,
                texts.len().div_ceil(batch_size)
            );

            let chunk_vec = chunk.to_vec();
            let batch_result = self.encode_batch(&chunk_vec).await?;

            // Extract the data from the Array2 and add to our collection
            all_embeddings.extend(batch_result.iter().cloned());
        }

        // Create final Array2 with all embeddings
        let shape = (texts.len(), TARGET_EMBEDDING_SIZE);
        Array2::from_shape_vec(shape, all_embeddings).map_err(|e| {
            error!("Failed to combine batch embeddings: {}", e);
            ApiError::ModelInferenceError(format!("Failed to combine batch embeddings: {}", e))
        })
    }
}
