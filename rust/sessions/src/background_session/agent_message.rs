//! Idle-only admission for AgentManager messages, shared by both handlers.

use super::{BackgroundSession, IncomingMessage, Ordering, SessionStatus};
use codelet_tools::agent_manager::AgentManagerResult;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AgentMessageError {
    #[error(
        "Session {session_id} is still busy ({status:?}); the message was not queued. \
         Call AgentManager action 'await_idle' with session_id '{session_id}', then send \
         a fresh message after it reports idle. If the recipient is waiting for you, \
         finish your task and return results instead of waiting on it."
    )]
    Busy {
        session_id: Uuid,
        status: SessionStatus,
    },
    #[error("Incoming message delivery failed for session {session_id}: {reason}")]
    DeliveryFailed { session_id: Uuid, reason: String },
}

impl From<AgentMessageError> for AgentManagerResult {
    fn from(error: AgentMessageError) -> Self {
        let message = error.to_string();
        let code = match error {
            AgentMessageError::Busy { .. } => "session_busy",
            AgentMessageError::DeliveryFailed { .. } => "delivery_failed",
        };
        Self::Error {
            error: true,
            code: code.to_string(),
            message,
        }
    }
}

impl BackgroundSession {
    /// Accept one AgentManager message only while idle, claiming the session
    /// before publishing the message so concurrent sends cannot build a backlog.
    pub fn receive_agent_message(
        &self,
        mut input: IncomingMessage,
    ) -> Result<(), AgentMessageError> {
        input.interrupt_epoch = self.queued_input_epoch();
        let status = self.get_status();
        if status != SessionStatus::Idle {
            return Err(AgentMessageError::Busy {
                session_id: self.id,
                status,
            });
        }

        let permit = self.incoming_message_tx.try_reserve().map_err(|error| {
            AgentMessageError::DeliveryFailed {
                session_id: self.id,
                reason: error.to_string(),
            }
        })?;
        let origin = codelet_core::persistence::input_receipts::InputOrigin::Supervisor {
            role: input.role_name.clone(),
            source_session: input.source_session_id.clone(),
        };
        let submission = self
            .retain_submission(
                origin,
                input.interrupt_epoch,
                &super::format_incoming_message(&input),
                input.images.as_deref().unwrap_or_default(),
            )
            .map_err(|reason| AgentMessageError::DeliveryFailed {
                session_id: self.id,
                reason,
            })?;
        input.submission_id = Some(submission);
        // BUG-648: the claim, its push and its journal entry are one change.
        let _order = self.status_order();
        self.status
            .compare_exchange(
                SessionStatus::Idle as u8,
                SessionStatus::Running as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map_err(|status| {
                self.refuse_submission(submission, "target session became busy");
                AgentMessageError::Busy {
                    session_id: self.id,
                    status: SessionStatus::from(status),
                }
            })?;

        self.incoming_message_pending
            .fetch_add(1, Ordering::Release);
        self.notify_status_change(SessionStatus::Idle as u8, SessionStatus::Running);
        permit.send(input);
        Ok(())
    }
}
