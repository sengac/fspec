//! BUG-284 task-loop admission: Pending receipts become Cancelled, Refused or
//! Admitted under the runtime lock; an admitted original is installed in
//! runtime before dispatch ownership is released, and its execution outcome
//! is recorded without ever revoking membership.

use super::state::PendingInput;
use crate::input_admission::{record_disposition, AdmissionOwnership};
use codelet_cli::session::Session;
use codelet_core::persistence::input_receipts::{
    self, ExecutionOutcome, InputAuthorityError, InputDisposition, InputOrigin,
};
use codelet_core::persistence::load_session;
use codelet_sessions::background_session::{input_content, BackgroundSession};
use rig::message::{Message, UserContent};
use std::sync::atomic::Ordering;
use uuid::Uuid;

/// Registers durable persistence for internal continuations that replace an
/// admitted input consumed by pre-prompt compaction, for the loop lifetime.
pub(super) struct ContinuationRegistration(Uuid);

impl ContinuationRegistration {
    pub(super) fn new(session_id: Uuid) -> Self {
        let persister: codelet_cli::session::admission::ContinuationPersister =
            std::sync::Arc::new(move |text: &str| {
                crate::persist::persist_user_message(&session_id, text)
            });
        codelet_cli::session::admission::set_continuation_persister(session_id, Some(persister));
        Self(session_id)
    }
}

impl Drop for ContinuationRegistration {
    fn drop(&mut self) {
        codelet_cli::session::admission::set_continuation_persister(self.0, None);
    }
}

pub(super) enum Gate {
    Cancelled,
    Refused(anyhow::Error),
    Dispatch(Uuid),
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| error.to_string())?
}

fn content(pending: &PendingInput) -> Vec<codelet_core::persistence::UserContent> {
    input_content(
        &pending.input.text,
        pending.input.images.as_deref().unwrap_or_default(),
    )
}

/// Inputs without a submission-time receipt (internal retries, MCP) are
/// retained at dequeue with their truthful origin before any other step.
pub(super) fn ensure_receipt(session: &BackgroundSession, pending: &mut PendingInput) -> bool {
    if pending.submission.is_some() {
        return true;
    }
    let origin = if pending.is_retry_input || pending.is_compaction_retry {
        InputOrigin::Internal
    } else {
        InputOrigin::Mcp
    };
    let images = pending.input.images.clone().unwrap_or_default();
    match session.retain_submission(
        origin,
        Some(pending.interrupt_epoch),
        &pending.input.text,
        &images,
    ) {
        Ok(id) => {
            pending.submission = Some(id);
            true
        }
        Err(error) => {
            tracing::error!(session_id = %session.id, %error, "input receipt retention failed");
            false
        }
    }
}

/// Settle a non-admitted disposition for the pending submission.
pub(super) async fn settle(
    session: &BackgroundSession,
    pending: &PendingInput,
    to: InputDisposition,
) {
    let (session_id, Some(id)) = (session.id, pending.submission) else {
        return;
    };
    if let Err(error) =
        blocking(move || input_receipts::settle(session_id, id, to).map_err(|e| e.to_string()))
            .await
    {
        tracing::error!(%session_id, submission = %id, %error, "input receipt settlement unresolved");
    }
}

/// Durably retain the original in the message store without membership.
pub(super) async fn store_original(
    session: &BackgroundSession,
    pending: &PendingInput,
) -> Result<(), String> {
    let (session_id, id) = (
        session.id,
        pending.submission.ok_or("missing submission identity")?,
    );
    let original = content(pending);
    blocking(move || crate::persist::persist_input_original(&session_id, id, original)).await
}

/// Admit and install under the runtime lock, or settle cancellation/refusal.
pub(super) async fn admit_turn(
    session: &BackgroundSession,
    inner: &mut Session,
    pending: &PendingInput,
) -> Gate {
    let Some(id) = pending.submission else {
        return Gate::Refused(anyhow::anyhow!("submission identity unavailable"));
    };
    if super::prompt::cancel_pending(session, pending, false).await {
        return Gate::Cancelled;
    }
    if let Err(error) = authorize(session, inner, pending).await {
        let reason = error.to_string();
        settle(session, pending, InputDisposition::Refused { reason }).await;
        return Gate::Refused(anyhow::Error::new(error));
    }
    let ownership = AdmissionOwnership::claim(session.id, id);
    let session_id = session.id;
    let admitted = blocking(move || {
        crate::input_admission::admission_barrier(session_id);
        input_receipts::admit(session_id, id).map_err(|error| error.to_string())
    })
    .await;
    drop(ownership);
    if let Err(error) = admitted {
        return Gate::Refused(anyhow::anyhow!(
            "input admission unresolved; inference blocked: {error}"
        ));
    }
    let images = pending.input.images.clone().map(|images| {
        images
            .into_iter()
            .map(|image| codelet_cli::interactive::BridgeImage {
                data: image.data,
                media_type: image.media_type,
            })
            .collect()
    });
    let message = Message::User {
        content: codelet_cli::interactive::build_user_content_with_images(
            &pending.input.text,
            images,
        ),
    };
    inner.install_admitted_input(id, message);
    if session.compaction_cancelled(pending.interrupt_epoch) {
        before_dispatch(
            session,
            inner,
            id,
            ExecutionOutcome::InterruptedBeforeDispatch,
        )
        .await;
        super::prompt::finish_cancelled_prompt(session, pending.interrupt_epoch, false);
        return Gate::Cancelled;
    }
    let started = blocking(move || {
        input_receipts::record_execution(session_id, id, ExecutionOutcome::DispatchStarted)
            .map_err(|error| error.to_string())
    })
    .await;
    match started {
        Ok(()) => Gate::Dispatch(id),
        Err(error) => {
            let reason = format!("dispatch ownership unavailable: {error}");
            before_dispatch(
                session,
                inner,
                id,
                ExecutionOutcome::FailedBeforeDispatch {
                    reason: reason.clone(),
                },
            )
            .await;
            Gate::Refused(anyhow::anyhow!(reason))
        }
    }
}

/// Verify receipt authority. A genuine user instruction is the explicit act
/// that records a truthful disposition for an uncertain earlier dispatch.
async fn authorize(
    session: &BackgroundSession,
    inner: &mut Session,
    pending: &PendingInput,
) -> Result<(), InputAuthorityError> {
    let session_id = session.id;
    loop {
        let check = move || {
            let manifest = load_session(session_id).map_err(InputAuthorityError::Session)?;
            input_receipts::inference_authority(&manifest)
        };
        match tokio::task::spawn_blocking(check).await {
            Ok(Err(InputAuthorityError::UncertainDispatch(previous)))
                if pending.is_fresh_user_input =>
            {
                let recovered = blocking(move || {
                    crate::input_admission::recover_uncertain_dispatch(session_id, previous)
                })
                .await
                .map_err(InputAuthorityError::Session)?;
                inner.messages.push(Message::User {
                    content: rig::OneOrMany::one(UserContent::text(recovered)),
                });
            }
            Ok(result) => return result,
            Err(error) => return Err(InputAuthorityError::Session(error.to_string())),
        }
    }
}

/// Record a pre-dispatch outcome with a durable, model-visible disposition.
async fn before_dispatch(
    session: &BackgroundSession,
    inner: &mut Session,
    id: Uuid,
    outcome: ExecutionOutcome,
) {
    inner.clear_admitted_input();
    let session_id = session.id;
    match blocking(move || record_disposition(session_id, id, outcome)).await {
        Ok(text) => inner.messages.push(Message::User {
            content: rig::OneOrMany::one(UserContent::text(text)),
        }),
        Err(error) => {
            tracing::error!(%session_id, submission = %id, %error, "admitted input disposition unresolved");
            session.handle_output(codelet_rpc_types::StreamChunk::error(format!(
                "Input disposition requires recovery before inference: {error}"
            )));
        }
    }
}

/// Settle the execution outcome after the provider run returns.
pub(super) async fn settle_dispatch(
    session: &BackgroundSession,
    inner: &mut Session,
    id: Uuid,
    result: &anyhow::Result<()>,
) {
    let interrupted = session.is_interrupted.load(Ordering::Acquire);
    if inner.admitted_input_pending() == Some(id) {
        let outcome = match result {
            Err(error) => ExecutionOutcome::FailedBeforeDispatch {
                reason: format!("{error:#}"),
            },
            Ok(()) if interrupted => ExecutionOutcome::InterruptedBeforeDispatch,
            Ok(()) => ExecutionOutcome::FailedBeforeDispatch {
                reason: "provider returned before dispatch".into(),
            },
        };
        before_dispatch(session, inner, id, outcome).await;
        return;
    }
    let outcome = match result {
        Err(error) => ExecutionOutcome::Failed {
            reason: format!("{error:#}"),
        },
        Ok(()) if interrupted => ExecutionOutcome::Interrupted,
        Ok(()) => ExecutionOutcome::Completed,
    };
    let session_id = session.id;
    if let Err(error) = blocking(move || {
        input_receipts::record_execution(session_id, id, outcome).map_err(|e| e.to_string())
    })
    .await
    {
        tracing::error!(%session_id, submission = %id, %error, "admitted input execution outcome unresolved");
    }
}
