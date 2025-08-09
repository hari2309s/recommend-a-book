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
