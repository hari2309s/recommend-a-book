use crate::error::ApiError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error};

const HUGGINGFACE_API_BASE: &str = "https://api-inference.huggingface.co/models";
const MODEL_NAME: &str = "sentence-transformers/all-MiniLM-L6-v2";

#[derive(Debug, Clone)]
pub struct SentenceEncoder {
    client: Client,
    api_key: String,
}

#[derive(Debug, Serialize)]
struct EncodeRequest {
    inputs: Vec<String>,
    options: Options,
}

#[derive(Debug, Serialize)]
struct Options {
    wait_for_model: bool,
    use_cache: bool,
}

#[derive(Debug, Deserialize)]
struct EncodeResponse(Vec<Vec<f32>>);

impl SentenceEncoder {
    pub fn new(api_key: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key: api_key.to_string(),
        }
    }

    pub async fn encode(&self, text: &str) -> Result<Vec<f32>, ApiError> {
        let embeddings = self.encode_batch(&[text.to_string()]).await?;
        Ok(embeddings[0].clone())
    }

    pub async fn encode_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, ApiError> {
        let request = EncodeRequest {
            inputs: texts.to_vec(),
            options: Options {
                wait_for_model: true,
                use_cache: true,
            },
        };

        debug!("Sending request to HuggingFace API");
        let url = format!("{}/{}", HUGGINGFACE_API_BASE, MODEL_NAME);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                ApiError::ExternalServiceError(format!("HuggingFace API request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("HuggingFace API error: {}", error_text);
            return Err(ApiError::ExternalServiceError(format!(
                "HuggingFace API error: {}",
                error_text
            )));
        }

        let embeddings: EncodeResponse = response.json().await.map_err(|e| {
            ApiError::SerializationError(format!("Failed to parse HuggingFace response: {}", e))
        })?;

        Ok(embeddings.0)
    }

    #[cfg(test)]
    pub fn mock() -> Self {
        Self {
            client: Client::new(),
            api_key: "test-key".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use tokio;

    #[tokio::test]
    async fn test_encode_single_text() {
        let encoder = SentenceEncoder::mock();
        let result = encoder.encode("test text").await;
        assert!(result.is_err()); // Will fail in mock because no actual API call
    }

    #[tokio::test]
    async fn test_encode_batch() {
        let encoder = SentenceEncoder::mock();
        let texts = vec!["text1".to_string(), "text2".to_string()];
        let result = encoder.encode_batch(&texts).await;
        assert!(result.is_err()); // Will fail in mock because no actual API call
    }
}
