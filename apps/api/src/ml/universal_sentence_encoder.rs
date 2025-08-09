use crate::error::ApiError;
use ndarray::{Array1, Array2};
use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModel, SentenceEmbeddingsModelType,
};
use std::{str::FromStr, sync::Arc};
use tokio::sync::Mutex;
use tracing::{debug, error, info};

/// Dimension of embeddings produced by all-MiniLM-L6-v2 model
const EMBEDDING_SIZE: usize = 384;

/// Provides sentence embeddings using the all-MiniLM-L6-v2 model,
/// which is a good balance between speed and accuracy.
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
            SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
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
    pub async fn encode(&self, text: &str) -> Result<Array1<f32>, ApiError> {
        let processed_text = self.preprocess_text(text);
        if processed_text.is_empty() {
            return Err(ApiError::InvalidInput(
                "Empty text after preprocessing".into(),
            ));
        }

        debug!("Encoding text: {}", processed_text);
        let model = self.model.lock().await;

        let embeddings = model.encode(&[processed_text]).map_err(|e| {
            error!("Failed to generate embedding: {}", e);
            ApiError::ModelInferenceError(format!("Encoding failed: {}", e))
        })?;

        if embeddings.is_empty() {
            error!("Model returned empty embeddings");
            return Err(ApiError::ModelInferenceError(
                "No embedding generated".into(),
            ));
        }

        debug!("Successfully generated embedding");
        Ok(Array1::from_vec(embeddings[0].clone()))
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
            flat_embeddings.len() / EMBEDDING_SIZE
        );

        // Reshape into a 2D array
        Array2::from_shape_vec((texts.len(), EMBEDDING_SIZE), flat_embeddings).map_err(|e| {
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

    #[cfg(test)]
    pub async fn mock() -> Self {
        // Create a mock model for testing
        let model = tokio::task::spawn_blocking(|| {
            SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
                .with_device(tch::Device::Cpu)
                .create_model()
        })
        .await
        .unwrap()
        .unwrap();

        Self {
            model: Arc::new(Mutex::new(model)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encode_single_text() {
        let encoder = UniversalSentenceEncoder::new().unwrap();
        let result = encoder.encode("test text").await;
        assert!(result.is_ok());
        let embedding = result.unwrap();
        assert_eq!(embedding.len(), EMBEDDING_SIZE);
    }

    #[tokio::test]
    async fn test_encode_batch() {
        let encoder = UniversalSentenceEncoder::new().unwrap();
        let texts = vec![
            "first test text".to_string(),
            "second test text".to_string(),
        ];
        let result = encoder.encode_batch(&texts).await.unwrap();
        assert_eq!(result.shape(), &[2, EMBEDDING_SIZE]);
    }

    #[tokio::test]
    async fn test_empty_text() {
        let encoder = UniversalSentenceEncoder::new().unwrap();
        let result = encoder.encode("").await;
        assert!(matches!(
            result,
            Err(ApiError::InvalidInput(msg)) if msg == "Empty text after preprocessing"
        ));
    }

    #[tokio::test]
    async fn test_empty_batch() {
        let encoder = UniversalSentenceEncoder::new().unwrap();
        let result = encoder.encode_batch(&[]).await;
        assert!(matches!(
            result,
            Err(ApiError::InvalidInput(msg)) if msg == "Empty batch provided"
        ));
    }

    #[test]
    fn test_preprocess_text() {
        let encoder = UniversalSentenceEncoder::new().unwrap();
        let processed = encoder.preprocess_text("Test, this! TEXT   here.");
        assert_eq!(processed, "test this text here");
    }

    #[test]
    fn test_preprocess_text_special_chars() {
        let encoder = UniversalSentenceEncoder::new().unwrap();
        let processed = encoder.preprocess_text("@#$%^&* Special chars!@#");
        assert_eq!(processed, "special chars");
    }
}
