//! TUI-130: c and Ctrl+C copy the whole text of the dialog on top.
//!
//! Feature: spec/features/copy-works-everywhere-text-is-shown.feature
//!
//! A dialog offers its text through `Component::copy_text` (the Error,
//! Session error and Notification dialogs, the board card details). The App
//! copies it through the one copy path, `Action::CopyToClipboard`, so the copy
//! is confirmed like every other copy, and the key never reaches the view
//! behind the dialog. The copy holds the dialog's whole text, never only the
//! rows on screen.

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

use super::state::App;
use crate::components::{Action, EventResult};

impl App {
    /// Copy the topmost dialog's text on c or Ctrl+C (`None`: not a copy
    /// key, or the dialog on top has no text to copy).
    pub(crate) fn copy_from_dialog(&mut self, event: &Event) -> Option<EventResult> {
        let Event::Key(key) = event else {
            return None;
        };
        let copy_key = key.code == KeyCode::Char('c')
            && (key.modifiers.is_empty() || key.modifiers == KeyModifiers::CONTROL);
        if key.kind != KeyEventKind::Press || !copy_key {
            return None;
        }
        let text = self.compositor.topmost_copy_text()?;
        self.dispatch(Action::CopyToClipboard(text));
        self.should_render = true;
        Some(EventResult::consumed())
    }
}
