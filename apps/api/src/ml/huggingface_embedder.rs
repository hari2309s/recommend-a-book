use crate::error::{ApiError, Result};
use log::{debug, error, info, warn};
use ndarray;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};
use std::time::Duration;

// Configuration constants
const TARGET_EMBEDDING_SIZE: usize = 512;
const DEFAULT_MODEL_NAME: &str = "sentence-transformers/all-MiniLM-L6-v2";
const DEFAULT_BASE_URL: &str = "https://api-inference.huggingface.co";
const DEFAULT_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_CONNECTION_TIMEOUT_SECONDS: u64 = 15;
const DEFAULT_RETRY_ATTEMPTS: u32 = 3;
const DEFAULT_RETRY_DELAY_MS: u64 = 500;
#[allow(dead_code)]
const BATCH_SIZE_LIMIT: usize = 8;

// Text processing limits
const MAX_TEXT_PREVIEW_LENGTH: usize = 100;
const EMBEDDING_CACHE_SIZE: usize = 100;

use lazy_static::lazy_static;

lazy_static! {
    // Global embedding cache to improve performance and reduce API calls
    static ref EMBEDDING_CACHE: RwLock<lru::LruCache<String, Vec<f32>>> = {
        let size = NonZeroUsize::new(EMBEDDING_CACHE_SIZE).unwrap();
        RwLock::new(lru::LruCache::new(size))
    };

    // Flag to track if prewarming has been done
    static ref PREWARM_COMPLETED: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);

    // Track initialization failures for better diagnostics
    static ref INIT_FAILURE_COUNT: std::sync::atomic::AtomicUsize =
        std::sync::atomic::AtomicUsize::new(0);
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
    /// Creates a new HuggingFaceEmbedder using HuggingFace's API
    /// Uses a 512-dimensional model for better quality embeddings
    /// # Returns
    /// * `Result<HuggingFaceEmbedder, ApiError>` - A new instance of the encoder or an error
    /// Create a new HuggingFaceEmbedder with optimized initialization for cold starts
    ///
    /// This constructor uses a more efficient initialization pattern that:
    /// 1. Initializes client and configuration immediately
    /// 2. Defers the actual API connection until first use
    /// 3. Supports prewarming to prepare the encoder before the first user request
    pub async fn new() -> Result<Self> {
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

        let connection_timeout = env::var("APP_EXTERNAL_SERVICE_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_CONNECTION_TIMEOUT_SECONDS);

        let base_url =
            env::var("APP_HUGGINGFACE_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());

        let model_name = env::var("APP_HUGGINGFACE_MODEL_NAME")
            .unwrap_or_else(|_| DEFAULT_MODEL_NAME.to_string());

        info!(
            "Initializing HuggingFace client with model: {}, timeout: {}s, connection timeout: {}s",
            model_name, timeout_seconds, connection_timeout
        );

        // Create client with optimized connection settings for better cold starts
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            // Increase connection pool to handle concurrent requests better
            .pool_max_idle_per_host(10)
            // Add connection timeout separate from request timeout
            .connect_timeout(Duration::from_secs(connection_timeout))
            // Enable TCP keepalive for better connection reuse
            .tcp_keepalive(Some(Duration::from_secs(60)))
            // Set a reasonable timeout for TLS handshakes
            .https_only(true)
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
    pub async fn prewarm(&self) -> Result<bool> {
        // Check if already prewarmed to avoid duplicate work
        let was_first = !PREWARM_COMPLETED.load(std::sync::atomic::Ordering::Acquire);

        if was_first {
            info!("Prewarming HuggingFace encoder...");

            // Get retry count from environment with fallback
            let max_retries = env::var("APP_EXTERNAL_SERVICE_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_RETRY_ATTEMPTS);

            // Test with a simple embedding - with retry logic
            let test_text = "This is a test sentence for prewarming the embeddings model.";

            // Use multiple retry attempts for better reliability during cold start
            for attempt in 1..=max_retries {
                info!("HuggingFace prewarm attempt {}/{}", attempt, max_retries);

                match self.encode(test_text).await {
                    Ok(_embedding) => {
                        // Mark as initialized
                        self.initialized
                            .store(true, std::sync::atomic::Ordering::Release);
                        PREWARM_COMPLETED.store(true, std::sync::atomic::Ordering::Release);

                        // Reset failure count on success
                        INIT_FAILURE_COUNT.store(0, std::sync::atomic::Ordering::Release);

                        info!(
                            "HuggingFace encoder successfully prewarmed on attempt {}",
                            attempt
                        );
                        break;
                    }
                    Err(e) if attempt < max_retries => {
                        // Track failures for diagnostics
                        let failures = INIT_FAILURE_COUNT
                            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                            + 1;

                        warn!(
                            "HuggingFace prewarm attempt {} failed (total failures: {}): {}",
                            attempt, failures, e
                        );

                        // Exponential backoff with jitter
                        let base_delay_ms = DEFAULT_RETRY_DELAY_MS * 2u64.pow(attempt as u32 - 1);
                        let jitter_ms = base_delay_ms / 4; // 25% jitter

                        // Add some randomization to avoid thundering herd
                        let jitter_factor = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .subsec_nanos()
                            % 100;

                        let delay_ms = base_delay_ms + (jitter_ms * jitter_factor as u64 / 100);

                        info!("Retrying HuggingFace prewarm in {}ms", delay_ms);
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    }
                    Err(e) => {
                        // Last attempt failed
                        error!("All HuggingFace prewarm attempts failed: {}", e);
                        return Err(e);
                    }
                }
            }
        } else {
            debug!("HuggingFace encoder already prewarmed, skipping");
        }

        Ok(was_first)
    }

    /// Encodes a single text string into a 512-dimensional vector embedding
    /// Uses the HuggingFace API to generate embeddings
    /// # Arguments
    /// * `text` - The text to encode
    /// # Returns
    /// * `Result<Vec<f32>, ApiError>` - A 512-dimensional embedding vector or an error
    pub async fn encode(&self, text: &str) -> Result<Vec<f32>> {
        let preprocessed = self.preprocess_text(text);

        // Check cache first for improved performance
        let cache_key = format!("text_{}", preprocessed);
        // Use read lock to check if item exists, clone it if it does
        if let Ok(cache_guard) = EMBEDDING_CACHE.read() {
            // LruCache get() returns a reference, so we need to clone it
            let cached_embedding = cache_guard.peek(&cache_key).cloned();
            if let Some(embedding) = cached_embedding {
                debug!(
                    "Cache hit for text embedding: {}",
                    &preprocessed[..std::cmp::min(30, preprocessed.len())]
                );
                return Ok(embedding);
            }
        }

        debug!(
            "Encoding text (length: {}): {}{}",
            text.len(),
            &text[..std::cmp::min(MAX_TEXT_PREVIEW_LENGTH, text.len())],
            if text.len() > MAX_TEXT_PREVIEW_LENGTH {
                "..."
            } else {
                ""
            }
        );

        let inputs = preprocessed;
        let (retry_attempts, retry_delay_ms) = self.get_retry_config();

        // Try the API request with retry
        for attempt in 1..=retry_attempts {
            match self.make_api_request(&inputs).await {
                Ok(response) => {
                    match self.process_api_response(response).await {
                        Ok(embedding) => {
                            // Cache the result for future use - get a write lock and add to cache
                            if let Ok(mut cache_guard) = EMBEDDING_CACHE.write() {
                                cache_guard.put(cache_key.clone(), embedding.clone());
                            }
                            return Ok(embedding);
                        }
                        Err(e) => {
                            if attempt == retry_attempts {
                                return Err(e);
                            }
                            // If not the last attempt, retry
                            warn!(
                                "Failed to process embedding (attempt {}/{}): {}. Retrying...",
                                attempt, retry_attempts, e
                            );
                            tokio::time::sleep(Duration::from_millis(
                                retry_delay_ms * 2u64.pow(attempt - 1),
                            ))
                            .await;
                        }
                    }
                }
                Err(e) => {
                    if attempt == retry_attempts {
                        return Err(e);
                    }
                    // If not the last attempt, retry
                    warn!(
                        "API request failed (attempt {}/{}): {}. Retrying...",
                        attempt, retry_attempts, e
                    );
                    tokio::time::sleep(Duration::from_millis(
                        retry_delay_ms * 2u64.pow(attempt - 1),
                    ))
                    .await;
                }
            }
        }

        // This should never be reached due to the loop structure, but Rust requires a return
        Err(ApiError::ModelError(
            "All retry attempts failed when encoding text".to_string(),
        ))
    }

    async fn make_api_request(&self, input: &str) -> Result<reqwest::Response> {
        #[derive(Serialize)]
        struct Request<'a> {
            inputs: &'a str,
            options: Options,
        }

        #[derive(Serialize)]
        struct Options {
            wait_for_model: bool,
            use_cache: bool,
        }

        let request = Request {
            inputs: input,
            options: Options {
                wait_for_model: true,
                use_cache: true,
            },
        };

        let response = self
            .client
            .post(&self.model_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                ApiError::ModelError(format!("Failed to send request to model API: {}", e))
            })?;

        Ok(response)
    }

    /// Get retry configuration from environment variables
    fn get_retry_config(&self) -> (u32, u64) {
        let retry_attempts = env::var("APP_EXTERNAL_SERVICE_RETRIES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                env::var("APP_HUGGINGFACE_RETRY_ATTEMPTS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(DEFAULT_RETRY_ATTEMPTS)
            });

        let retry_delay_ms = env::var("APP_HUGGINGFACE_RETRY_DELAY_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_RETRY_DELAY_MS);

        (retry_attempts, retry_delay_ms)
    }

    async fn process_api_response(&self, response: reqwest::Response) -> Result<Vec<f32>> {
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();

            // Special handling for common HuggingFace errors
            if status.as_u16() == 404 {
                return Err(ApiError::ModelError(format!(
                    "Model not found: {}. Please check the model name in your configuration.",
                    self.model_name
                )));
            } else if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err(ApiError::ModelError(
                    "Authentication failed. Please check your HuggingFace API key.".to_string(),
                ));
            } else if status.as_u16() == 429 {
                return Err(ApiError::ModelError(
                    "Rate limit exceeded. Please reduce the frequency of your requests or upgrade your HuggingFace subscription.".to_string(),
                ));
            }

            return Err(ApiError::ModelError(format!(
                "HuggingFace API returned non-success status: {} - {}",
                status, text
            )));
        }

        // Define a struct to parse different embedding response formats
        #[derive(Debug, Deserialize, Default)]
        struct EmbeddingResponse {
            #[serde(default)]
            embeddings: Vec<Vec<f32>>,

            #[serde(default)]
            embedding: Vec<f32>,
        }

        let response_text = response
            .text()
            .await
            .map_err(|e| ApiError::ModelError(format!("Failed to get response text: {}", e)))?;

        let response_json = match serde_json::from_str::<serde_json::Value>(&response_text) {
            Ok(json) => json,
            Err(e) => {
                return Err(ApiError::ModelError(format!(
                    "Failed to parse response as JSON: {}",
                    e
                )));
            }
        };

        debug!(
            "Got response with type: {}",
            response_json.as_array().is_some()
        );

        let mut embedding = Vec::new();

        if response_json.is_array() {
            // Response is an array of arrays or an array of floats
            let array = response_json.as_array().unwrap();

            if array.is_empty() {
                return Err(ApiError::ModelError(
                    "Received empty array from model".to_string(),
                ));
            }

            if array[0].is_array() {
                // [[0.1, 0.2, ...]] format
                if let Some(first_array) = array.get(0).and_then(|v| v.as_array()) {
                    embedding = first_array
                        .iter()
                        .filter_map(|v| v.as_f64().map(|f| f as f32))
                        .collect();
                }
            } else {
                // [0.1, 0.2, ...] format
                embedding = array
                    .iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect();
            }
        } else if response_json.is_object() {
            // Try parsing as EmbeddingResponse
            let parsed: EmbeddingResponse = serde_json::from_value(response_json).map_err(|e| {
                ApiError::ModelError(format!("Failed to parse embedding response: {}", e))
            })?;

            if !parsed.embedding.is_empty() {
                // { "embedding": [0.1, 0.2, ...] } format
                embedding = parsed.embedding;
            } else if !parsed.embeddings.is_empty() {
                // { "embeddings": [[0.1, 0.2, ...]] } format
                embedding = parsed.embeddings.into_iter().next().unwrap_or_default();
            }
        }

        if embedding.is_empty() {
            return Err(ApiError::ModelError(
                "Failed to extract embedding from response".to_string(),
            ));
        }

        debug!(
            "Got embedding of size {} from HuggingFace API",
            embedding.len()
        );

        // Ensure we always return a 512-dimensional vector
        let normalized = self.normalize_vector(&embedding);
        let embedding = if normalized.len() != TARGET_EMBEDDING_SIZE {
            debug!(
                "Resizing embedding from {} to {} dimensions",
                normalized.len(),
                TARGET_EMBEDDING_SIZE
            );
            self.map_to_512_dimensions(&normalized)
        } else {
            normalized
        };

        Ok(embedding)
    }

    /// Preprocess text for better embedding quality
    fn preprocess_text(&self, text: &str) -> String {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return "empty text".to_string();
        }
        trimmed.to_string()
    }

    /// Map an embedding vector to 512 dimensions
    /// This allows us to handle different model outputs consistently
    fn map_to_512_dimensions(&self, embedding: &[f32]) -> Vec<f32> {
        let input_dim = embedding.len();
        let target_dim = TARGET_EMBEDDING_SIZE;

        match input_dim {
            d if d == target_dim => embedding.to_vec(),
            d if d < target_dim => self.reduce_dimensions_intelligently(embedding, target_dim),
            d if d > target_dim => self.reduce_dimensions_intelligently(embedding, target_dim),
            _ => unreachable!(),
        }
    }

    /// Intelligently resize an embedding vector to the target dimensions
    /// Uses a technique that preserves as much information as possible
    fn reduce_dimensions_intelligently(&self, embedding: &[f32], target_dim: usize) -> Vec<f32> {
        let input_dim = embedding.len();
        let mut result = vec![0.0; target_dim];

        if input_dim > target_dim {
            // Downsampling: Average multiple input dimensions to one output
            let ratio = input_dim as f32 / target_dim as f32;

            for i in 0..target_dim {
                let start_idx = (i as f32 * ratio).floor() as usize;
                let end_idx = ((i + 1) as f32 * ratio).min(input_dim as f32).floor() as usize;

                let mut sum = 0.0;
                let mut count = 0;

                for j in start_idx..end_idx {
                    sum += embedding[j];
                    count += 1;
                }

                result[i] = if count > 0 { sum / count as f32 } else { 0.0 };
            }
        } else {
            // Upsampling: Interpolate between input values
            let ratio = (input_dim - 1) as f32 / (target_dim - 1) as f32;

            for i in 0..target_dim {
                let exact_idx = i as f32 * ratio;
                let lower_idx = exact_idx.floor() as usize;
                let upper_idx = (lower_idx + 1).min(input_dim - 1);
                let weight = exact_idx - lower_idx as f32;

                result[i] = embedding[lower_idx] * (1.0 - weight) + embedding[upper_idx] * weight;
            }
        }

        // Normalize the result
        self.normalize_vector(&result)
    }

    /// Normalize a vector to unit length
    fn normalize_vector(&self, vector: &[f32]) -> Vec<f32> {
        let squared_sum: f32 = vector.iter().map(|&x| x * x).sum();
        let magnitude = squared_sum.sqrt();

        if magnitude > 0.0 {
            vector.iter().map(|&x| x / magnitude).collect()
        } else {
            // If vector has zero magnitude, return zeros
            vec![0.0; vector.len()]
        }
    }

    #[allow(dead_code)]
    /// Get information about the model being used
    pub fn model_info(&self) -> String {
        format!("Model: {}", self.model_name)
    }

    #[allow(dead_code)]
    /// Encodes multiple texts in a batch
    /// # Arguments
    /// * `texts` - A slice of strings to encode
    /// # Returns
    /// * `Result<Array2<f32>, ApiError>` - A 2D array of embeddings or an error
    pub async fn encode_batch(&self, texts: &[String]) -> Result<ndarray::Array2<f32>> {
        if texts.is_empty() {
            return Err(ApiError::InvalidInput("Empty batch provided".to_string()));
        }

        debug!("Encoding batch of {} texts", texts.len());

        // Process texts one by one
        let mut all_embeddings = Vec::with_capacity(texts.len() * TARGET_EMBEDDING_SIZE);

        for (i, text) in texts.iter().enumerate() {
            match self.encode(text).await {
                Ok(embedding) => {
                    all_embeddings.extend_from_slice(&embedding);
                }
                Err(e) => {
                    error!(
                        "Failed to generate embedding for text {} in batch: {}",
                        i, e
                    );
                    return Err(e);
                }
            }
        }

        // Reshape the flat vector into a 2D array
        ndarray::Array2::from_shape_vec((texts.len(), TARGET_EMBEDDING_SIZE), all_embeddings)
            .map_err(|e| ApiError::ModelError(format!("Failed to reshape embeddings: {}", e)))
    }
}
