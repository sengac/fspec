//! BoardView shortcut keys (the menu registry's keys, TUI-129) kept out of
//! `views/board.rs` so it stays under the 300 LoC ceiling.
//!
//! Feature files:
//!   - spec/features/rpc356-changed-files-view.feature (RPC-356)
//!   - spec/features/rpc364-checkpoints-view.feature (RPC-364)
//!   - spec/features/rpc373-open-foundation.feature (RPC-373)
//!   - spec/features/rpc374-board-attachment-picker.feature (RPC-374)
//!   - spec/features/rpc395-new-agent-dot-key.feature (RPC-395)
//!   - spec/features/board-m-key-opens-the-mux-config-dialog-top-of-screen-mux-hint.feature (MUX-009)
//!   - spec/features/board-actions-popup-dialog-triggered-by-u-key.feature (BOARD-023)
//!   - spec/features/advertised-keys-do-what-their-hints-say.feature (TUI-129)

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::components::menu_bar::item_keys::shortcut_for_key;
use crate::components::menu_bar::items::MenuAction;
use crate::components::{Action, EventResult};

use super::BoardView;

/// Match the BoardView mode-view shortcuts against `key`.
///
/// TUI-129: every one-character key of the menu registry
/// (components/menu_bar/items.rs) runs its row's action, so the board,
/// the menu key column and the board help can never disagree; letters
/// match in either case and Ctrl/Alt chords fall through (e.g. the
/// App-level `Ctrl+D` hard-quit, RPC-102). A row that cannot act (no
/// attachments, no daemon directory) says why where its action runs.
///
/// Returns `Some(result)` when a shortcut claimed the key, `None` when
/// none matched (the caller keeps the navigation-key cascade running).
pub(super) fn handle_mode_view_key(view: &BoardView, key: &KeyEvent) -> Option<EventResult> {
    match shortcut_for_key(key) {
        // `?` stays with the App-level shortcut (app/events.rs), which
        // opens the same board help in every view.
        Some(MenuAction::Help) => return None,
        Some(action) => {
            view.emit(action.to_action());
            return Some(EventResult::consumed());
        }
        None => {}
    }
    match key.code {
        // BOARD-023 (re-purposed by MENU-005): 'u'/'U' opens the
        // menu-bar help dialog, on the single Board view and the focused
        // Board pane in mux mode. Not a menu row, so it keeps its own arm.
        KeyCode::Char('u') | KeyCode::Char('U')
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            view.emit(Action::OpenBoardKeybindingDialog);
            Some(EventResult::consumed())
        }
        _ => None,
    }
}
