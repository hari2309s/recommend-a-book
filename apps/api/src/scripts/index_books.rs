use crate::{
    config::Config,
    error::Result,
    ml::sentence_encoder::SentenceEncoder,
    models::Book,
    services::pinecone::{PineconeClient, Vector},
};
use csv::ReaderBuilder;
use futures::StreamExt;
use log::{error, info, warn};
use serde::Deserialize;
use std::{collections::HashSet, fs::File, path::Path};
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
/// This should match your TypeScript implementation for consistency
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

/// Validate and clean book data
fn validate_book_data(record: BookCsvRecord) -> Option<Book> {
    // Skip records without title or author
    let title = record.title?.trim().to_string();
    let authors = record.authors?.trim().to_string();

    if title.is_empty() || authors.is_empty() {
        return None;
    }

    let description = record
        .description
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty());

    let categories = record
        .categories
        .as_deref()
        .map(normalize_categories)
        .filter(|c| !c.is_empty());

    let published_year = record.published_year.and_then(|y| y.trim().parse().ok());

    let ratings_count = record.ratings_count.and_then(|c| c.trim().parse().ok());

    let rating = record
        .rating
        .and_then(|r| r.trim().parse::<f64>().ok())
        .filter(|&r| r >= 0.0 && r <= 5.0); // Validate rating range

    let thumbnail = record
        .thumbnail
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty() && (t.starts_with("http://") || t.starts_with("https://")));

    Some(Book {
        isbn13: record
            .isbn13
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        title: Some(title.clone()),
        author: Some(authors.clone()),
        normalized_author: Some(normalize_author(&authors)),
        description,
        categories,
        published_year,
        ratings_count,
        rating,
        thumbnail,
    })
}

/// Generate a consistent book ID
fn generate_book_id(book: &Book) -> String {
    if let Some(isbn) = &book.isbn13 {
        return isbn.clone();
    }

    // Fallback to title-author combination
    let title = book.title.as_deref().unwrap_or("unknown");
    let author = book.author.as_deref().unwrap_or("unknown");

    format!("{}-{}", title, author)
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>()
        .to_lowercase()
}

pub async fn index_books(config: &Config, csv_path: &Path) -> Result<()> {
    info!("🚀 Starting book indexing process...");
    info!("📁 CSV file: {}", csv_path.display());

    // Initialize sentence encoder
    info!("🤖 Initializing sentence encoder...");
    let encoder = SentenceEncoder::new(&config.hugging_face_api_key);

    // Initialize Pinecone client
    info!("📊 Initializing Pinecone client...");
    let pinecone = PineconeClient::new(&config.pinecone_api_key, &config.pinecone_index_name);

    // Read and parse CSV file
    info!("📖 Reading CSV file...");
    let file = File::open(csv_path)?;
    let mut rdr = ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(file);

    let mut books = Vec::new();
    let mut record_count = 0;
    let mut valid_count = 0;
    let mut skipped_count = 0;

    for result in rdr.deserialize() {
        record_count += 1;

        let record: BookCsvRecord = match result {
            Ok(record) => record,
            Err(e) => {
                error!("❌ Error parsing record {}: {}", record_count, e);
                skipped_count += 1;
                continue;
            }
        };

        match validate_book_data(record) {
            Some(book) => {
                books.push(book);
                valid_count += 1;
            }
            None => {
                skipped_count += 1;
            }
        }

        // Log progress every 1000 records
        if record_count % 1000 == 0 {
            info!(
                "📊 Processed {} records, {} valid, {} skipped",
                record_count, valid_count, skipped_count
            );
        }
    }

    info!("✅ CSV parsing complete:");
    info!("   📊 Total records: {}", record_count);
    info!("   ✅ Valid books: {}", valid_count);
    info!("   ❌ Skipped records: {}", skipped_count);

    if books.is_empty() {
        warn!("⚠️  No valid books found in CSV file");
        return Ok(());
    }

    // Remove duplicates based on ISBN or title+author
    let mut seen_ids = HashSet::new();
    books.retain(|book| {
        let id = generate_book_id(book);
        seen_ids.insert(id)
    });

    info!("🔄 After deduplication: {} unique books", books.len());

    // Process books in batches
    let batch_size = 25; // Smaller batches for more reliable processing
    let total_batches = (books.len() + batch_size - 1) / batch_size;
    let mut successful_batches = 0;
    let mut failed_batches = 0;

    info!(
        "🔄 Processing {} books in {} batches of {}",
        books.len(),
        total_batches,
        batch_size
    );

    for (batch_index, batch) in books.chunks(batch_size).enumerate() {
        let batch_num = batch_index + 1;
        info!("🔄 Processing batch {} of {}", batch_num, total_batches);

        // Create searchable text for each book
        let texts: Vec<String> = batch.iter().map(create_searchable_text).collect();

        // Generate embeddings for the batch
        let embeddings = match encoder.encode_batch(&texts).await {
            Ok(emb) => {
                info!("✅ Generated embeddings for batch {}", batch_num);
                emb
            }
            Err(e) => {
                error!(
                    "❌ Failed to generate embeddings for batch {}: {}",
                    batch_num, e
                );
                failed_batches += 1;
                continue;
            }
        };

        // Validate embedding dimensions
        if let Some(first_embedding) = embeddings.first() {
            let expected_dim = 512; // Universal Sentence Encoder
            if first_embedding.len() != expected_dim {
                error!("❌ Embedding dimension mismatch! Expected: {}, Got: {}. Check your model configuration.",
                       expected_dim, first_embedding.len());
                failed_batches += 1;
                continue;
            }
            info!(
                "✅ Embedding dimension validated: {}",
                first_embedding.len()
            );
        }

        // Create vectors for Pinecone
        let vectors: Vec<Vector> = batch
            .iter()
            .zip(embeddings.iter())
            .map(|(book, embedding)| {
                let id = generate_book_id(book);
                Vector {
                    id,
                    values: embedding.clone(),
                    metadata: serde_json::to_value(book).unwrap_or_default(),
                }
            })
            .collect();

        // Upsert vectors with retry
        match retry_with_backoff(|| Box::pin(pinecone.upsert(vectors.clone())), 3, 1000).await {
            Ok(_) => {
                info!(
                    "✅ Successfully indexed batch {} ({} books)",
                    batch_num,
                    vectors.len()
                );
                successful_batches += 1;
            }
            Err(e) => {
                error!(
                    "❌ Failed to index batch {} after retries: {}",
                    batch_num, e
                );
                failed_batches += 1;
            }
        }

        // Add small delay between batches to avoid rate limiting
        if batch_index < total_batches - 1 {
            sleep(Duration::from_millis(500)).await;
        }
    }

    // Calculate and log final statistics
    let unique_authors = books
        .iter()
        .filter_map(|b| b.normalized_author.as_ref())
        .collect::<HashSet<_>>()
        .len();

    let unique_categories: HashSet<&str> = books
        .iter()
        .filter_map(|b| b.categories.as_ref())
        .flat_map(|cats| cats.split(", "))
        .collect();

    let avg_rating = books
        .iter()
        .filter_map(|b| b.rating)
        .fold((0.0, 0), |(sum, count), rating| (sum + rating, count + 1));

    let avg_rating = if avg_rating.1 > 0 {
        avg_rating.0 / avg_rating.1 as f64
    } else {
        0.0
    };

    // Final summary
    info!("🎉 Indexing Complete!");
    info!("================================================");
    info!("📊 Statistics:");
    info!("   📚 Total books indexed: {}", books.len());
    info!("   👥 Unique authors: {}", unique_authors);
    info!("   🏷️  Unique categories: {}", unique_categories.len());
    info!("   ⭐ Average rating: {:.2}", avg_rating);
    info!("   ✅ Successful batches: {}", successful_batches);
    info!("   ❌ Failed batches: {}", failed_batches);
    info!(
        "   📊 Success rate: {:.1}%",
        (successful_batches as f64 / total_batches as f64) * 100.0
    );
    info!("================================================");

    if failed_batches > 0 {
        warn!("⚠️  Some batches failed to index. Consider re-running for complete indexing.");
    }

    Ok(())
}
