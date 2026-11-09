//! TUI-128: a dialog on top owns every key press and paste.
//!
//! Feature: spec/features/keys-typed-in-a-dialog-never-reach-the-view-behind-it.feature
//!
//! Every compositor layer is a dialog (Foreground or Critical priority).
//! A key or paste no dialog used stops at the one on top: it never
//! reaches the view behind it (board, agent view, mux and its
//! Shift+Left/Right pane keys) or the app shortcuts. Ctrl+D still quits.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::components::{EventResult, Priority};

use crate::app::App;

impl App {
    /// True while a dialog is the topmost compositor layer.
    pub(crate) fn topmost_is_dialog(&self) -> bool {
        matches!(
            self.compositor.topmost_priority(),
            Some(Priority::Foreground | Priority::Critical)
        )
    }

    /// Stop a key no dialog used at the dialog on top (`None`: no dialog
    /// is open, or `event` is not a key). Ctrl+D still quits.
    pub(crate) fn stop_key_at_dialog(&mut self, event: &Event) -> Option<EventResult> {
        let Event::Key(key) = event else {
            return None;
        };
        if !self.topmost_is_dialog() {
            return None;
        }
        if is_quit_key(key) {
            self.should_quit = true;
        }
        Some(EventResult::consumed())
    }
}

/// Ctrl+D quits fspec from anywhere (a dialog that uses Ctrl+D itself,
/// such as the role dialog, consumes it first).
fn is_quit_key(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('d') && key.modifiers.contains(KeyModifiers::CONTROL)
}
