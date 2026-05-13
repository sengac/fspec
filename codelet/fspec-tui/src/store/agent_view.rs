//! AgentViewStore — single source of truth for the AgentView session
//! navigation state.
//!
//! Feature: spec/features/rpc012-board-agent-navigation.feature
//! Card: RPC-012 (parent RPC-002).
//!
//! Plain owned Rust struct held by [`crate::app::App`]. Mirrors the
//! navigation-relevant slice of `src/tui/store/sessionStore.ts` —
//! specifically `currentSessionId`, `navigationTargetSessionId`,
//! `currentWorkUnitId`, `currentWorkUnitStatus`, `showCreateSessionDialog`,
//! and `shouldAutoCreateSession`.
//!
//! Multi-session, isolation, debug-state-by-session, and other
//! TS-only-for-now fields are deferred to downstream slices.

use codelet_rpc_types::SessionId;

/// AgentView session navigation state. Mutated only on the App task.
#[derive(Debug, Default)]
pub struct AgentViewStore {
    current_session: Option<SessionId>,
    navigation_target_session: Option<SessionId>,
    current_work_unit_id: Option<String>,
    current_work_unit_status: Option<String>,
    show_create_session_dialog: bool,
    should_auto_create_session: bool,
}

impl AgentViewStore {
    /// Borrow the active session for AgentView, or `None` before lazy
    /// creation has completed.
    pub fn current_session(&self) -> Option<&SessionId> {
        self.current_session.as_ref()
    }

    /// Set (or clear) the active session id.
    pub fn set_current_session(&mut self, session: Option<SessionId>) {
        self.current_session = session;
    }

    /// Borrow the pending navigation target — set by BoardView's
    /// `Shift+Right` handler and consumed by AgentView on next render.
    pub fn navigation_target_session(&self) -> Option<&SessionId> {
        self.navigation_target_session.as_ref()
    }

    /// Set (or clear) the navigation target.
    pub fn set_navigation_target(&mut self, target: Option<SessionId>) {
        self.navigation_target_session = target;
    }

    /// Consume the navigation target, returning the inner SessionId if
    /// any was present. Equivalent to TS
    /// `sessionStore.clearNavigationTarget()` after read.
    pub fn take_navigation_target(&mut self) -> Option<SessionId> {
        self.navigation_target_session.take()
    }

    /// Current work-unit id under the AgentView session header.
    pub fn current_work_unit_id(&self) -> Option<&str> {
        self.current_work_unit_id.as_deref()
    }

    /// Current work-unit status (e.g. "implementing") under the
    /// AgentView session header.
    pub fn current_work_unit_status(&self) -> Option<&str> {
        self.current_work_unit_status.as_deref()
    }

    /// Set the (work-unit-id, status) pair. Passing `(None, None)` clears
    /// both fields (mirrors TS `setCurrentWorkUnit(null, null)`).
    pub fn set_current_work_unit(
        &mut self,
        id: Option<String>,
        status: Option<String>,
    ) {
        self.current_work_unit_id = id;
        self.current_work_unit_status = status;
    }

    /// Is the create-session confirmation dialog currently requested?
    pub fn show_create_session_dialog(&self) -> bool {
        self.show_create_session_dialog
    }

    /// Has the user (or App::dispatch on `OpenAgentView(None)`)
    /// requested an immediate auto-creation? Mirrors TS
    /// `shouldAutoCreateSession`.
    pub fn should_auto_create_session(&self) -> bool {
        self.should_auto_create_session
    }

    /// Request the create-session dialog AND set the auto-create flag —
    /// the combined effect of `openCreateSessionDialog()` +
    /// `requestAutoCreateSession()` in the TS reference.
    pub fn request_create_session_dialog(&mut self) {
        self.show_create_session_dialog = true;
        self.should_auto_create_session = true;
    }

    /// Clear the dialog (and the auto-create request).
    pub fn clear_create_session_dialog(&mut self) {
        self.show_create_session_dialog = false;
        self.should_auto_create_session = false;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use codelet_rpc_types::SessionId;

    #[test]
    fn defaults_are_all_empty_or_false() {
        let store = AgentViewStore::default();
        assert!(store.current_session().is_none());
        assert!(store.navigation_target_session().is_none());
        assert!(store.current_work_unit_id().is_none());
        assert!(store.current_work_unit_status().is_none());
        assert!(!store.show_create_session_dialog());
        assert!(!store.should_auto_create_session());
    }

    #[test]
    fn take_navigation_target_returns_some_once_then_none() {
        let mut store = AgentViewStore::default();
        store.set_navigation_target(Some(SessionId::new("s-1")));
        assert_eq!(store.take_navigation_target(), Some(SessionId::new("s-1")));
        assert_eq!(store.take_navigation_target(), None);
    }

    #[test]
    fn request_create_session_dialog_sets_both_flags() {
        let mut store = AgentViewStore::default();
        store.request_create_session_dialog();
        assert!(store.show_create_session_dialog());
        assert!(store.should_auto_create_session());
        store.clear_create_session_dialog();
        assert!(!store.show_create_session_dialog());
        assert!(!store.should_auto_create_session());
    }

    #[test]
    fn set_current_work_unit_accepts_none_to_clear() {
        let mut store = AgentViewStore::default();
        store.set_current_work_unit(
            Some("AUTH-002".to_string()),
            Some("implementing".to_string()),
        );
        assert_eq!(store.current_work_unit_id(), Some("AUTH-002"));
        assert_eq!(store.current_work_unit_status(), Some("implementing"));
        store.set_current_work_unit(None, None);
        assert!(store.current_work_unit_id().is_none());
        assert!(store.current_work_unit_status().is_none());
    }
}
