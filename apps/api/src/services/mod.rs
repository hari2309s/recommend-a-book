pub mod pinecone;
pub mod recommendation;
pub mod search_history;
pub mod supabase;

// Re-export public types
pub use pinecone::Pinecone;
pub use recommendation::RecommendationService;
pub use search_history::SearchHistoryService;
pub use supabase::SupabaseClient;
