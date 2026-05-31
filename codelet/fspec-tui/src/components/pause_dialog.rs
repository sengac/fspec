//! PauseDialog — Priority::Critical modal for tool-pause approval prompts.
//!
//! Feature: spec/features/pause-and-hitl-dialogs.feature
//!
//! Renders one of two layouts depending on the `PauseState.kind`:
//!   - `PauseKind::Confirm` → 2-button (Accept / Deny) — Enter on the
//!     focused button emits `Action::PauseConfirmed { accept }`.
//!   - `PauseKind::Triple` → 3-button (Approve / Approve Session /
//!     Deny) — Enter on the focused button emits `Action::PauseTriple
//!     { choice }`.
//!
//! Esc on either variant emits `Action::PauseResumed { session_id }`.
//! All three commit-actions, plus Esc, also pop the dialog from the
//! Compositor via the canonical `PAUSE_DIALOG_ID` callback.

use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;

use codelet_rpc_types::{ApprovalChoice, PauseKind, PauseState, SessionId};

use super::dialog_theme::{
    render_dialog, Accent, DialogRow, FspecDialog, FOOTER_SEPARATOR,
};
use super::{Action, Callback, Component, EventResult, Priority};

/// Canonical id used by `Compositor::remove` when the dialog dismisses.
pub const PAUSE_DIALOG_ID: &str = "pause-dialog";

const CONFIRM_FOOTER: &str = "↑↓/Tab Cycle │ Enter Select │ Esc Resume";
const TRIPLE_FOOTER: &str = "↑↓/Tab Cycle │ Enter Select │ Esc Resume";

/// Logical button labels used by both kinds. Triple variants reorder.
const ACCEPT: &str = "Accept";
const DENY: &str = "Deny";
const APPROVE: &str = "Approve";
const APPROVE_SESSION: &str = "Approve Session";
const TRIPLE_DENY: &str = "Deny";

/// Priority::Critical modal dialog for tool-pause approval.
pub struct PauseDialog {
    id: String,
    session_id: SessionId,
    state: PauseState,
    focused: usize,
    action_tx: Option<UnboundedSender<Action>>,
    pending_action: Option<Action>,
}

impl PauseDialog {
    pub fn new(session_id: SessionId, state: PauseState) -> Self {
        Self {
            id: PAUSE_DIALOG_ID.to_string(),
            session_id,
            state,
            focused: 0,
            action_tx: None,
            pending_action: None,
        }
    }

    pub fn with_action_tx(mut self, action_tx: UnboundedSender<Action>) -> Self {
        self.action_tx = Some(action_tx);
        self
    }

    pub fn kind(&self) -> PauseKind {
        self.state.kind
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub fn focused_index(&self) -> usize {
        self.focused
    }

    /// Test accessor — currently focused button label.
    pub fn focused_label(&self) -> &'static str {
        let labels = self.button_labels();
        let idx = self.focused.min(labels.len().saturating_sub(1));
        labels[idx]
    }

    /// Test-only: drain any pending action stashed by `handle_event`
    /// when no `action_tx` was attached.
    pub fn take_pending_action(&mut self) -> Option<Action> {
        self.pending_action.take()
    }

    fn button_labels(&self) -> &'static [&'static str] {
        match self.state.kind {
            PauseKind::Confirm => &[ACCEPT, DENY],
            PauseKind::Triple => &[APPROVE, APPROVE_SESSION, TRIPLE_DENY],
        }
    }

    fn move_focus(&mut self, delta: i32) {
        let n = self.button_labels().len() as i32;
        if n == 0 {
            return;
        }
        let new = (self.focused as i32 + delta).rem_euclid(n) as usize;
        self.focused = new;
    }

    fn emit_action(&mut self, action: Action) {
        if let Some(tx) = self.action_tx.as_ref() {
            let _ = tx.send(action.clone());
        }
        self.pending_action = Some(action);
    }

    fn commit_focused_and_pop(&mut self) -> EventResult {
        let action = match (self.state.kind, self.focused) {
            (PauseKind::Confirm, 0) => Action::PauseConfirmed {
                session_id: self.session_id.clone(),
                accept: true,
            },
            (PauseKind::Confirm, _) => Action::PauseConfirmed {
                session_id: self.session_id.clone(),
                accept: false,
            },
            (PauseKind::Triple, 0) => Action::PauseTriple {
                session_id: self.session_id.clone(),
                choice: ApprovalChoice::Approve,
            },
            (PauseKind::Triple, 1) => Action::PauseTriple {
                session_id: self.session_id.clone(),
                choice: ApprovalChoice::ApproveSession,
            },
            (PauseKind::Triple, _) => Action::PauseTriple {
                session_id: self.session_id.clone(),
                choice: ApprovalChoice::Deny,
            },
        };
        self.emit_action(action);
        let id = self.id.clone();
        let callback: Callback = Box::new(move |compositor| {
            let _ = compositor.remove(&id);
        });
        EventResult::Consumed(Some(callback))
    }

    fn esc_and_pop(&mut self) -> EventResult {
        let action = Action::PauseResumed {
            session_id: self.session_id.clone(),
        };
        self.emit_action(action);
        let id = self.id.clone();
        let callback: Callback = Box::new(move |compositor| {
            let _ = compositor.remove(&id);
        });
        EventResult::Consumed(Some(callback))
    }

    fn prompt_rows(&self) -> Vec<DialogRow> {
        let style = Style::default().fg(Color::Gray).bg(Color::Black);
        self.state
            .prompt
            .lines()
            .map(|line| DialogRow {
                spans: vec![Span::styled(line.to_string(), style)],
                selectable: false,
                selected: false,
            })
            .collect()
    }

    fn buttons_row(&self) -> DialogRow {
        let labels = self.button_labels();
        let mut spans: Vec<Span<'static>> = Vec::new();
        for (i, label) in labels.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(FOOTER_SEPARATOR.to_string()));
            }
            let style = if i == self.focused {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Yellow)
                    .bg(Color::Black)
            };
            spans.push(Span::styled(format!("[ {label} ]"), style));
        }
        DialogRow {
            spans,
            selectable: false,
            selected: false,
        }
    }
}

impl Component for PauseDialog {
    fn priority(&self) -> Priority {
        Priority::Critical
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Esc => return self.esc_and_pop(),
                KeyCode::Enter => return self.commit_focused_and_pop(),
                KeyCode::Tab | KeyCode::Right => {
                    self.move_focus(1);
                    return EventResult::consumed();
                }
                KeyCode::BackTab | KeyCode::Left => {
                    self.move_focus(-1);
                    return EventResult::consumed();
                }
                KeyCode::Up => {
                    self.move_focus(-1);
                    return EventResult::consumed();
                }
                KeyCode::Down => {
                    self.move_focus(1);
                    return EventResult::consumed();
                }
                _ => {}
            }
        }
        if let Event::Mouse(m) = event {
            match m.kind {
                MouseEventKind::ScrollUp => {
                    self.move_focus(-1);
                    return EventResult::consumed();
                }
                MouseEventKind::ScrollDown => {
                    self.move_focus(1);
                    return EventResult::consumed();
                }
                _ => {}
            }
        }
        EventResult::ignored()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let mut rows = self.prompt_rows();
        rows.push(DialogRow {
            spans: vec![Span::raw(String::new())],
            selectable: false,
            selected: false,
        });
        rows.push(self.buttons_row());
        let title = match self.state.kind {
            PauseKind::Confirm => "Tool Pause — Confirm",
            PauseKind::Triple => "Tool Pause — Approval Required",
        };
        let footer = match self.state.kind {
            PauseKind::Confirm => CONFIRM_FOOTER,
            PauseKind::Triple => TRIPLE_FOOTER,
        };
        let dialog = FspecDialog {
            accent: Accent::Yellow,
            title,
            rows,
            footer,
            min_width: 50,
        };
        // Use the shared renderer; it computes its own dialog rect.
        render_dialog(area, buf, &dialog);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    #[test]
    fn confirm_default_focus_is_accept() {
        let mut dialog = PauseDialog::new(
            SessionId::new("s-1"),
            PauseState {
                kind: PauseKind::Confirm,
                prompt: "Run rm -rf /?".to_string(),
                tool_call_id: None,
            },
        );
        assert_eq!(dialog.focused_label(), "Accept");
        let _ = dialog.handle_event(&key(KeyCode::Tab));
        assert_eq!(dialog.focused_label(), "Deny");
    }

    #[test]
    fn triple_default_focus_is_approve_and_right_advances() {
        let mut dialog = PauseDialog::new(
            SessionId::new("s-1"),
            PauseState {
                kind: PauseKind::Triple,
                prompt: "Run scripted command?".to_string(),
                tool_call_id: None,
            },
        );
        assert_eq!(dialog.focused_label(), "Approve");
        let _ = dialog.handle_event(&key(KeyCode::Right));
        assert_eq!(dialog.focused_label(), "Approve Session");
        let _ = dialog.handle_event(&key(KeyCode::Right));
        assert_eq!(dialog.focused_label(), "Deny");
    }

    #[test]
    fn enter_on_confirm_accept_emits_confirmed_true() {
        let mut dialog = PauseDialog::new(
            SessionId::new("s-1"),
            PauseState {
                kind: PauseKind::Confirm,
                prompt: "p".to_string(),
                tool_call_id: None,
            },
        );
        let _ = dialog.handle_event(&key(KeyCode::Enter));
        match dialog.take_pending_action() {
            Some(Action::PauseConfirmed { session_id, accept }) => {
                assert_eq!(session_id, SessionId::new("s-1"));
                assert!(accept);
            }
            other => panic!("expected PauseConfirmed(true), got {other:?}"),
        }
    }

    #[test]
    fn esc_emits_pause_resumed() {
        let mut dialog = PauseDialog::new(
            SessionId::new("s-1"),
            PauseState {
                kind: PauseKind::Confirm,
                prompt: "p".to_string(),
                tool_call_id: None,
            },
        );
        let _ = dialog.handle_event(&key(KeyCode::Esc));
        match dialog.take_pending_action() {
            Some(Action::PauseResumed { session_id }) => {
                assert_eq!(session_id, SessionId::new("s-1"));
            }
            other => panic!("expected PauseResumed, got {other:?}"),
        }
    }
}
