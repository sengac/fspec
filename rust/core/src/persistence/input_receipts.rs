//! BUG-284: durable submission receipts.
//!
//! A submission is retained as an immutable, versioned receipt before its queue
//! acknowledgment. Receipts are audit/recovery evidence, not conversation
//! membership: only an explicit admission appends the stored original to
//! `manifest.messages`. Dispositions are monotone (Pending becomes Cancelled,
//! Refused or Admitted) and execution outcomes never downgrade admission.

#[path = "input_receipts/authority.rs"]
mod authority;
#[path = "input_receipts/store.rs"]
mod store;

pub use authority::{
    admit, inference_authority, recover_uncertain_dispatch, settle_admission, InputAuthorityError,
};

use super::message_envelope::UserContent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const INPUT_RECEIPT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum InputOrigin {
    User,
    Supervisor {
        role: String,
        source_session: String,
    },
    Mcp,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum InputDisposition {
    Pending,
    Cancelled,
    Refused { reason: String },
    Admitted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum ExecutionOutcome {
    DispatchStarted,
    InterruptedBeforeDispatch,
    FailedBeforeDispatch { reason: String },
    Completed,
    Interrupted,
    Failed { reason: String },
    RecoveredUncertainDispatch,
}

impl ExecutionOutcome {
    pub fn is_terminal(&self) -> bool {
        !matches!(self, Self::DispatchStarted)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputReceipt {
    pub version: u32,
    pub id: Uuid,
    pub session_id: Uuid,
    pub origin: InputOrigin,
    pub epoch: Option<u64>,
    pub submitted_at: DateTime<Utc>,
    pub content: Vec<UserContent>,
    pub content_hash: String,
    pub disposition: InputDisposition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<ExecutionOutcome>,
}

impl InputReceipt {
    /// Build a Pending receipt with a fresh server-local submission identity.
    pub fn pending(
        session_id: Uuid,
        origin: InputOrigin,
        epoch: Option<u64>,
        content: Vec<UserContent>,
    ) -> Result<Self, InputReceiptError> {
        let content_hash = super::compute_hash(&serde_json::to_vec(&content)?);
        Ok(Self {
            version: INPUT_RECEIPT_VERSION,
            id: Uuid::new_v4(),
            session_id,
            origin,
            epoch,
            submitted_at: Utc::now(),
            content,
            content_hash,
            disposition: InputDisposition::Pending,
            execution: None,
        })
    }

    /// The immutable identity that same-UUID retries must reproduce.
    fn same_identity(&self, other: &Self) -> bool {
        self.id == other.id
            && self.session_id == other.session_id
            && self.origin == other.origin
            && self.content_hash == other.content_hash
            && self.content == other.content
    }

    /// Plain text of the original, joined across text blocks.
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| match block {
                UserContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InputReceiptError {
    #[error("input receipt storage unavailable: {0}")]
    Io(#[from] std::io::Error),
    #[error("input receipt encoding failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("data directory unavailable: {0}")]
    DataDirectory(String),
    #[error("submission {0} conflicts with an existing receipt identity")]
    Conflict(Uuid),
    #[error("submission {id} has unsupported receipt version {version}")]
    UnknownVersion { id: Uuid, version: u32 },
    #[error("submission {0} has no retained receipt")]
    Missing(Uuid),
    #[error("submission {id} cannot move from {from} to {to}")]
    Transition { id: Uuid, from: String, to: String },
}

/// Durably retain a Pending receipt. Same-identity retries are idempotent;
/// a different original under the same UUID is refused.
pub fn retain(receipt: &InputReceipt) -> Result<(), InputReceiptError> {
    store::update(receipt.session_id, receipt.id, |existing| match existing {
        Some(existing) if existing.same_identity(receipt) => Ok(None),
        Some(_) => Err(InputReceiptError::Conflict(receipt.id)),
        None => Ok(Some(receipt.clone())),
    })
}

/// All receipts for a session in submission order. Unknown versions refuse.
pub fn list(session_id: Uuid) -> Result<Vec<InputReceipt>, InputReceiptError> {
    store::list(session_id)
}

pub fn get(session_id: Uuid, id: Uuid) -> Result<Option<InputReceipt>, InputReceiptError> {
    store::read(session_id, id)
}

/// Monotone disposition settlement: Pending moves once; repeats are idempotent.
pub fn settle(
    session_id: Uuid,
    id: Uuid,
    disposition: InputDisposition,
) -> Result<(), InputReceiptError> {
    store::update(session_id, id, |existing| {
        let mut receipt = existing.ok_or(InputReceiptError::Missing(id))?;
        if receipt.disposition == disposition {
            return Ok(None);
        }
        if receipt.disposition != InputDisposition::Pending {
            return Err(transition(id, &receipt.disposition, &disposition));
        }
        receipt.disposition = disposition;
        Ok(Some(receipt))
    })
}

/// Record execution of an admitted input without changing its admission.
pub fn record_execution(
    session_id: Uuid,
    id: Uuid,
    outcome: ExecutionOutcome,
) -> Result<(), InputReceiptError> {
    store::update(session_id, id, |existing| {
        let mut receipt = existing.ok_or(InputReceiptError::Missing(id))?;
        if receipt.disposition != InputDisposition::Admitted {
            return Err(transition(id, &receipt.disposition, &outcome));
        }
        let allowed = match &receipt.execution {
            Some(current) if *current == outcome => return Ok(None),
            Some(ExecutionOutcome::DispatchStarted) => outcome.is_terminal(),
            Some(_) => false,
            None => outcome != ExecutionOutcome::RecoveredUncertainDispatch,
        };
        if !allowed {
            return Err(transition(id, &receipt.execution, &outcome));
        }
        receipt.execution = Some(outcome);
        Ok(Some(receipt))
    })
}

fn transition(
    id: Uuid,
    from: &impl std::fmt::Debug,
    to: &impl std::fmt::Debug,
) -> InputReceiptError {
    InputReceiptError::Transition {
        id,
        from: format!("{from:?}"),
        to: format!("{to:?}"),
    }
}

/// Receipt identities that garbage collection must treat as message roots.
pub fn retained_ids(session_id: Uuid) -> Result<Vec<Uuid>, InputReceiptError> {
    store::ids(session_id)
}
