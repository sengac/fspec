//! BUG-284: durable input admission shared by the task loop and explicit
//! recovery. Admission ownership is observable while the owned worker settles
//! the durable commitment; dispositions after admission are durable,
//! conversation-visible user messages bound to the immutable submission.

use codelet_core::persistence::input_receipts::{self, ExecutionOutcome};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use uuid::Uuid;

static ADMITTING: LazyLock<Mutex<HashMap<Uuid, Uuid>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The submission whose admission currently owns publication for a session.
pub fn admitting_submission(session_id: Uuid) -> Option<Uuid> {
    ADMITTING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&session_id)
        .copied()
}

pub(crate) struct AdmissionOwnership(Uuid);

impl AdmissionOwnership {
    pub(crate) fn claim(session_id: Uuid, submission: Uuid) -> Self {
        ADMITTING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(session_id, submission);
        Self(session_id)
    }
}

impl Drop for AdmissionOwnership {
    fn drop(&mut self) {
        ADMITTING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.0);
    }
}

#[cfg(feature = "test-support")]
static BARRIERS: LazyLock<Mutex<HashMap<Uuid, std::sync::mpsc::Receiver<()>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Test support: the next admission of this session waits, while owning
/// publication, until the returned sender releases it.
#[cfg(feature = "test-support")]
pub fn hold_next_admission(session_id: Uuid) -> std::sync::mpsc::Sender<()> {
    let (release, wait) = std::sync::mpsc::channel();
    BARRIERS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(session_id, wait);
    release
}

/// Runs inside the owned blocking admission worker.
pub(crate) fn admission_barrier(session_id: Uuid) {
    #[cfg(feature = "test-support")]
    {
        let wait = BARRIERS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&session_id);
        if let Some(wait) = wait {
            let _ = wait.recv();
        }
    }
    #[cfg(not(feature = "test-support"))]
    let _ = session_id;
}

/// Truthful model-visible disposition text for an admitted submission. It
/// states only the fact: a failure's reason stays on the durable receipt and
/// in the live error, never in model history. Outcomes that are not a
/// disposition of an undispatched or uncertain submission are refused.
pub fn disposition_text(submission: Uuid, outcome: &ExecutionOutcome) -> Result<String, String> {
    let detail = match outcome {
        ExecutionOutcome::InterruptedBeforeDispatch => {
            "was canceled before any provider request was sent. It has not been executed"
        }
        ExecutionOutcome::FailedBeforeDispatch { .. } => {
            "failed before any provider request was sent. It has not been executed"
        }
        ExecutionOutcome::RecoveredUncertainDispatch => {
            "may or may not have reached the provider before the process stopped. Its effects are unknown and it must not be repeated without verification"
        }
        ExecutionOutcome::DispatchStarted
        | ExecutionOutcome::Completed
        | ExecutionOutcome::Interrupted
        | ExecutionOutcome::Failed { .. } => {
            return Err(format!(
                "submission {submission}: a dispatched outcome has no input disposition"
            ))
        }
    };
    Ok(format!(
        "[Input disposition] Submission {submission} was admitted to this conversation but {detail}. Do not treat it as completed."
    ))
}

/// Make a disposition durable in the admitted conversation, then record it on
/// the receipt. Returns the text the caller installs in runtime history.
pub fn record_disposition(
    session_id: Uuid,
    submission: Uuid,
    outcome: ExecutionOutcome,
) -> Result<String, String> {
    let text = disposition_text(submission, &outcome)?;
    crate::persist::persist_user_message(&session_id, &text)?;
    input_receipts::record_execution(session_id, submission, outcome)
        .map_err(|error| error.to_string())?;
    Ok(text)
}

/// Explicit recovery of an uncertain dispatch: records a truthful durable
/// disposition and releases the inference block. Nothing is re-executed.
pub fn recover_uncertain_dispatch(session_id: Uuid, submission: Uuid) -> Result<String, String> {
    record_disposition(
        session_id,
        submission,
        ExecutionOutcome::RecoveredUncertainDispatch,
    )
}
