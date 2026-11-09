//! Admission and inference authority derived from receipts plus committed
//! conversation membership. Membership is the durable admission commitment;
//! a receipt that disagrees with it refuses inference instead of guessing.

use super::{ExecutionOutcome, InputDisposition, InputReceiptError};
use crate::persistence::{
    conversation_commit::admit_stored_message, load_session,
    manifest_commit::reacknowledge_session_directory, ManifestCommitError, SessionManifest,
};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum InputAuthorityError {
    #[error(transparent)]
    Receipt(#[from] InputReceiptError),
    #[error("submission {id} admission was not committed: {source}")]
    Commit {
        id: Uuid,
        source: ManifestCommitError,
    },
    #[error("submission {id} admission durability is unresolved; inference is blocked: {reason}")]
    Unresolved { id: Uuid, reason: String },
    #[error("submission {0} is {1:?} and cannot be admitted")]
    NotAdmissible(Uuid, InputDisposition),
    #[error("submission {0} has contradictory receipt and conversation membership; explicit evidence-preserving recovery is required")]
    Contradiction(Uuid),
    #[error("submission {0} may already have been dispatched; continuation is blocked until explicit recovery records its disposition")]
    UncertainDispatch(Uuid),
    #[error("session state unavailable: {0}")]
    Session(String),
}

/// Promote a retained Pending original into conversation membership once.
/// A lost acknowledgment after manifest promotion settles the same receipt.
pub fn admit(session_id: Uuid, id: Uuid) -> Result<(), InputAuthorityError> {
    let receipt = super::get(session_id, id)?.ok_or(InputReceiptError::Missing(id))?;
    match receipt.disposition {
        InputDisposition::Pending | InputDisposition::Admitted => {}
        other => return Err(InputAuthorityError::NotAdmissible(id, other)),
    }
    match admit_stored_message(session_id, id) {
        Ok(_) => {}
        Err(ManifestCommitError::Promoted { .. }) => return settle_admission(session_id, id),
        Err(source) => return Err(InputAuthorityError::Commit { id, source }),
    }
    super::settle(session_id, id, InputDisposition::Admitted)?;
    Ok(())
}

/// Re-acknowledge manifest durability and settle the receipt from the
/// committed membership. Unresolved durability keeps inference blocked.
pub fn settle_admission(session_id: Uuid, id: Uuid) -> Result<(), InputAuthorityError> {
    reacknowledge_session_directory(session_id).map_err(|error| {
        InputAuthorityError::Unresolved {
            id,
            reason: error.to_string(),
        }
    })?;
    let manifest = load_session(session_id).map_err(InputAuthorityError::Session)?;
    if !manifest
        .messages
        .iter()
        .any(|message| message.message_id == id)
    {
        return Err(InputAuthorityError::Unresolved {
            id,
            reason: "promoted membership is absent on readback".into(),
        });
    }
    super::settle(session_id, id, InputDisposition::Admitted)?;
    Ok(())
}

/// Refuse inference when any receipt is unreadable, unresolved,
/// contradicts membership or records an uncertain dispatch.
pub fn inference_authority(manifest: &SessionManifest) -> Result<(), InputAuthorityError> {
    let members: HashSet<Uuid> = manifest
        .messages
        .iter()
        .map(|message| message.message_id)
        .collect();
    for receipt in super::list(manifest.id)? {
        let member = members.contains(&receipt.id);
        match (&receipt.disposition, member) {
            (InputDisposition::Pending, true) => {
                return Err(InputAuthorityError::Unresolved {
                    id: receipt.id,
                    reason: "membership promoted without settled receipt".into(),
                })
            }
            (InputDisposition::Cancelled | InputDisposition::Refused { .. }, true)
            | (InputDisposition::Admitted, false) => {
                return Err(InputAuthorityError::Contradiction(receipt.id))
            }
            (InputDisposition::Admitted, true)
                if receipt.execution == Some(ExecutionOutcome::DispatchStarted) =>
            {
                return Err(InputAuthorityError::UncertainDispatch(receipt.id))
            }
            _ => {}
        }
    }
    Ok(())
}

/// Explicit recovery for an uncertain dispatch. The caller must first make a
/// truthful disposition durable and model visible; this never re-executes.
pub fn recover_uncertain_dispatch(session_id: Uuid, id: Uuid) -> Result<(), InputAuthorityError> {
    super::record_execution(session_id, id, ExecutionOutcome::RecoveredUncertainDispatch)?;
    Ok(())
}
