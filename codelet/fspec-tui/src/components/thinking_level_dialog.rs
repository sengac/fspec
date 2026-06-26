//! ThinkingLevelDialog — Priority::Foreground modal for picking the
//! per-session thinking/reasoning level.
//!
//! Feature: spec/features/rpc022-thinking-level-dialog.feature
//! Feature: spec/features/rpc027-help-disconnect-thinking-dialogs.feature
//!
//! RPC-027: renders via the shared dialog_theme renderer with the
//! yellow accent. Adds the missing 'D Set Default' keybinding that
//! the TS reference has (ThinkingLevelDialog.tsx lines 93–96).

use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use codelet_rpc_types::{SessionId, ThinkingLevel};

use super::dialog_theme::{render_dialog, Accent, FspecDialog};
use super::dialog_theme_rows::label_description_default_row;
use super::{Action, Callback, Component, EventResult, Priority};

/// Canonical id used by `Compositor::remove`.
pub const THINKING_LEVEL_DIALOG_ID: &str = "thinking-level-dialog";

const LEVELS: [(ThinkingLevel, &str, &str); 4] = [
    (ThinkingLevel::Off, "Off", "No extended thinking"),
    (ThinkingLevel::Low, "Low", "~4K tokens, quick analysis"),
    (ThinkingLevel::Medium, "Medium", "~10K tokens, balanced"),
    (ThinkingLevel::High, "High", "~32K tokens, deep reasoning"),
];

const FOOTER: &str = "↑↓ Navigate │ Enter Select │ D Set Default │ Esc Close";

/// Priority::Foreground modal dialog for picking a thinking level.
pub struct ThinkingLevelDialog {
    id: String,
    session_id: SessionId,
    selected_index: usize,
    /// TUI-094: index in `LEVELS` of the persisted default level, or
    /// `None` when no default is set (TS-parity `defaultLevel === null`).
    default_index: Option<usize>,
    action_tx: Option<UnboundedSender<Action>>,
    pending_action: Option<Action>,
}

impl ThinkingLevelDialog {
    /// Construct a fresh dialog bound to `session_id` with
    /// `current_level` pre-highlighted.
    pub fn new(session_id: SessionId, current_level: ThinkingLevel) -> Self {
        let selected_index = LEVELS
            .iter()
            .position(|(l, _, _)| *l == current_level)
            .unwrap_or(0);
        Self {
            id: THINKING_LEVEL_DIALOG_ID.to_string(),
            session_id,
            selected_index,
            default_index: None,
            action_tx: None,
            pending_action: None,
        }
    }

    /// TUI-094: thread the persisted default level into the dialog
    /// (mirrors the nullable TS `defaultLevel` prop). `None` leaves the
    /// dialog with no `(default)` marker. Builder so `new` stays stable.
    pub fn with_default_level(mut self, default: Option<ThinkingLevel>) -> Self {
        self.default_index = default.and_then(|d| LEVELS.iter().position(|(l, _, _)| *l == d));
        self
    }

    pub fn with_action_tx(mut self, action_tx: UnboundedSender<Action>) -> Self {
        self.action_tx = Some(action_tx);
        self
    }

    /// Test accessor — currently highlighted level.
    pub fn selected_level(&self) -> ThinkingLevel {
        LEVELS[self.selected_index].0
    }

    /// Test accessor — currently highlighted row index.
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Test-only: drain any pending action stashed by `handle_event`
    /// when no `action_tx` was attached.
    pub fn take_pending_action(&mut self) -> Option<Action> {
        self.pending_action.take()
    }

    fn move_up(&mut self) {
        self.selected_index = if self.selected_index == 0 {
            LEVELS.len() - 1
        } else {
            self.selected_index - 1
        };
    }

    fn move_down(&mut self) {
        self.selected_index = (self.selected_index + 1) % LEVELS.len();
    }

    fn emit_action(&mut self, action: Action) {
        if let Some(tx) = self.action_tx.as_ref() {
            let _ = tx.send(action.clone());
        }
        self.pending_action = Some(action);
    }
}

impl Component for ThinkingLevelDialog {
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
                    let id = self.id.clone();
                    let callback: Callback = Box::new(move |compositor| {
                        let _ = compositor.remove(&id);
                    });
                    return EventResult::Consumed(Some(callback));
                }
                KeyCode::Up => {
                    self.move_up();
                    return EventResult::consumed();
                }
                KeyCode::Down => {
                    self.move_down();
                    return EventResult::consumed();
                }
                KeyCode::Enter => {
                    let level = self.selected_level();
                    let action = Action::ThinkingLevelSelected(self.session_id.clone(), level);
                    self.emit_action(action);
                    let id = self.id.clone();
                    let callback: Callback = Box::new(move |compositor| {
                        let _ = compositor.remove(&id);
                    });
                    return EventResult::Consumed(Some(callback));
                }
                // RPC-027: 'D' / 'd' sets the per-user default level
                // without closing the dialog (parity with the TS
                // ThinkingLevelDialog.tsx lines 93-96).
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    let level = self.selected_level();
                    self.default_index = Some(self.selected_index); // live marker move (TS parity)
                    let action = Action::SetThinkingLevelDefault(self.session_id.clone(), level);
                    self.emit_action(action);
                    return EventResult::consumed();
                }
                _ => {}
            }
        }
        // RPC-028: mouse-wheel advances/retreats the selection like the
        // arrow keys. The dialog renders centered so we don't bother
        // hit-testing — wheel events while the dialog is topmost belong
        // to it.
        if let Event::Mouse(m) = event {
            match m.kind {
                MouseEventKind::ScrollUp => {
                    self.move_up();
                    return EventResult::consumed();
                }
                MouseEventKind::ScrollDown => {
                    self.move_down();
                    return EventResult::consumed();
                }
                _ => {}
            }
        }
        EventResult::ignored()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let rows = LEVELS
            .iter()
            .enumerate()
            .map(|(i, (_, label, desc))| {
                label_description_default_row(
                    label,
                    desc,
                    i == self.selected_index,
                    Some(i) == self.default_index,
                )
            })
            .collect();
        let dialog = FspecDialog {
            accent: Accent::Yellow,
            title: "Thinking Level",
            rows,
            footer: FOOTER,
            min_width: 50,
        };
        render_dialog(area, buf, &dialog);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn thinking_level_dialog_rendering_is_byte_equal_across_runs_insta_snapshot() {
        let session_id = SessionId {
            value: "test-session".to_string(),
        };
        let mut dialog = ThinkingLevelDialog::new(session_id, ThinkingLevel::Off);
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
        terminal
            .draw(|frame| {
                dialog.render(frame.area(), frame.buffer_mut());
            })
            .expect("draw");
        let buf = terminal.backend().buffer().clone();
        let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
        for y in 0..buf.area.height {
            let mut row = String::with_capacity(buf.area.width as usize);
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            rows.push(row);
        }
        insta::assert_yaml_snapshot!("thinking_level_dialog__centered_popup_80x24", rows);
    }
}
