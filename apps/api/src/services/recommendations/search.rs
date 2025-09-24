//! Search strategy implementation for the recommendation service.
//!
//! This module defines different search strategies based on query intent,
//! including metadata filtering and hybrid search options.

use log::info;

use crate::error::ApiError;
use crate::models::Book;
use crate::services::pinecone::Pinecone;
use crate::services::recommendations::intent::QueryIntent;

/// A search strategy that determines how to search for books
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SearchStrategy {
    /// Optional metadata filter to narrow search by specific fields
    pub metadata_filter: Option<MetadataFilter>,
    /// Weight to give to semantic/vector similarity (0.0-1.0)
    pub semantic_weight: f32,
    /// Whether to use hybrid search combining semantic and metadata
    pub hybrid_search: bool,
}

impl Default for SearchStrategy {
    fn default() -> Self {
        Self {
            metadata_filter: None,
            semantic_weight: 0.8,
            hybrid_search: true,
        }
    }
}

/// Filter for querying by specific metadata fields
#[derive(Debug, Clone)]
pub struct MetadataFilter {
    /// Field name to filter on (e.g., "author", "category")
    pub field: String,
    /// Value to match against the field
    pub value: String,
    /// Whether to require exact match or use partial matching
    pub exact_match: bool,
}

/// Determine the appropriate search strategy based on the query intent
pub fn get_search_strategy(intent: &QueryIntent) -> SearchStrategy {
    match intent {
        QueryIntent::Author { name, .. } => {
            info!("Using author-based search strategy for '{}'", name);
            SearchStrategy {
                metadata_filter: Some(MetadataFilter {
                    field: "author".to_string(),
                    value: name.clone(),
                    exact_match: false,
                }),
                semantic_weight: 0.5, // Balance between author match and semantic similarity
                hybrid_search: true,
            }
        }
        QueryIntent::Genre { genre, .. } => {
            info!("Using genre-based search strategy for '{}'", genre);
            SearchStrategy {
                metadata_filter: Some(MetadataFilter {
                    field: "categories".to_string(),
                    value: genre.clone(),
                    exact_match: false,
                }),
                semantic_weight: 0.7, // Higher emphasis on semantic similarity for genres
                hybrid_search: true,
            }
        }
        QueryIntent::SimilarTo { .. } => {
            info!("Using similarity-based search strategy");
            SearchStrategy {
                metadata_filter: None, // No specific metadata filter for similarity searches
                semantic_weight: 0.9,  // Heavy emphasis on semantic similarity
                hybrid_search: true,
            }
        }
        QueryIntent::General { .. } => {
            info!("Using general search strategy");
            SearchStrategy {
                metadata_filter: None,
                semantic_weight: 0.8, // Balanced approach for general searches
                hybrid_search: true,
            }
        }
    }
}

/// Perform a hybrid search using both vector similarity and metadata filtering
pub async fn perform_hybrid_search(
    pinecone: &Pinecone,
    embedding: &[f32],
    strategy: &SearchStrategy,
    top_k: usize,
) -> Result<Vec<(Book, f32)>, ApiError> {
    info!(
        "Performing hybrid search with strategy: {:?}, top_k: {}",
        strategy, top_k
    );

    // Prepare filter if specified in strategy
    let filter = strategy.metadata_filter.as_ref().map(|filter| {
        serde_json::json!({
            filter.field.clone(): {
                // For exact match use eq, otherwise use $contains for partial matching
                if filter.exact_match { "$eq" } else { "$contains" }: filter.value.clone()
            }
        })
    });

    // Call Pinecone API with the embedding vector, filter, and number of results
    let search_results = pinecone
        .search(embedding, filter.as_ref(), top_k)
        .await
        .map_err(|e| {
            ApiError::external_service_error(format!("Pinecone search failed: {}", e))
                .with_context("search")
                .with_operation("hybrid_search")
        })?;

    // Map results to Book objects with their scores
    let books_with_scores: Vec<(Book, f32)> = search_results
        .iter()
        .filter_map(|result| {
            // Extract and deserialize the book metadata from the search result
            result
                .metadata
                .as_ref()
                .and_then(|metadata| serde_json::from_value::<Book>(metadata.clone()).ok())
                .map(|book| (book, result.score))
        })
        .collect();

    // Extract just the books for return value compatibility
    let _books = books_with_scores
        .iter()
        .map(|(book, _)| book.clone())
        .collect::<Vec<Book>>();

    if books_with_scores.is_empty() {
        info!("Hybrid search returned no results, may need fallback strategy");
    } else {
        info!("Hybrid search returned {} results", books_with_scores.len());
    }

    Ok(books_with_scores)
}

/// Perform a fallback search when the primary search returns no results
pub async fn perform_fallback_search(
    pinecone: &Pinecone,
    query: &str,
    top_k: usize,
) -> Result<Vec<(Book, f32)>, ApiError> {
    info!(
        "Performing fallback search for query '{}' with top_k: {}",
        query, top_k
    );

    // For fallback, we use a more general search without specific filters
    // This can be customized based on the application's needs
    let _fallback_strategy = SearchStrategy {
        metadata_filter: None,
        semantic_weight: 1.0, // Pure semantic search
        hybrid_search: false,
    };

    // Construct a simple search request to Pinecone
    // This could alternatively use a different approach like a database fulltext search
    let search_results = pinecone.text_search(query, top_k).await.map_err(|e| {
        ApiError::external_service_error(format!("Fallback search failed: {}", e))
            .with_context("search")
            .with_operation("fallback_search")
    })?;

    // Extract books with scores from search results
    let books_with_scores: Vec<(Book, f32)> = search_results
        .iter()
        .filter_map(|result| {
            result
                .metadata
                .as_ref()
                .and_then(|metadata| serde_json::from_value::<Book>(metadata.clone()).ok())
                .map(|book| (book, result.score))
        })
        .collect();

    info!(
        "Fallback search returned {} results",
        books_with_scores.len()
    );
    Ok(books_with_scores)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_author_strategy() {
        let intent = QueryIntent::Author {
            name: "J.K. Rowling".to_string(),
            original_query: "books by J.K. Rowling".to_string(),
        };

        let strategy = get_search_strategy(&intent);

        assert!(strategy.hybrid_search);
        assert_eq!(strategy.semantic_weight, 0.5);
        assert!(strategy.metadata_filter.is_some());

        if let Some(filter) = strategy.metadata_filter {
            assert_eq!(filter.field, "author");
            assert_eq!(filter.value, "J.K. Rowling");
            assert!(!filter.exact_match);
        }
    }

    #[test]
    fn test_genre_strategy() {
        let intent = QueryIntent::Genre {
            genre: "fantasy".to_string(),
            original_query: "fantasy books".to_string(),
        };

        let strategy = get_search_strategy(&intent);

        assert!(strategy.hybrid_search);
        assert_eq!(strategy.semantic_weight, 0.7);
        assert!(strategy.metadata_filter.is_some());

        if let Some(filter) = strategy.metadata_filter {
            assert_eq!(filter.field, "categories");
            assert_eq!(filter.value, "fantasy");
            assert!(!filter.exact_match);
        }
    }

    #[test]
    fn test_similarity_strategy() {
        let intent = QueryIntent::SimilarTo {
            original_query: "books similar to Harry Potter".to_string(),
        };

        let strategy = get_search_strategy(&intent);

        assert!(strategy.hybrid_search);
        assert_eq!(strategy.semantic_weight, 0.9);
        assert!(strategy.metadata_filter.is_none());
    }

    #[test]
    fn test_general_strategy() {
        let intent = QueryIntent::General {
            query: "magic and adventure".to_string(),
        };

        let strategy = get_search_strategy(&intent);

        assert!(strategy.hybrid_search);
        assert_eq!(strategy.semantic_weight, 0.8);
        assert!(strategy.metadata_filter.is_none());
    }
}
