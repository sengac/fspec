//! AttachmentPickerDialog — Priority::Foreground modal listing the selected
//! work unit's attachments (RPC-374).
//!
//! Feature: spec/features/rust-board-open-attachment.feature
//! Card: RPC-374 (parent RPC-371).
//!
//! Modeled on `create_session_dialog.rs` / `checkpoint_restore_dialog.rs`:
//! a yellow-accent modal that renders one selectable row per attachment,
//! showing the BASENAME for readability while preserving the full path for
//! the launch URL. Up/Down move the selection (clamped — it never wraps, so
//! the first/last row stay reachable predictably). Enter emits
//! `Action::OpenAttachment(full_path)` then pops the dialog; Esc pops.

use crossterm::event::{Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;

use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog};
use super::{Action, Callback, Component, EventResult, Priority};

/// Canonical id used by `Compositor::remove`.
pub const ATTACHMENT_PICKER_DIALOG_ID: &str = "attachment-picker-dialog";

const ACCENT: Accent = Accent::Yellow;
const FOOTER: &str = "↑ ↓ Select | Enter Open | Esc Cancel";
const MIN_WIDTH: u16 = 40;

/// Derive the displayed basename from a `/`-separated attachment path.
/// Keeps the original string when there is no separator.
fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Priority::Foreground modal dialog for picking one of a work unit's
/// attachments to open in the browser.
pub struct AttachmentPickerDialog {
    id: String,
    attachments: Vec<String>,
    selected: usize,
    action_tx: Option<UnboundedSender<Action>>,
}

impl AttachmentPickerDialog {
    /// Construct a fresh picker over the supplied full attachment paths,
    /// preserving order. Selection starts at the first row.
    pub fn new(attachments: Vec<String>) -> Self {
        Self {
            id: ATTACHMENT_PICKER_DIALOG_ID.to_string(),
            attachments,
            selected: 0,
            action_tx: None,
        }
    }

    /// Builder-style action_tx attach for the App's UnboundedSender.
    pub fn with_action_tx(mut self, tx: UnboundedSender<Action>) -> Self {
        self.action_tx = Some(tx);
        self
    }

    /// Test accessor — the basenames rendered, one per attachment, in order.
    pub fn row_labels(&self) -> Vec<String> {
        self.attachments
            .iter()
            .map(|p| basename(p).to_string())
            .collect()
    }

    /// Test accessor — the currently highlighted row index.
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn move_down(&mut self) {
        if self.selected + 1 < self.attachments.len() {
            self.selected += 1;
        }
    }

    fn emit(&self, action: Action) {
        if let Some(tx) = self.action_tx.as_ref() {
            let _ = tx.send(action);
        }
    }

    fn remove_callback(&self) -> Callback {
        let id = self.id.clone();
        Box::new(move |compositor| {
            let _ = compositor.remove(&id);
        })
    }
}

impl Component for AttachmentPickerDialog {
    fn priority(&self) -> Priority {
        Priority::Foreground
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Esc => {
                    return EventResult::Consumed(Some(self.remove_callback()));
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.move_up();
                    return EventResult::consumed();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.move_down();
                    return EventResult::consumed();
                }
                KeyCode::Enter => {
                    if let Some(path) = self.attachments.get(self.selected) {
                        self.emit(Action::OpenAttachment(path.clone()));
                    }
                    return EventResult::Consumed(Some(self.remove_callback()));
                }
                _ => {}
            }
        }
        EventResult::ignored()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let rows: Vec<DialogRow> = self
            .attachments
            .iter()
            .enumerate()
            .map(|(i, path)| DialogRow {
                spans: vec![Span::raw(basename(path).to_string())],
                selectable: true,
                selected: i == self.selected,
            })
            .collect();
        let dialog = FspecDialog {
            accent: ACCENT,
            title: "Open Attachment",
            rows,
            footer: FOOTER,
            min_width: MIN_WIDTH,
query_row: None,
        };
        render_dialog(area, buf, &dialog);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn row_labels_render_basenames_in_order() {
        let dialog = AttachmentPickerDialog::new(vec![
            "spec/attachments/RPC-001/design.md".to_string(),
            "spec/attachments/RPC-001/a b.md".to_string(),
        ]);
        assert_eq!(
            dialog.row_labels(),
            vec!["design.md".to_string(), "a b.md".to_string()]
        );
    }

    #[test]
    fn down_clamps_at_the_last_row_and_up_clamps_at_the_first() {
        let mut dialog = AttachmentPickerDialog::new(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(dialog.selected_index(), 0);
        dialog.move_up();
        assert_eq!(dialog.selected_index(), 0);
        dialog.move_down();
        assert_eq!(dialog.selected_index(), 1);
        dialog.move_down();
        assert_eq!(dialog.selected_index(), 1);
    }
}
