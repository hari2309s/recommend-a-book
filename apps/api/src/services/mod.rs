pub mod neo4j;
pub mod pinecone;
pub mod query_enhancer;
pub mod recommendation;
pub mod semantic_classifier;
pub mod templates;

// Re-export public types
pub use pinecone::Pinecone;
pub use query_enhancer::QueryEnhancer;
pub use recommendation::RecommendationService;

// Neo4j types are re-exported for use in the build_graph binary
#[cfg(feature = "graph")]
pub use neo4j::{BookNode, BookRelationship, GraphResponse, Neo4jClient, RelationType};
