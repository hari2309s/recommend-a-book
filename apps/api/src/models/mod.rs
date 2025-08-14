use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    #[serde(default)]
    pub id: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub author: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
    #[serde(default)]
    pub rating: f32,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub thumbnail: Option<String>,
    #[serde(default, deserialize_with = "deserialize_categories")]
    pub categories: Vec<String>,
    #[serde(
        rename = "publishedYear",
        default,
        deserialize_with = "deserialize_optional_i32"
    )]
    pub published_year: Option<i32>,
    #[serde(
        rename = "ratingsCount",
        default,
        deserialize_with = "deserialize_optional_i32"
    )]
    pub ratings_count: Option<i32>,
}

// Custom deserializers to handle various data formats from Pinecone
fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct StringVisitor;

    impl<'de> Visitor<'de> for StringVisitor {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string or null")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value.is_empty() {
                Ok(None)
            } else {
                Ok(Some(value.to_string()))
            }
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value.is_empty() {
                Ok(None)
            } else {
                Ok(Some(value))
            }
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_any(StringVisitor)
}

fn deserialize_optional_i32<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct I32Visitor;

    impl<'de> Visitor<'de> for I32Visitor {
        type Value = Option<i32>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an integer or null")
        }

        fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value as i32))
        }

        fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value as i32))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value as i32))
        }

        fn visit_f32<E>(self, value: f32) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value as i32))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value as i32))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse::<i32>().map(Some).or(Ok(None))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_any(I32Visitor)
}

fn deserialize_categories<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct CategoriesVisitor;

    impl<'de> Visitor<'de> for CategoriesVisitor {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string, array of strings, or null")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value.is_empty() {
                Ok(vec![])
            } else {
                // Split by common separators
                Ok(value
                    .split(&[',', ';', '|', '&'][..])
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect())
            }
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(&value)
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut categories = Vec::new();
            while let Some(item) = seq.next_element::<String>()? {
                if !item.trim().is_empty() {
                    categories.push(item.trim().to_string());
                }
            }
            Ok(categories)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![])
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![])
        }
    }

    deserializer.deserialize_any(CategoriesVisitor)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookRecommendation {
    pub id: Uuid,
    pub book: Book,
    pub similarity_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistory {
    pub id: Option<Uuid>,
    pub user_id: Uuid,
    pub query: String,
    pub recommendations: Vec<Book>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationRequest {
    pub query: String,
    pub top_k: Option<usize>,
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryRequest {
    pub user_id: String,
    pub limit: Option<i32>,
}
