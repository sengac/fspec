//! TUI-112 — mouse + key routing for `SessionWorktreesDialog`.
//!
//! Feature: spec/features/left-click-activates-buttons-rows-in-all-enter-selectable-modal-dialogs-new-agent-exit-session-etc.feature
//!
//! Splits `session_worktrees_dialog.rs` back under the 300-LoC ceiling:
//! the `Component::handle_event` override — keys through
//! `handle_key` (the pre-existing keyboard contract) and `Down(Left)`
//! through the TUI-112 R2 click routing (a click on a button emits that
//! button's action and removes the dialog; anything else is Ignored so
//! it bubbles to the view behind the modal).

use crossterm::event::{Event, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::components::{Action, Callback, Component, EventResult, Priority};

use super::session_worktrees_dialog::{
    SessionWorktreesDialog, SessionWorktreesDialogOutcome, SESSION_WORKTREES_DIALOG_ID,
};

fn worktrees_remove_callback() -> Callback {
    Box::new(|compositor| {
        let _ = compositor.remove(SESSION_WORKTREES_DIALOG_ID);
    })
}

fn outcome_action(outcome: SessionWorktreesDialogOutcome) -> Action {
    match outcome {
        SessionWorktreesDialogOutcome::Prune => Action::PruneLeakedWorktrees,
        _ => Action::CancelWorktreesDialog,
    }
}

impl Component for SessionWorktreesDialog {
    fn id(&self) -> &str {
        SESSION_WORKTREES_DIALOG_ID
    }

    fn priority(&self) -> Priority {
        Priority::Foreground
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        // TUI-112 (R2): a left-click on a button emits that button's
        // action (Prune / Cancel) and removes the dialog; a click off
        // every button or outside the frame is Ignored (R4/R5) so it
        // bubbles to the view behind the modal. Wheel behavior is
        // unchanged (Ignored — the dialog never handled wheel).
        if let Event::Mouse(m) = event {
            if matches!(m.kind, MouseEventKind::Down(MouseButton::Left)) {
                let layout = self.last_layout().clone();
                if !layout.contains(m.column, m.row) {
                    return EventResult::ignored();
                }
                match layout.hit(m.column, m.row) {
                    Some(idx) => {
                        self.emit_action(&outcome_action(self.outcome_for_index(idx)));
                        EventResult::Consumed(Some(worktrees_remove_callback()))
                    }
                    _ => EventResult::ignored(),
                }
            } else {
                EventResult::ignored()
            }
        } else if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return EventResult::ignored();
            }
            let outcome = self.handle_key(key.code, key.modifiers);
            match outcome {
                SessionWorktreesDialogOutcome::Prune | SessionWorktreesDialogOutcome::Cancel => {
                    self.emit_action(&outcome_action(outcome));
                    EventResult::Consumed(Some(worktrees_remove_callback()))
                }
                SessionWorktreesDialogOutcome::Continued => EventResult::consumed(),
                SessionWorktreesDialogOutcome::Ignored => EventResult::ignored(),
            }
        } else {
            EventResult::ignored()
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        SessionWorktreesDialog::render(self, area, buf);
    }
}
