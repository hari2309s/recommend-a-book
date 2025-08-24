use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub id: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    #[serde(deserialize_with = "deserialize_categories")]
    pub categories: Vec<String>,
    #[serde(alias = "image_url")]
    pub thumbnail: Option<String>,
    #[serde(
        default = "default_rating",
        deserialize_with = "deserialize_f32_from_string"
    )]
    pub rating: f32,
    #[serde(
        alias = "publishedYear",
        default,
        deserialize_with = "deserialize_optional_i32"
    )]
    pub year: Option<i32>,
    pub isbn: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_i32")]
    pub page_count: Option<i32>,
    #[serde(
        alias = "ratingsCount",
        default,
        deserialize_with = "deserialize_optional_i32"
    )]
    pub ratings_count: Option<i32>,
    pub language: Option<String>,
    pub publisher: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookRecommendation {
    pub id: String,
    pub book: Book,
    pub similarity_score: f32,
}
