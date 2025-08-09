use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistory {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub query: String,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_search_history_creation() {
        let now = Utc::now();
        let user_id = Uuid::new_v4();
        let search = SearchHistory {
            id: Uuid::new_v4(),
            user_id: Some(user_id),
            query: "fantasy books".to_string(),
            timestamp: now,
        };

        assert!(search.id != Uuid::nil());
        assert_eq!(search.user_id, Some(user_id));
        assert_eq!(search.query, "fantasy books");
        assert_eq!(search.timestamp, now);
    }

    #[test]
    fn test_search_history_without_user() {
        let timestamp = Utc.timestamp_opt(1234567890, 0).unwrap();
        let search = SearchHistory {
            id: Uuid::new_v4(),
            user_id: None,
            query: "mystery novels".to_string(),
            timestamp,
        };

        assert!(search.id != Uuid::nil());
        assert_eq!(search.user_id, None);
        assert_eq!(search.query, "mystery novels");
        assert_eq!(search.timestamp.timestamp(), 1234567890);
    }
}
