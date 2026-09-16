//! BoardView mode-view key arms (f/c/m/d/a/.) extracted to keep
//! `views/board.rs` under the 300 LoC ceiling.
//!
//! Feature files:
//!   - spec/features/rpc356-changed-files-view.feature (RPC-356)
//!   - spec/features/rpc364-checkpoints-view.feature (RPC-364)
//!   - spec/features/rpc373-open-foundation.feature (RPC-373)
//!   - spec/features/rpc374-board-attachment-picker.feature (RPC-374)
//!   - spec/features/rpc395-new-agent-dot-key.feature (RPC-395)
//!   - spec/features/board-m-key-opens-the-mux-config-dialog-top-of-screen-mux-hint.feature (MUX-009)
//!
//! Mirrors the `mouse.rs` extraction: a free function that matches the
//! mode-view shortcut keys and emits the corresponding Actions onto the
//! bus. Returns `Some(result)` when an arm claimed the key (the caller
//! returns it verbatim), `None` when no arm matched (the caller falls
//! through to the navigation-key match in `BoardView::handle_event`).
//!
//! All arms are modifier-free-only: Ctrl-chorded keys fall through to
//! the App-level Stage-4 shortcuts (e.g. `Ctrl+D` hard-quit, RPC-102),
//! consistent with the existing a/c/f/d guard pattern.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::components::{Action, EventResult};
use crate::store::BoardStore;

use super::BoardView;

/// Match the BoardView mode-view shortcuts against `key`.
///
/// Returns `Some(result)` when an arm claimed the key, `None` when no
/// mode-view shortcut matched (the caller keeps the navigation-key
/// cascade running).
pub(super) fn handle_mode_view_key(
    view: &BoardView,
    key: &KeyEvent,
    store: &BoardStore,
) -> Option<EventResult> {
    match key.code {
        // RPC-356: open the dual-pane Changed Files view.
        KeyCode::Char('f') | KeyCode::Char('F')
            if !key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            view.emit(Action::OpenChangedFilesView);
            Some(EventResult::consumed())
        }
        // RPC-364: open the three-pane Checkpoints view.
        KeyCode::Char('c') | KeyCode::Char('C')
            if !key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            view.emit(Action::OpenCheckpointsView);
            Some(EventResult::consumed())
        }
        // MUX-009: modifier-free 'm'/'M' opens the MuxConfigDialog
        // (the same dialog bare /mux opens — MUX-004). Serves the
        // single Board view AND the focused Board pane in mux mode
        // (keyboard isolation routes Board-pane keys here). Modifier-
        // free only, so Ctrl+M falls through to App-level handling.
        KeyCode::Char('m') | KeyCode::Char('M')
            if !key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            view.emit(Action::OpenMuxConfigDialog);
            Some(EventResult::consumed())
        }
        // RPC-373: open FOUNDATION.md in the browser via the viewer server.
        // Modifier-free only: `Ctrl+D` is reserved as the App-level
        // hard-quit shortcut (RPC-102) and must fall through to Stage 4.
        KeyCode::Char('d') | KeyCode::Char('D')
            if !key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            view.emit(Action::OpenFoundation);
            Some(EventResult::consumed())
        }
        // RPC-374: open the attachment picker for the selected work unit.
        // Always consume the key; emit the picker action only when the
        // selected unit has at least one attachment (silent no-op otherwise).
        // Modifier-free only, so Ctrl-chorded keys fall through.
        KeyCode::Char('a') | KeyCode::Char('A')
            if !key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            if store
                .selected_work_unit()
                .is_some_and(|u| !u.attachments.is_empty())
            {
                view.emit(Action::OpenAttachmentPicker);
            }
            Some(EventResult::consumed())
        }
        // RPC-395: '.' starts a new agent — mirror of the Shift+Right
        // handler. Modifier-free so Ctrl-chorded keys fall through.
        KeyCode::Char('.') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let target = view.selected_session(store);
            view.emit(Action::OpenAgentView(target));
            Some(EventResult::consumed())
        }
        _ => None,
    }
}
