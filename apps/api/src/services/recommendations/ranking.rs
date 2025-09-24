//! Result ranking implementation for the recommendation service.
//!
//! This module provides algorithms and utilities for ranking book recommendations
//! based on various factors such as relevance, popularity, and metadata.

use chrono::Datelike;
use log::{debug, info};
use std::cmp::Ordering;

use crate::error::ApiError;
use crate::models::{Book, BookRecommendation};
use crate::services::recommendations::intent::QueryIntent;
use crate::services::recommendations::search::SearchStrategy;

/// Factors that influence the ranking of recommendation results
#[derive(Debug, Clone)]
pub struct RankingFactors {
    /// Weight for semantic similarity (0.0-1.0)
    pub semantic_weight: f32,
    /// Weight for popularity metrics like ratings (0.0-1.0)
    pub popularity_weight: f32,
    /// Weight for recency of publication (0.0-1.0)
    pub recency_weight: f32,
    /// Weight for exact metadata matches (0.0-1.0)
    pub metadata_match_weight: f32,
    /// Minimum rating threshold to include in results
    pub min_rating_threshold: f32,
    /// Minimum ratings count to consider ratings reliable
    pub min_ratings_count: i32,
}

impl Default for RankingFactors {
    fn default() -> Self {
        Self {
            semantic_weight: 0.6,
            popularity_weight: 0.2,
            recency_weight: 0.1,
            metadata_match_weight: 0.1,
            min_rating_threshold: 3.5,
            min_ratings_count: 10,
        }
    }
}

/// Get appropriate ranking factors based on query intent
pub fn get_ranking_factors(intent: &QueryIntent) -> RankingFactors {
    match intent {
        QueryIntent::Author { .. } => RankingFactors {
            semantic_weight: 0.4,
            popularity_weight: 0.3,
            recency_weight: 0.1,
            metadata_match_weight: 0.2, // Higher weight for author matches
            ..Default::default()
        },
        QueryIntent::Genre { .. } => RankingFactors {
            semantic_weight: 0.5,
            popularity_weight: 0.3,
            recency_weight: 0.1,
            metadata_match_weight: 0.1,
            ..Default::default()
        },
        QueryIntent::SimilarTo { .. } => RankingFactors {
            semantic_weight: 0.7, // Highest semantic weight for similarity searches
            popularity_weight: 0.2,
            recency_weight: 0.05,
            metadata_match_weight: 0.05,
            ..Default::default()
        },
        QueryIntent::General { .. } => RankingFactors::default(),
    }
}

/// Rank book results based on various factors
pub fn rank_results(
    raw_results: Vec<(Book, f32)>, // (Book, similarity_score)
    intent: &QueryIntent,
    _strategy: &SearchStrategy,
    max_results: usize,
) -> Result<Vec<BookRecommendation>, ApiError> {
    if raw_results.is_empty() {
        return Ok(Vec::new());
    }

    // Get ranking factors based on query intent
    let factors = get_ranking_factors(intent);

    info!(
        "Ranking {} results with factors: {:?}",
        raw_results.len(),
        factors
    );

    // Pre-process and score each result
    let mut scored_results: Vec<(Book, f32, f32, f32, f32, f32)> =
        Vec::with_capacity(raw_results.len());
    let current_year = chrono::Utc::now().year();

    for (book, similarity_score) in raw_results {
        // Calculate individual scores
        let popularity_score = calculate_popularity_score(&book, factors.min_ratings_count);
        let recency_score = calculate_recency_score(&book, current_year);
        let metadata_match_score = calculate_metadata_match_score(&book, intent);

        // Apply minimum rating threshold if book has sufficient ratings
        if let Some(count) = book.ratings_count {
            if count >= factors.min_ratings_count && book.rating < factors.min_rating_threshold {
                debug!(
                    "Filtering out book '{}' with low rating: {}",
                    book.title.as_deref().unwrap_or("Unknown"),
                    book.rating
                );
                continue;
            }
        }

        // Calculate combined score with weights
        let combined_score = (similarity_score * factors.semantic_weight)
            + (popularity_score * factors.popularity_weight)
            + (recency_score * factors.recency_weight)
            + (metadata_match_score * factors.metadata_match_weight);

        // Add to scored results
        scored_results.push((
            book,
            combined_score,
            similarity_score,
            popularity_score,
            recency_score,
            metadata_match_score,
        ));
    }

    // Sort by combined score (descending)
    scored_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

    // Take top results
    let top_results = scored_results
        .iter()
        .take(max_results)
        .enumerate()
        .map(|(i, (book, combined_score, _similarity_score, _, _, _))| {
            // Create a BookRecommendation with a unique ID
            BookRecommendation {
                id: format!("rec_{}", i),
                book: book.clone(),
                similarity_score: *combined_score,
            }
        })
        .collect::<Vec<_>>();

    info!(
        "Ranked {} results down to {} recommendations",
        scored_results.len(),
        top_results.len()
    );

    Ok(top_results)
}

/// Calculate a popularity score based on rating and ratings count
fn calculate_popularity_score(book: &Book, min_ratings_count: i32) -> f32 {
    let base_score = book.rating / 5.0; // Normalize to 0.0-1.0 range

    // Apply a logarithmic boost for popular books
    if let Some(count) = book.ratings_count {
        if count >= min_ratings_count {
            // Logarithmic scale for ratings count (diminishing returns)
            let count_factor = (count as f32).log10() / 4.0; // Normalize
            return base_score * (1.0 + count_factor.min(1.0));
        }
    }

    base_score
}

/// Calculate a recency score based on publication year
fn calculate_recency_score(book: &Book, current_year: i32) -> f32 {
    if let Some(year) = book.year {
        if year > 0 && year <= current_year {
            // Linear scaling - newer books get higher scores
            // Books from 50+ years ago get minimum score
            let years_old = (current_year - year) as f32;
            let recency = 1.0 - (years_old / 50.0).min(1.0);
            return recency;
        }
    }

    // Default recency score for books without a year
    0.5
}

/// Calculate a metadata match score based on how well the book matches the intent
fn calculate_metadata_match_score(book: &Book, intent: &QueryIntent) -> f32 {
    match intent {
        QueryIntent::Author { name, .. } => {
            if let Some(author) = &book.author {
                // Score based on how closely the author matches
                let author_lower = author.to_lowercase();
                let name_lower = name.to_lowercase();

                if author_lower == name_lower {
                    return 1.0; // Exact match
                } else if author_lower.contains(&name_lower) || name_lower.contains(&author_lower) {
                    return 0.8; // Partial match
                }
            }
            0.0 // No match
        }
        QueryIntent::Genre { genre, .. } => {
            let genre_lower = genre.to_lowercase();

            // Check if any of the book's categories match the genre
            let category_match = book.categories.iter().any(|category| {
                let category_lower = category.to_lowercase();
                if category_lower == genre_lower {
                    return true;
                }
                if category_lower.contains(&genre_lower) || genre_lower.contains(&category_lower) {
                    return true;
                }
                false
            });

            if category_match {
                return 1.0;
            }

            // Check if the genre appears in the book title or description
            if let Some(title) = &book.title {
                if title.to_lowercase().contains(&genre_lower) {
                    return 0.8;
                }
            }

            if let Some(description) = &book.description {
                if description.to_lowercase().contains(&genre_lower) {
                    return 0.6;
                }
            }

            0.0
        }
        QueryIntent::SimilarTo { original_query, .. } => {
            // For similarity searches, we might look for keywords in the title/description
            let lowercase_query = original_query.to_lowercase();
            let query_terms: Vec<&str> = lowercase_query
                .split_whitespace()
                .filter(|&term| term.len() > 3) // Skip short words
                .collect();

            let mut match_score: f32 = 0.0;

            if let Some(title) = &book.title {
                let title_lower = title.to_lowercase();
                for term in &query_terms {
                    if title_lower.contains(term) {
                        match_score += 0.2;
                    }
                }
            }

            if let Some(description) = &book.description {
                let desc_lower = description.to_lowercase();
                for term in &query_terms {
                    if desc_lower.contains(term) {
                        match_score += 0.1;
                    }
                }
            }

            match_score.min(1.0)
        }
        QueryIntent::General { query } => {
            // For general searches, do a more basic keyword matching
            let query_lower = query.to_lowercase();
            let mut match_score: f32 = 0.0;

            if let Some(title) = &book.title {
                if title.to_lowercase().contains(&query_lower) {
                    match_score += 0.4;
                }
            }

            if let Some(description) = &book.description {
                if description.to_lowercase().contains(&query_lower) {
                    match_score += 0.2;
                }
            }

            for category in &book.categories {
                if category.to_lowercase().contains(&query_lower) {
                    match_score += 0.3;
                    break;
                }
            }

            match_score.min(1.0)
        }
    }
}

// Diversity boosting to ensure a variety of recommendations
pub fn apply_diversity_boost(recommendations: &mut Vec<BookRecommendation>, diversity_factor: f32) {
    if recommendations.len() <= 3 {
        return; // Not enough items to diversify
    }

    // Track authors we've already seen
    let mut seen_authors = std::collections::HashSet::new();

    for i in 0..recommendations.len() {
        if let Some(author) = &recommendations[i].book.author {
            // If we've seen this author before, apply a diversity penalty
            if !seen_authors.insert(author.to_lowercase()) {
                // Apply a penalty by reducing the score
                recommendations[i].similarity_score *= 1.0 - diversity_factor;
            }
        }
    }

    // Re-sort after applying diversity boosting
    recommendations.sort_by(|a, b| {
        b.similarity_score
            .partial_cmp(&a.similarity_score)
            .unwrap_or(Ordering::Equal)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_book(
        title: &str,
        author: &str,
        rating: f32,
        year: i32,
        ratings_count: i32,
    ) -> Book {
        Book {
            id: Some(Uuid::new_v4().to_string()),
            title: Some(title.to_string()),
            author: Some(author.to_string()),
            description: Some(format!("Description of {}", title)),
            categories: vec!["Fiction".to_string(), "Fantasy".to_string()],
            thumbnail: None,
            rating,
            year: Some(year),
            isbn: None,
            page_count: Some(300),
            ratings_count: Some(ratings_count),
            language: Some("English".to_string()),
            publisher: Some("Test Publisher".to_string()),
        }
    }

    #[test]
    fn test_ranking_factors_by_intent() {
        let author_intent = QueryIntent::Author {
            name: "J.K. Rowling".to_string(),
            original_query: "books by J.K. Rowling".to_string(),
        };

        let genre_intent = QueryIntent::Genre {
            genre: "fantasy".to_string(),
            original_query: "fantasy books".to_string(),
        };

        let similar_intent = QueryIntent::SimilarTo {
            original_query: "books like Harry Potter".to_string(),
        };

        let general_intent = QueryIntent::General {
            query: "magic".to_string(),
        };

        let author_factors = get_ranking_factors(&author_intent);
        let genre_factors = get_ranking_factors(&genre_intent);
        let similar_factors = get_ranking_factors(&similar_intent);
        let general_factors = get_ranking_factors(&general_intent);

        // Check that factors are different based on intent
        assert_ne!(
            author_factors.semantic_weight,
            similar_factors.semantic_weight
        );
        assert_ne!(
            genre_factors.metadata_match_weight,
            general_factors.metadata_match_weight
        );

        // Similarity intent should have highest semantic weight
        assert!(similar_factors.semantic_weight > author_factors.semantic_weight);
        assert!(similar_factors.semantic_weight > genre_factors.semantic_weight);
        assert!(similar_factors.semantic_weight > general_factors.semantic_weight);

        // Author intent should have higher metadata weight
        assert!(author_factors.metadata_match_weight > similar_factors.metadata_match_weight);
    }

    #[test]
    fn test_calculate_popularity_score() {
        let popular_book = create_test_book("Popular", "Author", 4.5, 2020, 10000);
        let average_book = create_test_book("Average", "Author", 3.5, 2020, 100);
        let unpopular_book = create_test_book("Unpopular", "Author", 4.0, 2020, 5);

        let popular_score = calculate_popularity_score(&popular_book, 10);
        let average_score = calculate_popularity_score(&average_book, 10);
        let unpopular_score = calculate_popularity_score(&unpopular_book, 10);

        // Higher rating and more ratings should result in higher score
        assert!(popular_score > average_score);

        // Book with few ratings should have lower score even with good rating
        assert!(average_score > unpopular_score);
    }

    #[test]
    fn test_calculate_recency_score() {
        let current_year = 2023;
        let new_book = create_test_book("New", "Author", 4.0, 2022, 100);
        let old_book = create_test_book("Old", "Author", 4.0, 1970, 100);
        let very_old_book = create_test_book("Classic", "Author", 4.0, 1900, 100);

        let new_score = calculate_recency_score(&new_book, current_year);
        let old_score = calculate_recency_score(&old_book, current_year);
        let very_old_score = calculate_recency_score(&very_old_book, current_year);

        // Newer books should have higher scores
        assert!(new_score > old_score);
        assert!(old_score > very_old_score);

        // Very old books should have minimum score
        assert_eq!(very_old_score, 0.0);
    }
}
