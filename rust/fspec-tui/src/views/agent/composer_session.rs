//! BUG-552 — a composer submission carries the session whose composer
//! produced it.
//!
//! Feature: spec/features/a-submission-goes-to-the-session-it-was-typed-in.feature
//!
//! The one live composer always holds exactly one session's draft (BUG-549,
//! `ComposerDrafts::bound`); `App::sync_composer_draft` hands that session
//! to the view at the end of every dispatch, event and paste. Enter and
//! Alt+Enter stamp it on the submission when the key is handled, so a focus
//! change handled before the queued submission can never take it.

use codelet_rpc_types::SessionId;

use crate::components::Action;

use super::slash_commands::SlashCommandAction;
use super::AgentView;

impl AgentView {
    /// The session whose draft the composer holds (BUG-549 bound session).
    pub fn set_composer_session(&mut self, session: Option<SessionId>) {
        self.composer_session = session;
    }

    pub fn composer_session(&self) -> Option<&SessionId> {
        self.composer_session.as_ref()
    }

    /// Emit `text` submitted from the composer for the composer's session:
    /// queued behind the running turn when `queue`, otherwise a send (or a
    /// steer while that session is busy). With no session open nothing is
    /// emitted: no session could take it.
    pub(super) fn emit_submission(&self, text: String, queue: bool) {
        let Some(session) = self.composer_session.clone() else {
            tracing::debug!(target: "bug552", "composer submission with no session dropped");
            return;
        };
        self.emit(if queue {
            Action::InputQueued(session, text)
        } else {
            Action::InputSubmitted(session, text)
        });
    }

    /// BUG-560: emit a command picked from the slash popup for the
    /// composer's session (`None` when no session is open), captured now so
    /// a focus change handled before the pick can never take it.
    pub(super) fn emit_slash_pick(&self, action: SlashCommandAction) {
        self.emit(Action::SlashCommandSelected(
            self.composer_session.clone(),
            action,
        ));
    }
}
