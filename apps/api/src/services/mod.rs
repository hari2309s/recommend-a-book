pub mod pinecone;
pub mod recommendations;
pub mod supabase;
pub mod utils;

// Re-export public types
pub use pinecone::Pinecone;
pub use recommendations::RecommendationService;
