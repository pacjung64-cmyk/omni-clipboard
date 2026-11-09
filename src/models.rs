use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentType {
    Text,
    Code { language: String },
    Url,
    Sensitive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipItem {
    pub id: String,
    pub content_type: ContentType,
    pub raw_content: String,
    pub content_hash: String,
    pub preview_text: String,
    pub char_count: usize,
    pub is_pinned: bool,
    pub created_at: i64,
    pub source_app: Option<String>,
}

impl ClipItem {
    pub fn new(content: &str, content_type: ContentType, source_app: Option<String>) -> Self {
        let hash = Self::compute_hash(content);
        let preview = Self::generate_preview(content);
        let char_count = content.chars().count();

        Self {
            id: Uuid::new_v4().to_string(),
            content_type,
            raw_content: content.to_string(),
            content_hash: hash,
            preview_text: preview,
            char_count,
            is_pinned: false,
            created_at: Utc::now().timestamp_millis(),
            source_app,
        }
    }

    pub fn compute_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn generate_preview(content: &str) -> String {
        let trimmed = content.trim();
        let first_line = trimmed.lines().next().unwrap_or("");
        if first_line.chars().count() > 100 {
            format!("{}...", first_line.chars().take(97).collect::<String>())
        } else {
            first_line.to_string()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    pub query: String,
    pub pinned_only: bool,
    pub limit: usize,
}
