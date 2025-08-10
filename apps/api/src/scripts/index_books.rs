use crate::{
    config::Config,
    error::Result,
    ml::sentence_encoder::SentenceEncoder,
    models::Book,
    services::pinecone::{PineconeClient, Vector},
};
use csv::ReaderBuilder;
use futures::StreamExt;
use log::{error, info};
use serde::Deserialize;
use std::{fs::File, path::Path};
use tokio::time::{sleep, Duration};
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Deserialize)]
struct BookCsvRecord {
    #[serde(alias = "Title")]
    title: Option<String>,
    #[serde(alias = "Authors", alias = "Author")]
    authors: Option<String>,
    #[serde(alias = "Description")]
    description: Option<String>,
    #[serde(alias = "Categories")]
    categories: Option<String>,
    isbn13: Option<String>,
    published_year: Option<String>,
    ratings_count: Option<String>,
    #[serde(alias = "average_rating", alias = "rating")]
    rating: Option<String>,
    #[serde(alias = "image_url", alias = "thumbnail")]
    thumbnail: Option<String>,
}

/// Normalize author names for better matching
fn normalize_author(author: &str) -> String {
    author
        .trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .nfkc()
        .collect::<String>()
        .to_lowercase()
}

/// Extract and normalize categories
fn normalize_categories(categories: &str) -> String {
    categories
        .trim()
        .to_lowercase()
        .replace(&['&', '|', ';'][..], ",")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Create a rich text representation for better semantic search
fn create_searchable_text(book: &Book) -> String {
    let mut parts = Vec::new();

    if let Some(title) = &book.title {
        parts.push(format!("Title: {}", title));
    }
    if let Some(author) = &book.author {
        parts.push(format!("Author: {}", author));
    }
    if let Some(categories) = &book.categories {
        parts.push(format!("Categories: {}", categories));
    }
    if let Some(description) = &book.description {
        parts.push(format!("Description: {}", description));
    }

    parts.join(". ")
}

/// Retry operation with exponential backoff
async fn retry_with_backoff<F, T, E>(
    operation: F,
    max_retries: u32,
    base_delay_ms: u64,
) -> Result<T>
where
    F: Fn() -> futures::future::BoxFuture<'_, std::result::Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempt += 1;
                if attempt >= max_retries {
                    return Err(anyhow::anyhow!(
                        "Operation failed after {} attempts: {}",
                        max_retries,
                        e
                    ));
                }
                let delay = base_delay_ms * 2u64.pow(attempt - 1);
                error!("Attempt {} failed, retrying in {}ms: {}", attempt, delay, e);
                sleep(Duration::from_millis(delay)).await;
            }
        }
    }
}

pub async fn index_books(config: &Config, csv_path: &Path) -> Result<()> {
    info!("Initializing sentence encoder...");
    let encoder = SentenceEncoder::new(&config.hugging_face_api_key);

    info!("Initializing Pinecone client...");
    let pinecone = PineconeClient::new(&config.pinecone_api_key, &config.pinecone_index_name);

    info!("Reading CSV file: {}", csv_path.display());
    let file = File::open(csv_path)?;
    let mut rdr = ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(file);

    let mut books = Vec::new();
    let mut record_count = 0;

    for result in rdr.deserialize() {
        record_count += 1;
        let record: BookCsvRecord = match result {
            Ok(record) => record,
            Err(e) => {
                error!("Error parsing record {}: {}", record_count, e);
                continue;
            }
        };

        // Skip records without title or author
        let (Some(title), Some(authors)) = (record.title, record.authors) else {
            continue;
        };

        let book = Book {
            isbn13: record.isbn13,
            title: Some(title.clone()),
            author: Some(authors.clone()),
            normalized_author: Some(normalize_author(&authors)),
            description: record.description,
            categories: record.categories.as_deref().map(normalize_categories),
            published_year: record.published_year.and_then(|y| y.parse().ok()),
            ratings_count: record.ratings_count.and_then(|c| c.parse().ok()),
            rating: record.rating.and_then(|r| r.parse().ok()),
            thumbnail: record.thumbnail,
        };

        books.push(book);
    }

    info!("Parsed {} books", books.len());
    if books.is_empty() {
        error!("No valid books found in CSV file");
        return Ok(());
    }

    // Process books in batches
    let batch_size = 50;
    for (batch_index, batch) in books.chunks(batch_size).enumerate() {
        info!("Processing batch {}", batch_index + 1);

        // Create searchable text for each book
        let texts: Vec<String> = batch
            .iter()
            .map(|book| create_searchable_text(book))
            .collect();

        // Generate embeddings for the batch
        let embeddings = match encoder.encode_batch(&texts).await {
            Ok(emb) => emb,
            Err(e) => {
                error!(
                    "Failed to generate embeddings for batch {}: {}",
                    batch_index + 1,
                    e
                );
                continue;
            }
        };

        // Create vectors for Pinecone
        let vectors: Vec<Vector> = batch
            .iter()
            .zip(embeddings.iter())
            .map(|(book, embedding)| {
                let id = book.isbn13.clone().unwrap_or_else(|| {
                    format!(
                        "{}-{}",
                        book.title.as_ref().unwrap(),
                        book.author.as_ref().unwrap()
                    )
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '-')
                    .collect()
                });

                Vector {
                    id,
                    values: embedding.clone(),
                    metadata: serde_json::to_value(book).unwrap_or_default(),
                }
            })
            .collect();

        // Upsert vectors with retry
        match retry_with_backoff(|| Box::pin(pinecone.upsert(vectors.clone())), 3, 1000).await {
            Ok(_) => info!(
                "✅ Successfully indexed batch {} ({} books)",
                batch_index + 1,
                vectors.len()
            ),
            Err(e) => error!("❌ Failed to index batch {}: {}", batch_index + 1, e),
        }
    }

    // Log statistics
    let unique_authors = books
        .iter()
        .filter_map(|b| b.normalized_author.as_ref())
        .collect::<std::collections::HashSet<_>>()
        .len();

    let unique_categories = books
        .iter()
        .filter_map(|b| b.categories.as_ref())
        .flat_map(|cats| cats.split(", "))
        .collect::<std::collections::HashSet<_>>()
        .len();

    info!("📊 Indexing Complete");
    info!("   Total books: {}", books.len());
    info!("   Unique authors: {}", unique_authors);
    info!("   Unique categories: {}", unique_categories);

    Ok(())
}
