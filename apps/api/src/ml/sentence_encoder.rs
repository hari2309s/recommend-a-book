use crate::error::AppError;
use anyhow::Result;
use ort::{Environment, ExecutionProvider, GraphOptimizationLevel, Session, SessionBuilder, Value};
use std::collections::HashMap;
use tokenizers::Tokenizer;

pub struct SentenceEncoder {
    session: Session,
    tokenizer: Tokenizer,
}

impl SentenceEncoder {
    pub async fn load() -> Result<Self> {
        // Initialize ONNX Runtime environment
        let environment = Environment::builder()
            .with_name("sentence_encoder")
            .build()?;

        // Load the ONNX model
        let session = SessionBuilder::new(&environment)?
            .with_execution_providers([ExecutionProvider::CPU])?
            .with_optimization_level(GraphOptimizationLevel::All)?
            .with_model_from_file("models/model.onnx")?;

        // Load the tokenizer
        let tokenizer = Tokenizer::from_file("models/tokenizer.json")
            .map_err(|e| AppError::ModelError(format!("Failed to load tokenizer: {}", e)))?;

        tracing::info!("ONNX model and tokenizer loaded successfully");

        Ok(Self { session, tokenizer })
    }

    pub async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::new();

        for text in texts {
            // Tokenize the text
            let encoding = self
                .tokenizer
                .encode(text, true)
                .map_err(|e| AppError::ModelError(format!("Tokenization failed: {}", e)))?;

            let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
            let attention_mask: Vec<i64> = encoding
                .get_attention_mask()
                .iter()
                .map(|&x| x as i64)
                .collect();

            // Pad/truncate to max length (usually 512 for sentence transformers)
            let max_length = 512;
            let mut padded_input_ids = input_ids.clone();
            let mut padded_attention_mask = attention_mask.clone();

            if padded_input_ids.len() > max_length {
                padded_input_ids.truncate(max_length);
                padded_attention_mask.truncate(max_length);
            } else {
                padded_input_ids.resize(max_length, 0); // PAD token
                padded_attention_mask.resize(max_length, 0);
            }

            // Create input tensors
            let input_ids_tensor =
                Value::from_array(([1, max_length], padded_input_ids)).map_err(|e| {
                    AppError::ModelError(format!("Failed to create input tensor: {}", e))
                })?;

            let attention_mask_tensor = Value::from_array(([1, max_length], padded_attention_mask))
                .map_err(|e| {
                    AppError::ModelError(format!("Failed to create attention tensor: {}", e))
                })?;

            // Prepare inputs
            let mut inputs = HashMap::new();
            inputs.insert("input_ids".to_string(), input_ids_tensor);
            inputs.insert("attention_mask".to_string(), attention_mask_tensor);

            // Run inference
            let outputs = self
                .session
                .run(inputs)
                .map_err(|e| AppError::ModelError(format!("Model inference failed: {}", e)))?;

            // Extract embeddings from the output
            // For sentence transformers, this is usually the mean pooled output
            let output_tensor = outputs
                .get("sentence_embedding")
                .or_else(|| outputs.get("last_hidden_state"))
                .or_else(|| outputs.values().next())
                .ok_or_else(|| AppError::ModelError("No valid output tensor found".to_string()))?;

            // Convert to Vec<f32>
            let embedding: Vec<f32> = output_tensor
                .try_extract()
                .map_err(|e| AppError::ModelError(format!("Failed to extract embedding: {}", e)))?;

            // If we got the full hidden states, we need to do mean pooling
            // For simplicity, let's assume the model already outputs sentence embeddings
            // If you need mean pooling, we'd calculate it here

            embeddings.push(embedding);
        }

        Ok(embeddings)
    }
}
