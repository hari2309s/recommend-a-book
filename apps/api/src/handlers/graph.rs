use crate::{
    error::ApiError,
    services::neo4j::{GraphResponse, GraphStats, Neo4jClient},
};
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct GraphQueryParams {
    /// The ID of the book to get the graph for
    #[schema(example = "book-123")]
    pub book_id: String,
    /// The depth of graph traversal (default: 2, max: 5)
    #[serde(default = "default_depth")]
    #[schema(example = 2, minimum = 1, maximum = 5)]
    pub depth: usize,
}

fn default_depth() -> usize {
    2
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SearchQueryParams {
    /// The search query for book titles or IDs
    #[schema(example = "The Hobbit")]
    pub query: String,
    /// Maximum number of results to return (default: 20, max: 100)
    #[serde(default = "default_limit")]
    #[schema(example = 20, minimum = 1, maximum = 100)]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SimilarBooksResponse {
    /// List of books similar to the queried book
    pub books: Vec<crate::services::neo4j::BookNode>,
}

/// Get the graph neighborhood for a specific book
#[utoipa::path(
    get,
    path = "/api/graph/book",
    tag = "Graph",
    params(
        ("book_id" = String, Query, description = "Book ID to get graph for", example = "book-123"),
        ("depth" = Option<usize>, Query, description = "Graph traversal depth (default: 2, max: 5)", example = 2)
    ),
    responses(
        (status = 200, description = "Successfully retrieved book graph with nodes and relationships", body = GraphResponse,
            example = json!({
                "nodes": [
                    {
                        "id": "book-123",
                        "title": "The Hobbit",
                        "author": "J.R.R. Tolkien",
                        "categories": ["Fantasy", "Adventure"],
                        "rating": 4.5,
                        "year": 1937,
                        "description": "A fantasy adventure..."
                    }
                ],
                "relationships": [
                    {
                        "from_id": "book-123",
                        "to_id": "book-456",
                        "relation_type": "SIMILAR_TO",
                        "weight": 0.92
                    }
                ]
            })
        ),
        (status = 404, description = "Book not found"),
        (status = 500, description = "Internal server error")
    ),
    summary = "Get book relationship graph",
    description = "Returns a graph of related books including nodes and relationships up to the specified depth. \
                   Each node represents a book with its metadata, and relationships show connections like SIMILAR_TO, \
                   SAME_AUTHOR, SAME_GENRE, etc. The weight indicates the strength of the relationship."
)]
#[actix_web::get("/book")]
pub async fn get_book_graph(
    params: web::Query<GraphQueryParams>,
    neo4j: web::Data<Neo4jClient>,
) -> Result<HttpResponse, ApiError> {
    let depth = params.depth.min(5); // Cap at 5 for performance
    let graph = neo4j.get_book_graph(&params.book_id, depth).await?;

    if graph.nodes.is_empty() {
        return Err(ApiError::NotFound(format!(
            "Book with ID {} not found",
            params.book_id
        )));
    }

    Ok(HttpResponse::Ok().json(graph))
}

/// Get books similar to a specific book
#[utoipa::path(
    get,
    path = "/api/graph/similar",
    tag = "Graph",
    params(
        ("book_id" = String, Query, description = "Book ID to find similar books for", example = "book-123"),
        ("limit" = Option<usize>, Query, description = "Maximum number of results (default: 20, max: 100)", example = 20)
    ),
    responses(
        (status = 200, description = "Successfully retrieved similar books", body = SimilarBooksResponse,
            example = json!({
                "books": [
                    {
                        "id": "book-456",
                        "title": "The Lord of the Rings",
                        "author": "J.R.R. Tolkien",
                        "categories": ["Fantasy", "Epic"],
                        "rating": 4.6,
                        "year": 1954,
                        "description": "An epic fantasy trilogy..."
                    }
                ]
            })
        ),
        (status = 500, description = "Internal server error")
    ),
    summary = "Get similar books",
    description = "Returns books that are semantically similar to the specified book based on embeddings, \
                   genre, author, and other factors. Results are ordered by similarity score."
)]
#[actix_web::get("/similar")]
pub async fn get_similar_books(
    params: web::Query<SearchQueryParams>,
    neo4j: web::Data<Neo4jClient>,
) -> Result<HttpResponse, ApiError> {
    let limit = params.limit.min(100); // Cap at 100 for performance
    let books = neo4j.get_similar_books(&params.query, limit).await?;
    Ok(HttpResponse::Ok().json(SimilarBooksResponse { books }))
}

/// Search for books by title
#[utoipa::path(
    get,
    path = "/api/graph/search",
    tag = "Graph",
    params(
        ("query" = String, Query, description = "Search query for book titles", example = "Hobbit"),
        ("limit" = Option<usize>, Query, description = "Maximum number of results (default: 20, max: 100)", example = 20)
    ),
    responses(
        (status = 200, description = "Successfully retrieved search results", body = SimilarBooksResponse,
            example = json!({
                "books": [
                    {
                        "id": "book-123",
                        "title": "The Hobbit",
                        "author": "J.R.R. Tolkien",
                        "categories": ["Fantasy", "Adventure"],
                        "rating": 4.5,
                        "year": 1937,
                        "description": "A fantasy adventure..."
                    }
                ]
            })
        ),
        (status = 500, description = "Internal server error")
    ),
    summary = "Search books by title",
    description = "Search for books by title pattern using case-insensitive matching. \
                   Results are ordered by rating."
)]
#[actix_web::get("/search")]
pub async fn search_books(
    params: web::Query<SearchQueryParams>,
    neo4j: web::Data<Neo4jClient>,
) -> Result<HttpResponse, ApiError> {
    let limit = params.limit.min(100); // Cap at 100 for performance
    let books = neo4j.search_books(&params.query, limit).await?;
    Ok(HttpResponse::Ok().json(SimilarBooksResponse { books }))
}

/// Get graph statistics
#[utoipa::path(
    get,
    path = "/api/graph/stats",
    tag = "Graph",
    responses(
        (status = 200, description = "Successfully retrieved graph statistics", body = GraphStats,
            example = json!({
                "total_books": 1000,
                "total_relationships": 5000
            })
        ),
        (status = 500, description = "Internal server error")
    ),
    summary = "Get graph statistics",
    description = "Returns statistics about the book graph including total number of nodes (books) \
                   and total number of relationships between books."
)]
#[actix_web::get("/stats")]
pub async fn get_graph_stats(neo4j: web::Data<Neo4jClient>) -> Result<HttpResponse, ApiError> {
    let stats = neo4j.get_graph_stats().await?;
    Ok(HttpResponse::Ok().json(stats))
}

pub fn graph_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/graph")
            .service(get_book_graph)
            .service(get_similar_books)
            .service(search_books)
            .service(get_graph_stats),
    );
}
