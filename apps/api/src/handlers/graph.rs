use crate::{
    error::ApiError,
    services::neo4j::{GraphResponse, GraphStats, Neo4jClient},
};
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct GraphQueryParams {
    pub book_id: String,
    #[serde(default = "default_depth")]
    pub depth: usize,
}

fn default_depth() -> usize {
    2
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SearchQueryParams {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SimilarBooksResponse {
    pub books: Vec<crate::services::neo4j::BookNode>,
}

/// Get the graph neighborhood for a specific book
#[utoipa::path(
    get,
    path = "/api/graph/book",
    tag = "Graph",
    params(
        ("book_id" = String, Query, description = "Book ID to get graph for"),
        ("depth" = Option<usize>, Query, description = "Graph traversal depth (default: 2)")
    ),
    responses(
        (status = 200, description = "Successfully retrieved book graph", body = GraphResponse),
        (status = 404, description = "Book not found"),
        (status = 500, description = "Internal server error")
    ),
    summary = "Get book relationship graph",
    description = "Returns a graph of related books including nodes and relationships up to the specified depth"
)]
#[actix_web::get("/book")]
pub async fn get_book_graph(
    params: web::Query<GraphQueryParams>,
    neo4j: web::Data<Neo4jClient>,
) -> Result<HttpResponse, ApiError> {
    let graph = neo4j.get_book_graph(&params.book_id, params.depth).await?;

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
        ("book_id" = String, Query, description = "Book ID to find similar books for"),
        ("limit" = Option<usize>, Query, description = "Maximum number of results (default: 20)")
    ),
    responses(
        (status = 200, description = "Successfully retrieved similar books", body = SimilarBooksResponse),
        (status = 500, description = "Internal server error")
    ),
    summary = "Get similar books",
    description = "Returns books that are semantically similar to the specified book"
)]
#[actix_web::get("/similar")]
pub async fn get_similar_books(
    params: web::Query<SearchQueryParams>,
    neo4j: web::Data<Neo4jClient>,
) -> Result<HttpResponse, ApiError> {
    let books = neo4j.get_similar_books(&params.query, params.limit).await?;
    Ok(HttpResponse::Ok().json(SimilarBooksResponse { books }))
}

/// Search for books by title
#[utoipa::path(
    get,
    path = "/api/graph/search",
    tag = "Graph",
    params(
        ("query" = String, Query, description = "Search query for book titles"),
        ("limit" = Option<usize>, Query, description = "Maximum number of results (default: 20)")
    ),
    responses(
        (status = 200, description = "Successfully retrieved search results", body = SimilarBooksResponse),
        (status = 500, description = "Internal server error")
    ),
    summary = "Search books",
    description = "Search for books by title pattern"
)]
#[actix_web::get("/search")]
pub async fn search_books(
    params: web::Query<SearchQueryParams>,
    neo4j: web::Data<Neo4jClient>,
) -> Result<HttpResponse, ApiError> {
    let books = neo4j.search_books(&params.query, params.limit).await?;
    Ok(HttpResponse::Ok().json(SimilarBooksResponse { books }))
}

/// Get graph statistics
#[utoipa::path(
    get,
    path = "/api/graph/stats",
    tag = "Graph",
    responses(
        (status = 200, description = "Successfully retrieved graph statistics", body = GraphStats),
        (status = 500, description = "Internal server error")
    ),
    summary = "Get graph statistics",
    description = "Returns statistics about the book graph including total nodes and relationships"
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
