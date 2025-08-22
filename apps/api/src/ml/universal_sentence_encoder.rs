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
const DEFAULT_TIMEOUT_SECONDS: u64 = 60;

/// Text processing limits
const MAX_TEXT_PREVIEW_LENGTH: usize = 100;

/// Provides sentence embeddings using HuggingFace Inference API
/// This eliminates the need to download and load large models locally
#[derive(Clone)]
pub struct UniversalSentenceEncoder {
    client: Client,
    api_key: String,
    model_url: String,
    model_name: String,
}

impl UniversalSentenceEncoder {
    /// Creates a new instance of the sentence encoder using HuggingFace API
    /// Uses a 512-dimensional model for better quality embeddings
    pub async fn new() -> Result<Self, ApiError> {
        info!("Initializing HuggingFace API sentence encoder...");

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

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_seconds))
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
        };

        // Test the connection
        encoder.test_connection().await?;

        info!(
            "Successfully initialized HuggingFace API encoder with {}",
            encoder.model_name
        );
        Ok(encoder)
    }

    /// Test the API connection and key validity
    pub async fn test_connection(&self) -> Result<(), ApiError> {
        info!(
            "Testing HuggingFace API connection with model: {}",
            self.model_name
        );

        let test_response = self
            .client
            .post(&self.model_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "inputs": "test connection",
                "options": {
                    "wait_for_model": true,
                    "use_cache": false
                }
            }))
            .send()
            .await
            .map_err(|e| {
                error!("Connection test failed: {}", e);
                ApiError::ExternalServiceError(format!("HuggingFace connection test failed: {}", e))
            })?;

        let status = test_response.status();
        match status.as_u16() {
            200..=299 => {
                info!("HuggingFace API connection test successful");
                Ok(())
            }
            401 => {
                error!("HuggingFace API key is invalid (401 Unauthorized)");
                Err(ApiError::AuthenticationError(
                    "Invalid HuggingFace API key".to_string(),
                ))
            }
            403 => {
                error!("HuggingFace API access forbidden (403)");
                Err(ApiError::AuthenticationError(
                    "HuggingFace API access forbidden".to_string(),
                ))
            }
            503 => {
                warn!("Model is currently loading, this is normal for first request");
                Ok(())
            }
            _ => {
                let error_text = test_response.text().await.unwrap_or_default();
                warn!(
                    "HuggingFace API test returned {}: {} (might be normal)",
                    status, error_text
                );
                Ok(()) // Don't fail here as model might just be loading
            }
        }
    }

    /// Encodes a single text string into a 512-dimensional vector embedding
    pub async fn encode(&self, text: &str) -> Result<Vec<f32>, ApiError> {
        debug!(
            "Encoding text with {}: '{}'",
            self.model_name,
            if text.len() > MAX_TEXT_PREVIEW_LENGTH {
                &text[..MAX_TEXT_PREVIEW_LENGTH]
            } else {
                text
            }
        );

        let processed_text = self.preprocess_text(text);
        if processed_text.is_empty() {
            return Err(ApiError::InvalidInput(
                "Empty text after preprocessing".to_string(),
            ));
        }

        let response = self
            .client
            .post(&self.model_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "inputs": processed_text,
                "options": {
                    "wait_for_model": true,
                    "use_cache": true
                }
            }))
            .send()
            .await
            .map_err(|e| {
                error!("Failed to call HuggingFace API: {}", e);
                ApiError::ExternalServiceError(format!("HuggingFace API request failed: {}", e))
            })?;

        let status = response.status();
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

        let embeddings: Vec<f32> = response.json().await.map_err(|e| {
            error!("Failed to parse HuggingFace response: {}", e);
            ApiError::SerializationError(format!("Failed to parse HuggingFace response: {}", e))
        })?;

        if embeddings.is_empty() {
            error!("HuggingFace API returned empty embeddings");
            return Err(ApiError::ModelInferenceError(
                "Empty embeddings returned from HuggingFace API".to_string(),
            ));
        }

        debug!("Raw embedding dimensions: {}", embeddings.len());

        // Map to exactly 512 dimensions using intelligent dimensionality reduction
        let mapped_embeddings = self.map_to_512_dimensions(&embeddings);

        debug!("Successfully generated 512D embedding for text");
        Ok(mapped_embeddings)
    }

    /// Encodes multiple texts in a batch
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

        let response = self
            .client
            .post(&self.model_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "inputs": processed_texts,
                "options": {
                    "wait_for_model": true,
                    "use_cache": true
                }
            }))
            .send()
            .await
            .map_err(|e| {
                error!("Failed to call HuggingFace API for batch: {}", e);
                ApiError::ExternalServiceError(format!(
                    "HuggingFace batch API request failed: {}",
                    e
                ))
            })?;

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

        // Process each embedding and collect into flat vector
        let mut flat_embeddings = Vec::with_capacity(texts.len() * TARGET_EMBEDDING_SIZE);
        for (i, embedding) in embeddings.iter().enumerate() {
            debug!(
                "Processing embedding {} with {} dimensions",
                i,
                embedding.len()
            );
            let mapped = self.map_to_512_dimensions(embedding);
            flat_embeddings.extend(mapped);
        }

        debug!("Successfully generated {} 512D embeddings", texts.len());

        // Reshape into 2D array
        Array2::from_shape_vec((texts.len(), TARGET_EMBEDDING_SIZE), flat_embeddings).map_err(|e| {
            error!("Failed to reshape batch embeddings: {}", e);
            ApiError::ModelInferenceError(format!("Failed to reshape batch embeddings: {}", e))
        })
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
            return result;
        }

        // For larger embeddings (like 1024D from bge-large), use intelligent reduction
        self.reduce_dimensions_intelligently(original)
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
    pub fn model_info(&self) -> (String, usize) {
        (self.model_name.clone(), TARGET_EMBEDDING_SIZE)
    }
}
