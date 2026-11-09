//! TUI-122 — Priority::Foreground modal for adding a workspace by path.
//!
//! Feature: spec/features/add-and-remove-workspaces-from-the-board-dropdown.feature
//!
//! Opened by `a` in the board workspace dropdown. A single-line text input
//! (like `RoleDialog`); Enter emits `Action::AddWorkspace(path)` and closes,
//! Esc cancels. When the daemon refuses the path the App reopens the dialog
//! seeded with that path and the daemon's reason. TUI-131: a path longer
//! than the dialog shows the part around the cursor (cut with `…`), and the
//! reason wraps to the dialog width.

use crossterm::event::{Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;
use tui_textarea::{Input, TextArea};

use super::daemon_form_dialog::scroll_window;
use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog, FOOTER_SEPARATOR};
use super::dialog_theme_rows::{dialog_body_width, wrapped_rows};
use super::{Action, Callback, Component, EventResult, Priority};

/// Compositor id of the add-workspace dialog.
pub const ADD_WORKSPACE_DIALOG_ID: &str = "add-workspace-dialog";

/// Path input for a new workspace, with the last refusal shown beneath it.
pub struct AddWorkspaceDialog {
    textarea: TextArea<'static>,
    error: Option<String>,
    action_tx: UnboundedSender<Action>,
}

impl AddWorkspaceDialog {
    /// A dialog seeded with `path` and, after a refusal, its `error`.
    pub fn new(path: &str, error: Option<String>, action_tx: UnboundedSender<Action>) -> Self {
        let mut textarea = TextArea::new(vec![path.to_owned()]);
        textarea.set_cursor_line_style(Style::default());
        textarea.move_cursor(tui_textarea::CursorMove::End);
        Self {
            textarea,
            error,
            action_tx,
        }
    }

    /// The typed path.
    pub fn draft(&self) -> String {
        self.textarea.lines().join("")
    }

    fn close() -> Callback {
        Box::new(|compositor| {
            let _ = compositor.remove(ADD_WORKSPACE_DIALOG_ID);
        })
    }

    fn submit(&self) -> EventResult {
        let path = self.draft().trim().to_owned();
        if path.is_empty() {
            return EventResult::consumed();
        }
        if self.action_tx.send(Action::AddWorkspace(path)).is_err() {
            tracing::error!("add-workspace dialog: action bus closed; the path was not submitted");
        }
        EventResult::Consumed(Some(Self::close()))
    }
}

impl Component for AddWorkspaceDialog {
    fn priority(&self) -> Priority {
        Priority::Foreground
    }

    fn id(&self) -> &str {
        ADD_WORKSPACE_DIALOG_ID
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Esc => EventResult::Consumed(Some(Self::close())),
                KeyCode::Enter => self.submit(),
                _ => {
                    let _ = self
                        .textarea
                        .input(Input::from(crossterm::event::KeyEvent::new(
                            key.code,
                            key.modifiers,
                        )));
                    EventResult::consumed()
                }
            },
            Event::Paste(text) => {
                // A path is one line: line breaks in a paste are dropped.
                let _ = self.textarea.insert_str(text.replace(['\r', '\n'], ""));
                EventResult::consumed()
            }
            _ => EventResult::ignored(),
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let width = dialog_body_width(area);
        let draft: Vec<char> = self.draft().chars().collect();
        let mut rows = vec![DialogRow {
            spans: vec![Span::raw(if draft.is_empty() {
                " ".to_owned()
            } else {
                scroll_window(&draft, self.textarea.cursor().1, width)
            })],
            selectable: false,
            selected: false,
        }];
        if let Some(error) = &self.error {
            rows.extend(wrapped_rows(error, Style::default().fg(Color::Red), width));
        }
        let sep = FOOTER_SEPARATOR;
        let footer = format!("Enter Add{sep}Esc Cancel");
        render_dialog(
            area,
            buf,
            &FspecDialog {
                accent: Accent::Cyan,
                title: "Add workspace (path to an fspec project)",
                rows,
                footer: footer.as_str(),
                min_width: 60,
                query_row: None,
            },
        );
    }
}
