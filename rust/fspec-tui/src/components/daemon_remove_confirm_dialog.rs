//! TUI-123 — Priority::Foreground y/n prompt before a saved daemon is
//! removed.
//!
//! Feature: spec/features/daemon-management-on-par-with-workspaces.feature
//!
//! "Remove saved daemon <name> and its stored token? y/n": `y` emits
//! `Action::ConfirmRemoveSavedDaemon`, any other key cancels (product
//! owner, 2026-11-05).

use crossterm::event::{Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;

use super::daemon_form_dialog::wrap_text;
use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog, FOOTER_SEPARATOR};
use super::{Action, Callback, Component, EventResult, Priority};
use crate::daemons::DaemonChoice;

/// Compositor id of the removal prompt.
pub const DAEMON_REMOVE_CONFIRM_DIALOG_ID: &str = "daemon-remove-confirm-dialog";

/// Content width of the prompt when the terminal allows it.
const PROMPT_WIDTH: u16 = 60;

/// Asks before the saved daemon `choice` is removed.
pub struct DaemonRemoveConfirmDialog {
    choice: DaemonChoice,
    action_tx: UnboundedSender<Action>,
}

impl DaemonRemoveConfirmDialog {
    pub fn new(choice: DaemonChoice, action_tx: UnboundedSender<Action>) -> Self {
        Self { choice, action_tx }
    }

    fn close() -> Callback {
        Box::new(|compositor| {
            let _ = compositor.remove(DAEMON_REMOVE_CONFIRM_DIALOG_ID);
        })
    }
}

impl Component for DaemonRemoveConfirmDialog {
    fn priority(&self) -> Priority {
        Priority::Foreground
    }

    fn id(&self) -> &str {
        DAEMON_REMOVE_CONFIRM_DIALOG_ID
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        match event {
            Event::Key(key) => {
                if matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y')) {
                    let action = Action::ConfirmRemoveSavedDaemon(self.choice.clone());
                    if self.action_tx.send(action).is_err() {
                        tracing::error!("daemon removal: action bus closed; nothing was removed");
                    }
                }
                EventResult::Consumed(Some(Self::close()))
            }
            // Modal: nothing behind the prompt reacts until it is answered.
            _ => EventResult::consumed(),
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let width = usize::from(area.width.min(PROMPT_WIDTH + 4).saturating_sub(4));
        let prompt = format!(
            "Remove saved daemon {} and its stored token? y/n",
            self.choice.name
        );
        let rows = wrap_text(&prompt, width)
            .into_iter()
            .map(|line| DialogRow {
                spans: vec![Span::raw(line)],
                selectable: false,
                selected: false,
            })
            .collect();
        let footer = format!("y Remove{FOOTER_SEPARATOR}any other key Cancel");
        render_dialog(
            area,
            buf,
            &FspecDialog {
                accent: Accent::Yellow,
                title: "Remove saved daemon",
                rows,
                footer: footer.as_str(),
                min_width: PROMPT_WIDTH,
                query_row: None,
            },
        );
    }
}
