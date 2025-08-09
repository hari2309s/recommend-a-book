pub mod pinecone;
pub mod recommendation;
pub mod search_history;
pub mod sentence_encoder;
pub mod supabase;

// Re-export public types
pub use pinecone::PineconeClient;
pub use recommendation::RecommendationService;
pub use search_history::SearchHistoryService;
pub use sentence_encoder::SentenceEncoder;
pub use supabase::SupabaseClient;
