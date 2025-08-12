use crate::error::Result;
use crate::{
    error::ApiError,
    ml::universal_sentence_encoder::UniversalSentenceEncoder,
    models::Book,
    services::{pinecone::Pinecone, supabase::SupabaseClient},
};
use regex::Regex;
use serde::Serialize;
use std::{collections::HashSet, sync::LazyLock};
use tracing::{debug, info};

static AUTHOR_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)(?:books?\s+)?(?:written\s+)?by\s+([a-zA-Z\s.'-]+)").unwrap(),
        Regex::new(r"(?i)(?:works?\s+)?(?:of|from)\s+([a-zA-Z\s.'-]+)").unwrap(),
        Regex::new(r"(?i)([a-zA-Z\s.'-]+)'s\s+books?").unwrap(),
        Regex::new(r"(?i)author:?\s*([a-zA-Z\s.'-]+)").unwrap(),
    ]
});

static GENRE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(
            r"(?i)(?:genre:?\s*)?(?:books?\s+in\s+)?([a-zA-Z\s&-]+?)\s+(?:books?|novels?|genre)",
        )
        .unwrap(),
        Regex::new(
            r"(?i)(?:recommend\s+)?([a-zA-Z\s&-]+?)\s+(?:books?|novels?|fiction|non-fiction)",
        )
        .unwrap(),
    ]
});

static SIMILAR_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)(?:books?\s+)?(?:similar\s+to|like)\s+(.+)").unwrap(),
        Regex::new(r"(?i)(?:more\s+books?\s+like)\s+(.+)").unwrap(),
    ]
});

static COMMON_GENRES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "fiction",
        "non-fiction",
        "mystery",
        "romance",
        "fantasy",
        "sci-fi",
        "science fiction",
        "biography",
        "history",
        "self-help",
        "business",
        "philosophy",
        "poetry",
        "drama",
        "thriller",
        "horror",
        "young adult",
        "children",
    ]
    .into()
});

#[derive(Debug, Clone)]
enum QueryIntent {
    Author {
        name: String,
        original_query: String,
    },
    Genre {
        genre: String,
        original_query: String,
    },
    SimilarTo {
        reference: String,
        original_query: String,
    },
    General {
        query: String,
    },
}

#[derive(Debug)]
struct SearchStrategy {
    metadata_filter: Option<MetadataFilter>,
    semantic_weight: f32,
    hybrid_search: bool,
}

#[derive(Debug, Serialize)]
struct MetadataFilter {
    field: String,
    value: String,
    exact_match: bool,
}

#[derive(Clone)]
pub struct RecommendationService {
    sentence_encoder: UniversalSentenceEncoder,
    pinecone: Pinecone,
    supabase: SupabaseClient,
}

impl RecommendationService {
    pub fn new(
        sentence_encoder: UniversalSentenceEncoder,
        pinecone: Pinecone,
        supabase: SupabaseClient,
    ) -> Self {
        Self {
            sentence_encoder,
            pinecone,
            supabase,
        }
    }

    pub async fn get_recommendations(&self, query: &str, top_k: usize) -> Result<Vec<Book>> {
        if query.trim().is_empty() {
            return Err(ApiError::InvalidInput("Query cannot be empty".into()));
        }

        // Parse query intent
        let intent = self.parse_query_intent(query);
        debug!(?intent, "Detected query intent");

        // Get search strategy
        let strategy = self.get_search_strategy(&intent);
        debug!(?strategy, "Using search strategy");

        // Perform hybrid search
        let raw_results = self
            .perform_hybrid_search(&intent, &strategy, top_k)
            .await?;
        debug!(result_count = raw_results.len(), "Got raw search results");

        // Rank and process results
        let ranked_results = self.rank_results(raw_results, &intent, top_k);
        info!(
            final_count = ranked_results.len(),
            "Returning ranked results"
        );

        Ok(ranked_results)
    }

    fn parse_query_intent(&self, query: &str) -> QueryIntent {
        // Try to match author patterns
        for pattern in AUTHOR_PATTERNS.iter() {
            if let Some(cap) = pattern.captures(query) {
                let author = cap[1]
                    .trim()
                    .replace(|c: char| !c.is_alphanumeric() && c != ' ', "");
                return QueryIntent::Author {
                    name: author,
                    original_query: query.to_string(),
                };
            }
        }

        // Try to match genre patterns
        for pattern in GENRE_PATTERNS.iter() {
            if let Some(cap) = pattern.captures(query) {
                let potential_genre = cap[1].trim().to_lowercase();
                if COMMON_GENRES.iter().any(|genre| {
                    potential_genre.contains(*genre) || genre.contains(&potential_genre)
                }) {
                    return QueryIntent::Genre {
                        genre: potential_genre,
                        original_query: query.to_string(),
                    };
                }
            }
        }

        // Try to match similar-to patterns
        for pattern in SIMILAR_PATTERNS.iter() {
            if let Some(cap) = pattern.captures(query) {
                return QueryIntent::SimilarTo {
                    reference: cap[1].trim().to_string(),
                    original_query: query.to_string(),
                };
            }
        }

        // Default to general search
        QueryIntent::General {
            query: query.to_string(),
        }
    }

    fn get_search_strategy(&self, intent: &QueryIntent) -> SearchStrategy {
        match intent {
            QueryIntent::Author { name, .. } => SearchStrategy {
                metadata_filter: Some(MetadataFilter {
                    field: "author".into(),
                    value: name.clone(),
                    exact_match: false,
                }),
                semantic_weight: 0.3,
                hybrid_search: true,
            },
            QueryIntent::Genre { genre, .. } => SearchStrategy {
                metadata_filter: Some(MetadataFilter {
                    field: "categories".into(),
                    value: genre.clone(),
                    exact_match: false,
                }),
                semantic_weight: 0.7,
                hybrid_search: true,
            },
            QueryIntent::SimilarTo { .. } | QueryIntent::General { .. } => SearchStrategy {
                metadata_filter: None,
                semantic_weight: 1.0,
                hybrid_search: false,
            },
        }
    }

    async fn perform_hybrid_search(
        &self,
        intent: &QueryIntent,
        strategy: &SearchStrategy,
        top_k: usize,
    ) -> Result<Vec<Book>> {
        let mut results = Vec::new();

        // Try metadata filtering if applicable
        if let Some(filter) = &strategy.metadata_filter {
            // Try exact match first
            if filter.exact_match {
                let exact_matches = self
                    .pinecone
                    .query_metadata(&filter.field, &filter.value, true, top_k * 2)
                    .await?;
                results.extend(exact_matches);
            }

            // If we need more results, try partial matching
            if results.len() < top_k {
                let partial_matches = self
                    .pinecone
                    .query_metadata(&filter.field, &filter.value, false, top_k * 2)
                    .await?;

                // Add only new results
                let existing_ids: HashSet<_> = results.iter().map(|r| r.id.clone()).collect();
                results.extend(
                    partial_matches
                        .into_iter()
                        .filter(|book| !existing_ids.contains(&book.id)),
                );
            }
        }

        // Perform semantic search if needed
        if results.len() < top_k || strategy.hybrid_search {
            let query_text = match intent {
                QueryIntent::Author { original_query, .. }
                | QueryIntent::Genre { original_query, .. }
                | QueryIntent::SimilarTo { original_query, .. } => original_query,
                QueryIntent::General { query } => query,
            };

            let embedding = self.sentence_encoder.encode(query_text).await?;
            let semantic_results = self
                .pinecone
                .query_vector(embedding.as_slice().unwrap(), top_k * 2)
                .await?;

            if strategy.hybrid_search {
                // Weight semantic results
                for result in &mut results {
                    result.rating *= strategy.semantic_weight;
                }
            }

            // Add new semantic results
            let existing_ids: HashSet<_> = results.iter().map(|r| r.id.clone()).collect();
            results.extend(
                semantic_results
                    .into_iter()
                    .filter(|book| !existing_ids.contains(&book.id)),
            );
        }

        Ok(results)
    }

    fn rank_results(
        &self,
        mut results: Vec<Book>,
        intent: &QueryIntent,
        top_k: usize,
    ) -> Vec<Book> {
        match intent {
            QueryIntent::Author { name, .. } => {
                let name_lower = name.to_lowercase();
                results.sort_by(|a, b| {
                    let a_author = a.author.to_lowercase();
                    let b_author = b.author.to_lowercase();

                    let a_exact = a_author.contains(&name_lower) as i32;
                    let b_exact = b_author.contains(&name_lower) as i32;

                    b_exact.cmp(&a_exact).then_with(|| {
                        b.rating
                            .partial_cmp(&a.rating)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                });
            }
            QueryIntent::Genre { genre, .. } => {
                let genre_lower = genre.to_lowercase();
                results.sort_by(|a, b| {
                    let a_categories = a.categories.join(", ").to_lowercase();
                    let b_categories = b.categories.join(", ").to_lowercase();

                    let a_match = a_categories.contains(&genre_lower) as i32;
                    let b_match = b_categories.contains(&genre_lower) as i32;

                    b_match.cmp(&a_match).then_with(|| {
                        b.rating
                            .partial_cmp(&a.rating)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                });
            }
            _ => {
                results.sort_by(|a, b| {
                    b.rating
                        .partial_cmp(&a.rating)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        // Remove duplicates and limit results
        let mut seen = HashSet::new();
        results
            .into_iter()
            .filter(|book| {
                let key = format!("{}-{}", &book.title, &book.author);
                seen.insert(key)
            })
            .take(top_k)
            .collect()
    }
}
