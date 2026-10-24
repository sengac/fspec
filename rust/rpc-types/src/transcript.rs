use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptOriginal {
    pub turn_index: usize,
    pub message_id: String,
    pub timestamp: String,
    pub chunk_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TranscriptCursor {
    pub generation: String,
    pub sequence: u64,
    pub byte_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptFragment {
    pub sequence: u64,
    pub byte_offset: usize,
    pub total_bytes: usize,
    pub json_fragment: String,
    pub original: Option<TranscriptOriginal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptPage {
    pub generation: String,
    pub snapshot_end: u64,
    pub fragment: Option<TranscriptFragment>,
    pub next_cursor: TranscriptCursor,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum TranscriptError {
    #[error("transcript session is unavailable: {0}")]
    SessionUnavailable(String),
    #[error("transcript generation changed")]
    GenerationMismatch,
    #[error("invalid transcript cursor")]
    InvalidCursor,
    #[error("invalid transcript snapshot boundary")]
    InvalidSnapshot,
    #[error("transcript page byte limit must be between 1024 and 65536")]
    InvalidByteLimit,
    #[error("transcript journal unavailable: {0}")]
    JournalUnavailable(String),
    #[error("invalid transcript payload: {0}")]
    InvalidPayload(String),
}
