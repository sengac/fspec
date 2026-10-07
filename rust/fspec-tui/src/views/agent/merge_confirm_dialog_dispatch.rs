//! TUI-112 — mouse + outcome routing for `MergeConfirmDialog`.
//!
//! Feature: spec/features/left-click-activates-buttons-rows-in-all-enter-selectable-modal-dialogs-new-agent-exit-session-etc.feature
//!
//! Splits `merge_confirm_dialog.rs` back under the 300-LoC ceiling:
//! the `Component::handle_event` override (keys → `handle_key`, the
//! pre-existing keyboard contract; `Down(Left)` → R2 click routing),
//! the terminal-outcome → `Action` mapping, and the self-remove
//! callback.

use crossterm::event::{Event, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::components::{Action, Callback, Component, EventResult};

use super::merge_confirm_dialog::{
    MergeConfirmDialog, MergeConfirmDialogOutcome, MERGE_CONFIRM_DIALOG_ID,
};

/// TUI-112: map a terminal outcome to the App `Action` that
/// dispatch_merge_worktree routes it into. `Continued` / `Ignored`
/// never reach here — `handle_event` handles them inline.
fn outcome_action(outcome: &MergeConfirmDialogOutcome) -> Action {
    match outcome {
        MergeConfirmDialogOutcome::Merge { session_id } => Action::MergeConfirmed {
            session_id: session_id.clone(),
        },
        MergeConfirmDialogOutcome::Discard { session_id } => Action::DiscardConfirmed {
            session_id: session_id.clone(),
        },
        MergeConfirmDialogOutcome::Cancel => Action::CancelMergeDialog,
        MergeConfirmDialogOutcome::Continued | MergeConfirmDialogOutcome::Ignored => {
            unreachable!("terminal-only outcomes are routed here")
        }
    }
}

fn merge_remove_callback() -> Callback {
    Box::new(|compositor| {
        let _ = compositor.remove(MERGE_CONFIRM_DIALOG_ID);
    })
}

impl MergeConfirmDialog {
    /// TUI-112: emit an outcome on the App's action channel (when
    /// attached) so `App::dispatch` routes it end-to-end.
    fn emit_outcome(&mut self, outcome: MergeConfirmDialogOutcome) -> EventResult {
        if let Some(tx) = self.action_tx.as_ref() {
            let _ = tx.send(outcome_action(&outcome));
        }
        self.pending_outcome = Some(outcome);
        EventResult::Consumed(Some(merge_remove_callback()))
    }

    /// TUI-112: route a left-button press (R2). A click on a button
    /// returns the SAME outcome as Left/Right + Enter on that button;
    /// a click off every button or outside the frame is `Ignored`
    /// (R4/R5 — the caller may route it elsewhere).
    pub fn handle_click(&self, col: u16, row: u16) -> MergeConfirmDialogOutcome {
        let layout = self.last_layout();
        if !layout.contains(col, row) {
            return MergeConfirmDialogOutcome::Ignored;
        }
        match layout.hit(col, row) {
            Some(idx) => self.outcome_for_index(idx),
            None => MergeConfirmDialogOutcome::Ignored,
        }
    }
}

impl Component for MergeConfirmDialog {
    fn id(&self) -> &str {
        MERGE_CONFIRM_DIALOG_ID
    }

    fn priority(&self) -> crate::components::Priority {
        crate::components::Priority::Foreground
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return EventResult::ignored();
            }
            match self.handle_key(key.code, key.modifiers) {
                MergeConfirmDialogOutcome::Continued => EventResult::consumed(),
                MergeConfirmDialogOutcome::Ignored => EventResult::ignored(),
                other => self.emit_outcome(other),
            }
        } else if let Event::Mouse(m) = event {
            if matches!(m.kind, MouseEventKind::Down(MouseButton::Left)) {
                match self.handle_click(m.column, m.row) {
                    MergeConfirmDialogOutcome::Ignored => EventResult::ignored(),
                    other => self.emit_outcome(other),
                }
            } else {
                EventResult::ignored()
            }
        } else {
            EventResult::ignored()
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        MergeConfirmDialog::render(self, area, buf);
    }
}
