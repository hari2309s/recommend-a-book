use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Embedding generation failed")]
    EmbeddingError,

    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("Failed to parse metadata")]
    MetadataParsingError,

    #[error("Model loading error: {0}")]
    ModelError(String),
}
