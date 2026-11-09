//! Shared fixtures for the AgentManager test modules: handler reset, the mock
//! sync handler with message tracking, and the mock await_idle handler.
//! Split out of tests.rs when its tests moved to one file per feature (COV-075).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]

use super::handler::*;
use super::types::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;

use super::handler::AgentManagerAsyncHandler;
use super::types::{AwaitOutcome, AwaitSessionResult};

pub(super) fn with_clean_handlers<T>(f: impl FnOnce() -> T) -> T {
    clear_all_agent_manager_handlers();
    let result = f();
    clear_all_agent_manager_handlers();
    result
}

/// Tracks messages sent to a mock session for verification
#[derive(Debug, Clone)]
pub(super) struct DeliveredMessage {
    pub(super) source_session_id: String,
    pub(super) role_name: String,
    pub(super) message: String,
}

/// Helper to create a mock handler that tracks calls and returns configurable results.
/// Supports all 5 actions: spawn, list, get_status, close, message.
///
/// The `message_log` collects all delivered messages for verification.
/// The `channel_full` flag simulates a full incoming_message channel.
pub(super) fn mock_handler_with_messaging(
    sessions: Vec<SessionEntry>,
    message_log: Arc<std::sync::Mutex<Vec<DeliveredMessage>>>,
    channel_full: bool,
) -> (AgentManagerHandler, Arc<AtomicBool>) {
    let called = Arc::new(AtomicBool::new(false));
    let called_clone = called.clone();
    let sessions = Arc::new(sessions);

    let handler: AgentManagerHandler = Arc::new(move |action, calling_session_id| {
        called_clone.store(true, Ordering::SeqCst);
        match action {
            AgentManagerAction::Spawn { role: _ } => {
                let subordinate_id = Uuid::new_v4().to_string();
                AgentManagerResult::Spawned {
                    session_id: subordinate_id,
                }
            }
            AgentManagerAction::List => AgentManagerResult::Listed {
                sessions: (*sessions).clone(),
            },
            AgentManagerAction::GetStatus { session_id } => {
                if let Some(entry) = sessions.iter().find(|s| s.session_id == session_id) {
                    AgentManagerResult::Status(SessionStatus {
                        session_id: entry.session_id.clone(),
                        role: entry.role.clone(),
                        status: entry.status.clone(),
                        model: Some("anthropic/claude-sonnet-4".to_string()),
                        spawner_id: entry.spawner_id.clone(),
                        subordinate_ids: entry.subordinate_ids.clone(),
                        pending_messages: 0,
                        pending_request: None,
                    })
                } else {
                    AgentManagerResult::session_not_found(&session_id)
                }
            }
            AgentManagerAction::Close { session_id } => {
                if let Some(entry) = sessions.iter().find(|s| s.session_id == session_id) {
                    if entry.spawner_id.as_deref() == Some(&calling_session_id.to_string()) {
                        AgentManagerResult::Closed {
                            closed: true,
                            session_id,
                        }
                    } else {
                        AgentManagerResult::permission_denied(
                            "Only the spawner can close a subordinate session",
                        )
                    }
                } else {
                    AgentManagerResult::session_not_found(&session_id)
                }
            }
            AgentManagerAction::Message {
                session_id,
                message,
                context: _,
            } => {
                // Check if target session exists
                let target_exists = sessions.iter().any(|s| s.session_id == session_id)
                    || session_id == calling_session_id.to_string();

                if !target_exists {
                    return AgentManagerResult::session_not_found(&session_id);
                }

                if channel_full {
                    return AgentManagerResult::delivery_failed(&format!(
                        "Incoming message channel full for session {session_id}"
                    ));
                }

                // Find sender's role from sessions list
                let sender_role = sessions
                    .iter()
                    .find(|s| s.session_id == calling_session_id.to_string())
                    .and_then(|s| s.role.clone())
                    .unwrap_or_default();

                message_log.lock().unwrap().push(DeliveredMessage {
                    source_session_id: calling_session_id.to_string(),
                    role_name: sender_role,
                    message,
                });

                AgentManagerResult::MessageDelivered {
                    delivered: true,
                    session_id,
                }
            }
            AgentManagerAction::SetRole { session_id, role } => {
                let target_id = session_id.unwrap_or_else(|| calling_session_id.to_string());

                let target_exists = sessions.iter().any(|s| s.session_id == target_id)
                    || target_id == calling_session_id.to_string();

                if !target_exists {
                    return AgentManagerResult::session_not_found(&target_id);
                }

                if role.is_empty() {
                    AgentManagerResult::RoleSet {
                        session_id: target_id,
                        role: None,
                    }
                } else {
                    AgentManagerResult::RoleSet {
                        session_id: target_id,
                        role: Some(role),
                    }
                }
            }
            AgentManagerAction::Answer { .. }
            | AgentManagerAction::Approve { .. }
            | AgentManagerAction::Escalate { .. } => {
                AgentManagerResult::invalid_parameter("requests are not scripted in this mock")
            }
            AgentManagerAction::AwaitIdle { .. } => AgentManagerResult::invalid_parameter(
                "await_idle must be dispatched through async handler",
            ),
            AgentManagerAction::Profile { .. } => AgentManagerResult::invalid_parameter(
                "profile must be dispatched through async handler",
            ),
        }
    });

    (handler, called)
}

/// Backward-compatible mock_handler that wraps mock_handler_with_messaging
/// for existing core tests
pub(super) fn mock_handler(sessions: Vec<SessionEntry>) -> (AgentManagerHandler, Arc<AtomicBool>) {
    let message_log = Arc::new(std::sync::Mutex::new(Vec::new()));
    mock_handler_with_messaging(sessions, message_log, false)
}

pub(super) fn mock_async_handler(
    session_outcomes: std::collections::HashMap<String, AwaitOutcome>,
) -> AgentManagerAsyncHandler {
    let outcomes = Arc::new(session_outcomes);
    Arc::new(
        move |action: AgentManagerAction, _calling_session_id: Uuid| {
            let outcomes = outcomes.clone();
            Box::pin(async move {
                match action {
                    AgentManagerAction::AwaitIdle {
                        session_id,
                        timeout: _,
                    } => {
                        let ids = session_id.into_vec();
                        for id in &ids {
                            if !outcomes.contains_key(id) {
                                return AgentManagerResult::session_not_found(id);
                            }
                        }
                        let results: Vec<AwaitSessionResult> = ids
                            .into_iter()
                            .map(|id| {
                                let status =
                                    outcomes.get(&id).cloned().unwrap_or(AwaitOutcome::TimedOut);
                                AwaitSessionResult {
                                    session_id: id,
                                    status,
                                    request: None,
                                }
                            })
                            .collect();
                        AgentManagerResult::AwaitResult { results }
                    }
                    _ => AgentManagerResult::invalid_parameter("expected AwaitIdle"),
                }
            })
        },
    )
}
