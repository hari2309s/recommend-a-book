use crate::error::AppError;
use crate::ml::sentence_encoder::SentenceEncoder;
use crate::models::{Book, PineconeMetadata};
use anyhow::Result;
use pinecone_sdk::{Index, QueryRequest};

pub struct RecommendationService {
    model: SentenceEncoder,
    pinecone_index: Index,
}

impl RecommendationService {
    pub fn new(model: SentenceEncoder, pinecone_index: Index) -> Self {
        Self {
            model,
            pinecone_index,
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

        // Query Pinecone
        let query_request = QueryRequest {
            vector: embedding.clone(),
            top_k: top_k as u32,
            include_metadata: true,
            ..Default::default()
        };

        let results = self.pinecone_index.query(&query_request).await?;

        // Convert results to Book structs
        let books = results
            .matches
            .iter()
            .filter_map(|match_result| {
                match_result.metadata.as_ref().map(|metadata| {
                    // Convert metadata to Book
                    Book {
                        title: metadata.get("title")?.as_str()?.to_string(),
                        author: metadata.get("author")?.as_str()?.to_string(),
                        description: metadata.get("description")?.as_str()?.to_string(),
                        rating: metadata.get("rating")?.as_str()?.to_string(),
                        thumbnail: metadata.get("thumbnail")?.as_str()?.to_string(),
                        categories: metadata.get("categories")?.as_str()?.to_string(),
                        published_year: metadata.get("publishedYear")?.as_str()?.to_string(),
                        ratings_count: metadata.get("ratingsCount")?.as_str()?.to_string(),
                    }
                })
            })
            .collect::<Option<Vec<Book>>>()
            .ok_or(AppError::MetadataParsingError)?;

        Ok(books)
    }
}
