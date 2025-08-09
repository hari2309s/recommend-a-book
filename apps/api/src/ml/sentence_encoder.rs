use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

const SBERT_API_URL: &str =
    "https://api-inference.huggingface.co/models/sentence-transformers/all-MiniLM-L12-v2";

#[derive(Debug)]
pub struct SentenceEncoder {
    client: Client,
    api_key: String,
}

#[derive(Debug, Serialize)]
struct EncoderRequest {
    inputs: Vec<String>,
    options: Options,
}

#[derive(Debug, Serialize)]
struct Options {
    wait_for_model: bool,
}

#[derive(Debug, Deserialize)]
struct EncoderResponse(Vec<Vec<f32>>);

impl SentenceEncoder {
    pub fn new(api_key: String) -> Self {
        info!("Initializing sentence encoder with Hugging Face API");
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, api_key }
    }

    pub async fn encode(&self, text: &str) -> Result<Vec<f32>> {
        debug!("Encoding text: {}", text);
        let request = EncoderRequest {
            inputs: vec![text.to_string()],
            options: Options {
                wait_for_model: true,
            },
        };

        let response = self
            .client
            .post(SBERT_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("API request failed: {}", error_text);
            return Err(anyhow::anyhow!("API request failed: {}", error_text));
        }

        let embeddings: EncoderResponse = response.json().await?;
        debug!("Successfully encoded text into embedding vector");

        // Return the first (and only) embedding
        Ok(embeddings.0.into_iter().next().unwrap_or_default())
    }

    pub async fn encode_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        debug!("Encoding batch of {} texts", texts.len());
        let request = EncoderRequest {
            inputs: texts.to_vec(),
            options: Options {
                wait_for_model: true,
            },
        };

        let response = self
            .client
            .post(SBERT_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("API request failed: {}", error_text);
            return Err(anyhow::anyhow!("API request failed: {}", error_text));
        }

        let embeddings: EncoderResponse = response.json().await?;
        debug!("Successfully encoded batch into embedding vectors");

        Ok(embeddings.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::mock;

    #[tokio::test]
    async fn test_encode_single_text() {
        let mock_response = vec![vec![0.1, 0.2, 0.3]];
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&mock_response).unwrap())
            .create();

        let encoder = SentenceEncoder::new("test-key".to_string());
        let result = encoder.encode("test text").await;

        mock.assert();
        assert!(result.is_ok());
        let embedding = result.unwrap();
        assert_eq!(embedding, vec![0.1, 0.2, 0.3]);
    }

    #[tokio::test]
    async fn test_encode_batch() {
        let mock_response = vec![vec![0.1, 0.2, 0.3], vec![0.4, 0.5, 0.6]];
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&mock_response).unwrap())
            .create();

        let encoder = SentenceEncoder::new("test-key".to_string());
        let texts = vec!["first text".to_string(), "second text".to_string()];
        let result = encoder.encode_batch(&texts).await;

        mock.assert();
        assert!(result.is_ok());
        let embeddings = result.unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0], vec![0.1, 0.2, 0.3]);
        assert_eq!(embeddings[1], vec![0.4, 0.5, 0.6]);
    }

    #[tokio::test]
    async fn test_error_handling() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/")
            .with_status(500)
            .with_body("Internal server error")
            .create();

        let encoder = SentenceEncoder::new("test-key".to_string());
        let result = encoder.encode("test text").await;

        mock.assert();
        assert!(result.is_err());
    }
}
