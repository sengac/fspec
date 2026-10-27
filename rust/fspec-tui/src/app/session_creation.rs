//! PROV-101 FIX 1: explicit handling of a declined session creation.
//!
//! Feature: spec/features/session-creation-decline-surfaced.feature
//!
//! Checked creation returns an empty [`SessionId`] only when no default model
//! is set. Genuine creation failures travel through the backend error channel
//! and the global session diagnostics dialog, independently of this decline.
//!
//! [`post_create_session_action`] is the single, pure mapping every spawned
//! create-session task funnels its result through: a real id becomes
//! [`Action::SessionCreated`]; an empty id becomes
//! [`Action::SessionCreationDeclined`] (which App::dispatch turns into a modal
//! ErrorDialog). No caller may append an empty-id session.

use codelet_rpc_types::SessionId;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::watch;

use crate::components::Action;

/// Map a `create_session` result id to the follow-up [`Action`].
///
/// An empty (or whitespace-only) session id is a PROV-101 decline and maps to
/// [`Action::SessionCreationDeclined`]; any real id maps to
/// [`Action::SessionCreated`].
#[must_use]
pub fn post_create_session_action(session: SessionId) -> Action {
    if session.value.trim().is_empty() {
        Action::SessionCreationDeclined
    } else {
        Action::SessionCreated(session)
    }
}

pub(crate) fn post_create_session_with_work_unit(
    session: SessionId,
    card: Option<String>,
) -> Action {
    match (post_create_session_action(session), card) {
        (Action::SessionCreated(session), Some(card)) => {
            Action::WorkUnitSessionCreated(session, card)
        }
        (action, _) => action,
    }
}

/// Route a `create_session` result from a spawned bootstrap / lazy-session task.
///
/// For a real id: seed the active-session watch channel AND emit
/// [`Action::SessionCreated`]. For an empty id (PROV-101 decline): emit the
/// explicit [`Action::SessionCreationDeclined`] ONLY — the empty id is never
/// seeded as the active session.
pub fn route_bootstrap_create_session(
    session: SessionId,
    active_session_tx: &watch::Sender<Option<SessionId>>,
    action_tx: &UnboundedSender<Action>,
) {
    match post_create_session_action(session) {
        Action::SessionCreated(sid) => {
            let _ = active_session_tx.send(Some(sid.clone()));
            let _ = action_tx.send(Action::SessionCreated(sid));
        }
        other => {
            let _ = action_tx.send(other);
        }
    }
}
