use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveCursor {
    pub version: u32,
    pub session_id: Uuid,
    pub revision: u64,
    pub end_turn: usize,
    pub prefix_hash: String,
    pub journal_end: usize,
    pub journal_hash: String,
    pub turn_index: usize,
    pub byte_offset: usize,
    pub record_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveRecordKind {
    Conversation,
    ProviderState,
    LegacyText,
    Transcript,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveFragment {
    pub turn_index: usize,
    pub message_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub record_kind: ArchiveRecordKind,
    pub byte_offset: usize,
    pub total_bytes: usize,
    pub json_fragment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivePage {
    pub session_id: Uuid,
    pub source_revision: u64,
    pub end_turn: usize,
    pub fragment: Option<ArchiveFragment>,
    pub next_cursor: Option<ArchiveCursor>,
}

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("invalid archive request: {0}")]
    InvalidRequest(String),
    #[error("invalid archive cursor: {0}")]
    InvalidCursor(String),
    #[error("archive snapshot source changed")]
    SourceChanged,
    #[error("archive source unavailable: {message}")]
    SourceUnavailable { message: String },
    #[error("archive message {message_id} is unavailable")]
    MissingMessage { message_id: Uuid },
    #[error("archive envelope {message_id} is invalid: {message}")]
    InvalidEnvelope { message_id: Uuid, message: String },
    #[error("archive record {message_id} exceeds page budget {max_bytes}")]
    OversizedRecord { message_id: Uuid, max_bytes: usize },
    #[error("archive serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}
