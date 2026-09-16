//! RPC-020 — file search popup key routing (factored from
//! `file_search_popup.rs` to keep that file under the 300-LoC ceiling).
//!
//! Feature: spec/features/rpc020-slash-and-file-popups.feature

use crossterm::event::{KeyCode, KeyModifiers};

use super::file_search_popup::FilePopupOutcome;

/// Route a single key event through the popup's selection/scroll state.
/// Modifier keys (Shift/Ctrl) are ignored so the MultiLineInput keeps
/// handling them.
pub fn route_key(
    popup: &mut super::FileSearchPopup,
    code: KeyCode,
    mods: KeyModifiers,
) -> FilePopupOutcome {
    if mods.contains(KeyModifiers::SHIFT) || mods.contains(KeyModifiers::CONTROL) {
        return FilePopupOutcome::Ignored;
    }
    match code {
        KeyCode::Esc => FilePopupOutcome::Dismiss,
        KeyCode::Up => {
            popup.move_by(-1);
            FilePopupOutcome::Continued
        }
        KeyCode::Down => {
            popup.move_by(1);
            FilePopupOutcome::Continued
        }
        KeyCode::PageUp => {
            let step = -(popup.visible_rows() as i32);
            popup.move_by(step);
            FilePopupOutcome::Continued
        }
        KeyCode::PageDown => {
            let step = popup.visible_rows() as i32;
            popup.move_by(step);
            FilePopupOutcome::Continued
        }
        KeyCode::Home => {
            popup.reset_to_top();
            FilePopupOutcome::Continued
        }
        KeyCode::End => {
            popup.go_end();
            FilePopupOutcome::Continued
        }
        KeyCode::Enter => match popup.selected() {
            Some(path) => FilePopupOutcome::SelectedEnter(path.to_string()),
            None => FilePopupOutcome::Ignored,
        },
        KeyCode::Tab => match popup.selected() {
            Some(path) => FilePopupOutcome::SelectedTab(path.to_string()),
            None => FilePopupOutcome::Ignored,
        },
        _ => FilePopupOutcome::Ignored,
    }
}
