use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

pub const MAX_ORIGINAL_BYTES: usize = 61_440;
pub const MAX_ORIGINAL_RECORDS: usize = 100;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OriginalsRequest {
    pub session_id: String,
    pub start_turn: Option<usize>,
    pub end_turn: Option<usize>,
    pub next_turn: Option<usize>,
    pub limit: Option<usize>,
    pub max_bytes: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OriginalRecord {
    pub turn_index: usize,
    pub message_id: Uuid,
    pub envelope: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginalSource {
    Original,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OriginalPage {
    pub session_id: Uuid,
    pub source: OriginalSource,
    pub truncated: bool,
    pub records: Vec<OriginalRecord>,
    pub next_turn: Option<usize>,
}

impl OriginalPage {
    pub fn serialized_len(&self) -> Result<usize, OriginalsError> {
        #[derive(Serialize)]
        struct WirePage<'a> {
            result_type: &'static str,
            #[serde(flatten)]
            page: &'a OriginalPage,
        }
        serde_json::to_vec(&WirePage {
            result_type: "originals",
            page: self,
        })
        .map(|bytes| bytes.len())
        .map_err(|error| OriginalsError::SourceUnavailable {
            message: error.to_string(),
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Error)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum OriginalsError {
    #[error("invalid original retrieval request: {message}")]
    InvalidRequest { message: String },
    #[error("original source unavailable: {message}")]
    SourceUnavailable { message: String },
    #[error("stored message {message_id} is missing")]
    MissingMessage { message_id: Uuid },
    #[error("stored message {message_id} has no original envelope")]
    MissingEnvelope { message_id: Uuid },
    #[error("invalid original blob: {message}")]
    InvalidBlob { message: String },
    #[error("original at turn {turn_index} exceeds the {max_bytes}-byte page budget")]
    OversizedEnvelope { turn_index: usize, max_bytes: usize },
}

impl From<Result<OriginalPage, OriginalsError>> for super::types::SessionSearchResult {
    fn from(result: Result<OriginalPage, OriginalsError>) -> Self {
        match result {
            Ok(page) => Self::Originals { page },
            Err(error) => Self::OriginalsError { error },
        }
    }
}
