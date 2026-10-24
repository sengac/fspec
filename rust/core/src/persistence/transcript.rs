use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod projection;
mod store;
pub use projection::{JournalProjection, RecoveredTool, ToolState};
pub use store::{JournalError, JournalStore};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalIdentity {
    pub session_id: Uuid,
    pub request_id: Uuid,
    pub attempt_id: Uuid,
    pub message_id: Uuid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalOutcome {
    ProviderError,
    UserCancelled,
    LoopInterrupted,
    Interrupted,
    CompactionFailed,
    Completed,
}

impl TerminalOutcome {
    pub fn message(self) -> &'static str {
        match self {
            Self::ProviderError => "Provider request failed.",
            Self::UserCancelled => "Response cancelled by the user.",
            Self::LoopInterrupted => "Response interrupted by loop detection.",
            Self::Interrupted => "Response interrupted.",
            Self::CompactionFailed => "Compaction failed before publication.",
            Self::Completed => "Response completed.",
        }
    }

    pub fn severity(self) -> &'static str {
        match self {
            Self::ProviderError => "error",
            Self::CompactionFailed | Self::LoopInterrupted | Self::Interrupted => "warning",
            Self::UserCancelled | Self::Completed => "info",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum JournalEvent {
    AttemptStarted,
    AttemptAborted,
    Text {
        text: String,
    },
    Thinking {
        thinking: String,
    },
    ToolRequested {
        id: String,
        call_id: Option<String>,
        name: String,
        input: serde_json::Value,
    },
    ToolStarted {
        id: String,
    },
    ToolOutcomeUnknown {
        id: String,
    },
    ToolCompleted {
        id: String,
        content: String,
        is_error: bool,
    },
    ToolCancelled {
        id: String,
    },
    Terminal {
        outcome: TerminalOutcome,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestAnchor {
    pub original_count: usize,
    pub preceding_message_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalRecord {
    pub version: u32,
    pub id: Uuid,
    pub identity: JournalIdentity,
    pub sequence: u64,
    pub timestamp: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_anchor: Option<RequestAnchor>,
    pub event: JournalEvent,
}

impl JournalRecord {
    pub fn new(identity: JournalIdentity, sequence: u64, event: JournalEvent) -> Self {
        Self {
            version: 1,
            id: Uuid::new_v4(),
            identity,
            sequence,
            timestamp: Utc::now(),
            request_anchor: None,
            event,
        }
    }
}
