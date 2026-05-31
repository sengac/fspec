//! RPC-096 — End-of-list navigation targets for AgentView Shift+Left/Right.
//!
//! Ports the TS `useSessionNavigation` hook semantics
//! (`src/tui/hooks/useSessionNavigation.ts`) to Rust:
//!   * Shift+Right past the last session resolves to
//!     [`NavTarget::CreateDialog`] — opens the Create Session modal
//!     (no auto-spawn).
//!   * Shift+Left past the first session resolves to
//!     [`NavTarget::Board`] — exits AgentView back to BoardView via
//!     `Action::BackToBoard`.
//!   * Anywhere mid-list resolves to [`NavTarget::Session`] — the
//!     RPC-024 draft round-trip is preserved.
//!
//! Lives in its own sibling module so `agent_view.rs` stays under the
//! 300-LoC ceiling pinned by `rpc024-source-shape.feature`.

use super::AgentViewStore;
use codelet_rpc_types::SessionId;

/// Where a Shift+Left/Right keypress should land.
///
/// Mirrors the TS `NavigationResult` discriminant
/// (`session` / `create-dialog` / `board`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavTarget {
    /// Switch focus to `open_sessions[index]`.
    Session(usize),
    /// Off the right end of the list — open the CreateSessionDialog
    /// modal (TS `openCreateSessionDialog`).
    CreateDialog,
    /// Off the left end of the list — exit AgentView back to BoardView
    /// (TS `clearActiveSession()` + `onNavigateToBoard()`).
    Board,
}

impl AgentViewStore {
    /// Resolve the next Shift+Right target without mutating state.
    ///
    /// Empty store → `CreateDialog`. At last index → `CreateDialog`.
    /// Otherwise → `Session(current_session_index + 1)`.
    pub fn navigate_next(&self) -> NavTarget {
        let len = self.open_sessions().len();
        if len == 0 {
            return NavTarget::CreateDialog;
        }
        let next = self.current_session_index() + 1;
        if next >= len {
            NavTarget::CreateDialog
        } else {
            NavTarget::Session(next)
        }
    }

    /// Resolve the next Shift+Left target without mutating state.
    ///
    /// Empty store → `Board`. At first index → `Board`. Otherwise →
    /// `Session(current_session_index - 1)`.
    pub fn navigate_prev(&self) -> NavTarget {
        let len = self.open_sessions().len();
        if len == 0 {
            return NavTarget::Board;
        }
        let cur = self.current_session_index();
        if cur == 0 {
            NavTarget::Board
        } else {
            NavTarget::Session(cur - 1)
        }
    }

    /// RPC-097 reopen #2: mirror TS `sessionGetFirst()` semantics —
    /// return the first open session id, if any. Consulted by
    /// `App::handle_open_agent_view(None)` (BoardView Shift+Right path)
    /// BEFORE falling through to mount `CreateSessionDialog`, so that
    /// an already-open session is resumed instead of re-prompting the
    /// user to create another. See:
    ///   spec/attachments/RPC-097/reopen2-active-session-list-not-checked.md
    pub fn first_open_session_id(&self) -> Option<SessionId> {
        self.open_sessions().first().map(|c| c.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::SessionContext;
    use codelet_rpc_types::SessionId;

    fn sid(s: &str) -> SessionId {
        SessionId::new(s)
    }

    #[test]
    fn empty_store_next_is_create_dialog_prev_is_board() {
        let store = AgentViewStore::default();
        assert!(matches!(store.navigate_next(), NavTarget::CreateDialog));
        assert!(matches!(store.navigate_prev(), NavTarget::Board));
    }

    #[test]
    fn last_index_next_is_create_dialog() {
        let mut store = AgentViewStore::default();
        store.append_session(SessionContext::new(sid("s-1")));
        store.append_session(SessionContext::new(sid("s-2")));
        // append_session focuses the new tail, so index == len - 1.
        assert_eq!(store.current_session_index(), 1);
        assert!(matches!(store.navigate_next(), NavTarget::CreateDialog));
        assert_eq!(store.navigate_prev(), NavTarget::Session(0));
    }
}
