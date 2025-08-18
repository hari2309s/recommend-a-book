use crate::error::ApiError;
use ndarray::{Array1, Array2};
use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModel, SentenceEmbeddingsModelType,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

/// Original dimension of embeddings produced by DistilRoBERTa model
const ORIGINAL_EMBEDDING_SIZE: usize = 768;
/// Target dimension for Pinecone index
const TARGET_EMBEDDING_SIZE: usize = 512;

/// Provides sentence embeddings using the DistilRoBERTa model,
/// which provides high accuracy embeddings and is mapped to match
/// our vector database dimensions.
#[derive(Clone)]
pub struct UniversalSentenceEncoder {
    model: Arc<Mutex<SentenceEmbeddingsModel>>,
}

impl UniversalSentenceEncoder {
    /// Creates a new instance of the sentence encoder.
    /// This will download the model on first use if not already present.
    pub async fn new() -> Result<Self, ApiError> {
        info!("Initializing sentence embeddings model...");

        // Spawn blocking task to initialize the model
        let model = tokio::task::spawn_blocking(|| {
            SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllDistilrobertaV1)
                .with_device(tch::Device::Cpu)
                .create_model()
        })
        .await
        .map_err(|e| {
            error!("Failed to initialize model in blocking task: {}", e);
            ApiError::ModelLoadError(format!("Failed to spawn model initialization: {}", e))
        })?
        .map_err(|e| {
            error!("Failed to create sentence embeddings model: {}", e);
            ApiError::ModelLoadError(format!("Failed to initialize model: {}", e))
        })?;

        info!("Successfully initialized sentence embeddings model");
        Ok(Self {
            model: Arc::new(Mutex::new(model)),
        })
    }

    /// Encodes a single text string into a vector embedding.
    pub async fn encode(&self, text: &str) -> Result<Vec<f32>, ApiError> {
        debug!("Encoding text: {}", text);

        // Preprocess the text
        let processed_text = self.preprocess_text(text);
        if processed_text.is_empty() {
            return Err(ApiError::InvalidInput(
                "Empty text after preprocessing".into(),
            ));
        }

        let model = self.model.lock().await;

        let embeddings = model.encode(&[processed_text]).map_err(|e| {
            error!("Failed to generate embedding: {}", e);
            ApiError::ModelInferenceError(format!("Encoding failed: {}", e))
        })?;

        if embeddings.is_empty() {
            error!("Model returned empty embeddings");
            return Err(ApiError::ModelInferenceError(
                "Empty embeddings returned".into(),
            ));
        }

        // Map 768D embeddings to 512D using averaging of adjacent dimensions
        let mapped_embeddings = self.map_dimensions(&embeddings[0]);
        debug!("Successfully generated embedding");
        Ok(mapped_embeddings)
    }

    /// Encodes multiple texts in a batch, which is more efficient than encoding one at a time.
    pub async fn encode_batch(&self, texts: &[String]) -> Result<Array2<f32>, ApiError> {
        if texts.is_empty() {
            return Err(ApiError::InvalidInput("Empty batch provided".into()));
        }

        let processed_texts: Vec<String> = texts
            .iter()
            .map(|text| self.preprocess_text(text))
            .collect();

        if processed_texts.iter().any(|text| text.is_empty()) {
            return Err(ApiError::InvalidInput(
                "One or more texts were empty after preprocessing".into(),
            ));
        }

        debug!("Encoding batch of {} texts", texts.len());
        let model = self.model.lock().await;

        let embeddings = model.encode(&processed_texts).map_err(|e| {
            error!("Failed to generate batch embeddings: {}", e);
            ApiError::ModelInferenceError(format!("Batch encoding failed: {}", e))
        })?;

        if embeddings.is_empty() {
            error!("Model returned empty embeddings for batch");
            return Err(ApiError::ModelInferenceError(
                "No embeddings generated".into(),
            ));
        }

        // Convert to 2D array
        let flat_embeddings: Vec<f32> = embeddings.into_iter().flat_map(|v| v).collect();
        debug!(
            "Successfully generated {} embeddings",
            flat_embeddings.len() / TARGET_EMBEDDING_SIZE
        );

        // Reshape into a 2D array
        Array2::from_shape_vec((texts.len(), TARGET_EMBEDDING_SIZE), flat_embeddings).map_err(|e| {
            error!("Failed to reshape embeddings: {}", e);
            ApiError::ModelInferenceError(format!("Failed to reshape embeddings: {}", e))
        })
    }

    /// Preprocesses text before encoding:
    /// - Converts to lowercase
    /// - Removes special characters except spaces
    /// - Normalizes whitespace
    fn preprocess_text(&self, text: &str) -> String {
        text.trim()
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ")
    }

    /// Maps embeddings from 768 dimensions to 512 dimensions using averaging of adjacent dimensions
    fn map_dimensions(&self, original: &[f32]) -> Vec<f32> {
        debug_assert_eq!(original.len(), ORIGINAL_EMBEDDING_SIZE);

        let original = Array1::from_vec(original.to_vec());
        let chunks = ORIGINAL_EMBEDDING_SIZE / TARGET_EMBEDDING_SIZE;
        let remainder = ORIGINAL_EMBEDDING_SIZE % TARGET_EMBEDDING_SIZE;

        let mut result = Vec::with_capacity(TARGET_EMBEDDING_SIZE);

        for i in 0..TARGET_EMBEDDING_SIZE {
            let start = i * chunks;
            let end = if i == TARGET_EMBEDDING_SIZE - 1 {
                start + chunks + remainder
            } else {
                start + chunks
            };

            let chunk_mean = original
                .slice(ndarray::s![start..end])
                .mean()
                .unwrap_or(0.0);
            result.push(chunk_mean);
        }

        result
    }
}
