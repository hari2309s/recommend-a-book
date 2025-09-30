use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;
use utoipa::ToSchema;

fn deserialize_categories<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        String(String),
        Vec(Vec<String>),
    }

    match StringOrVec::deserialize(deserializer)? {
        StringOrVec::String(s) => {
            // Handle comma-separated categories or single category
            if s.contains(',') {
                Ok(s.split(',').map(|s| s.trim().to_string()).collect())
            } else {
                Ok(vec![s])
            }
        }
        StringOrVec::Vec(v) => Ok(v),
    }
}

fn deserialize_f32_from_string<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrFloat {
        String(String),
        Float(f32),
    }

    match StringOrFloat::deserialize(deserializer)? {
        StringOrFloat::String(s) => f32::from_str(&s).map_err(serde::de::Error::custom),
        StringOrFloat::Float(f) => Ok(f),
    }
}

fn default_rating() -> f32 {
    0.0
}

fn deserialize_optional_i32<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrInt {
        String(String),
        Int(i32),
        Null,
    }

    match StringOrInt::deserialize(deserializer)? {
        StringOrInt::String(s) => {
            if s.is_empty() {
                Ok(None)
            } else {
                i32::from_str(&s)
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
        }
        StringOrInt::Int(i) => Ok(Some(i)),
        StringOrInt::Null => Ok(None),
    }
}

/// Book model representing a book in the recommendation system
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Book {
    /// Unique identifier for the book
    #[schema(example = "book_12345")]
    pub id: Option<String>,

    /// Title of the book
    #[schema(example = "The Hobbit")]
    pub title: Option<String>,

    /// Author(s) of the book
    #[schema(example = "J.R.R. Tolkien")]
    pub author: Option<String>,

    /// Description or summary of the book
    #[schema(
        example = "A fantasy adventure about a hobbit's unexpected journey to reclaim a treasure guarded by a dragon."
    )]
    pub description: Option<String>,

    /// Categories/genres of the book (can be parsed from string or array)
    #[serde(deserialize_with = "deserialize_categories")]
    #[schema(example = json!(["Fantasy", "Adventure", "Classic Literature"]))]
    pub categories: Vec<String>,

    /// Thumbnail image URL for the book cover
    #[serde(alias = "image_url")]
    #[schema(example = "https://example.com/book-cover.jpg")]
    pub thumbnail: Option<String>,

    /// Average rating of the book (0.0 to 5.0)
    #[serde(
        default = "default_rating",
        deserialize_with = "deserialize_f32_from_string"
    )]
    #[schema(example = 4.5, minimum = 0.0, maximum = 5.0)]
    pub rating: f32,

    /// Publication year of the book
    #[serde(
        alias = "publishedYear",
        default,
        deserialize_with = "deserialize_optional_i32"
    )]
    #[schema(example = 1937)]
    pub year: Option<i32>,

    /// ISBN of the book
    #[schema(example = "978-0547928227")]
    pub isbn: Option<String>,

    /// Number of pages in the book
    #[serde(default, deserialize_with = "deserialize_optional_i32")]
    #[schema(example = 310)]
    pub page_count: Option<i32>,

    /// Total number of ratings the book has received
    #[serde(
        alias = "ratingsCount",
        default,
        deserialize_with = "deserialize_optional_i32"
    )]
    #[schema(example = 1500)]
    pub ratings_count: Option<i32>,

    /// Language of the book
    #[schema(example = "English")]
    pub language: Option<String>,

    /// Publisher of the book
    #[schema(example = "Houghton Mifflin Harcourt")]
    pub publisher: Option<String>,
}

/// Book recommendation with similarity score
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BookRecommendation {
    /// Unique identifier for this recommendation
    #[schema(example = "rec_12345")]
    pub id: String,

    /// The recommended book
    pub book: Book,

    /// Similarity score indicating how well this book matches the query (0.0 to 1.0)
    #[schema(example = 0.85, minimum = 0.0, maximum = 1.0)]
    pub similarity_score: f32,
}
