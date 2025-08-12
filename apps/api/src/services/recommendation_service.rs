use crate::error::AppError;
use crate::ml::sentence_encoder::SentenceEncoder;
use crate::models::Book;
use crate::services::Pinecone;
use anyhow::{Context, Result};

pub struct RecommendationService {
    model: SentenceEncoder,
    pinecone: Pinecone,
}

impl RecommendationService {
    pub fn new(model: SentenceEncoder, pinecone: Pinecone) -> Self {
        Self {
            model,
            pinecone,
        }
    }

    pub async fn get_recommendations(&self, query: &str, top_k: usize) -> Result<Vec<Book>> {
        // Generate embedding for the query
        let query_embedding = self.model.embed(&[query.to_string()]).await?;
        let embedding = query_embedding.first().ok_or(AppError::EmbeddingError)?;

        // Verify embedding dimension (sentence transformers usually output 384 or 768 dims)
        if embedding.len() != 384 && embedding.len() != 768 && embedding.len() != 512 {
            return Err(AppError::DimensionMismatch {
                expected: 384, // or whatever your model outputs
                got: embedding.len(),
            }
            .into());
        }

        // Query Pinecone using the new client
        let books = self.pinecone
            .query_vector(embedding, top_k)
            .await
            .context("Failed to query Pinecone for recommendations")?;

        Ok(books)
    }
}
