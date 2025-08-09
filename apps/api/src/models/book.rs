use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub title: String,
    pub author: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookRecommendation {
    pub id: Uuid,
    pub book: Book,
    pub similarity_score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_book_creation() {
        let book = Book {
            title: "The Great Gatsby".to_string(),
            author: "F. Scott Fitzgerald".to_string(),
            description: "A story of the Jazz Age".to_string(),
        };

        assert_eq!(book.title, "The Great Gatsby");
        assert_eq!(book.author, "F. Scott Fitzgerald");
        assert_eq!(book.description, "A story of the Jazz Age");
    }

    #[test]
    fn test_book_recommendation_creation() {
        let book = Book {
            title: "1984".to_string(),
            author: "George Orwell".to_string(),
            description: "A dystopian novel".to_string(),
        };

        let id = Uuid::new_v4();
        let recommendation = BookRecommendation {
            id,
            book,
            similarity_score: 0.95,
        };

        assert_eq!(recommendation.id, id);
        assert_eq!(recommendation.book.title, "1984");
        assert_eq!(recommendation.similarity_score, 0.95);
    }
}
