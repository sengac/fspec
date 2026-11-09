//! PROV-101 FIX 1 / BUG-410: the follow-up action for a created session.
//!
//! Feature: spec/features/session-creation-decline-surfaced.feature
//!
//! `create_session` returns `Err` when it declines (for example no default
//! model is set); that error travels through the single creation-failure path
//! in `session_creation_tasks` (the session failure dialog). A returned
//! [`SessionId`] is always a created session.

use codelet_rpc_types::SessionId;

use crate::components::Action;

/// Map a created session to its follow-up [`Action`]: bound to the card it
/// was created for, or a plain [`Action::SessionCreated`].
pub(crate) fn post_create_session_with_work_unit(
    session: SessionId,
    card: Option<String>,
) -> Action {
    match card {
        Some(card) => Action::WorkUnitSessionCreated(session, card),
        None => Action::SessionCreated(session),
    }
}
