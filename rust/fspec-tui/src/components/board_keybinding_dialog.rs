//! BOARD-023 — BoardKeybindingDialog: the 'u' actions popup.
//!
//! Feature: spec/features/board-actions-popup-dialog-triggered-by-u-key.feature
//!
//! Priority::Foreground modal pushed onto the Compositor by
//! `App::handle_open_board_keybinding_dialog` (BOARD-023, R1/R2).
//!
//! Body rows (R3, in order): the six board shortcuts from the single
//! source of truth `views::board::board_shortcuts::SHORTCUTS`
//! (C Checkpoints, F Changed Files, D FOUNDATION.md, . New Agent,
//! / Search, M Mux), then `? Help` (emits `Action::OpenBoardHelp`) and
//! `Esc Exit` (emits `Action::OpenBoardExitConfirmation`). There is NO
//! trigger row — the dialog is already open and Esc closes it (R8); the
//! 'u' trigger is advertised in the header hint and the board help lines
//! instead (R6/R10).
//!
//! Navigation: `↑`/`↓` move the cursor with wrap-around (R4); the
//! mouse wheel drives the same movement (MuxConfigDialog R4 parity).
//!
//! Commit (R5/R6):
//! - `Enter` → emit the SAME Action the bare board key would emit
//!   (or the OpenBoardHelp / OpenBoardExitConfirmation variants for
//!   the last two rows) + close.
//! - `Esc` → close WITHOUT executing the highlighted row (R8).
//!
//! True-modal key blocking (R7, BUG-161 parity with
//! WorkUnitSearchDialog): every key this dialog does not explicitly
//! handle — including `'u'`/`'U'` (R2 idempotent no-op), `j/k/h/l`,
//! `[`/`]`, Shift/Ctrl chords, and pastes — is CONSUMED as a no-op so the
//! BoardView behind it stays frozen.
//!
//! Renders via the shared `dialog_theme::render_dialog` with
//! `Accent::Yellow` (informational popups: attachment picker, thinking
//! level, exit confirmation). The title reads "Actions" (R12).
//!
//! Card: BOARD-023.

use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use codelet_rpc_types::SessionId;

use crate::views::board::board_shortcuts::SHORTCUTS;

use super::dialog_theme::{render_dialog, Accent, FspecDialog};
use super::dialog_theme_rows::label_description_row;
use super::{Action, Callback, Component, EventResult, Priority};

/// Canonical id used by `Compositor::remove` AND
/// `Compositor::contains` to address the dialog idempotently (R2).
pub const BOARD_KEYBINDING_DIALOG_ID: &str = "board-actions-dialog";

const ACCENT: Accent = Accent::Yellow;
const TITLE: &str = "Actions";
const FOOTER: &str = "↑↓ Navigate │ Enter Execute │ Esc Close";
const MIN_WIDTH: u16 = 48;

/// The two extra rows appended after the six bare-keyboard shortcuts.
const HELP_LABEL: &str = "? Help";
const EXIT_LABEL: &str = "Esc Exit";

/// Total row count (R3: six shortcuts + Help + Exit — no trigger row).
const ROW_COUNT: usize = 8;

/// Priority::Foreground modal dialog listing every board shortcut with
/// a one-line description.
pub struct BoardKeybindingDialog {
    id: String,
    /// Cursor: index into the 8 rows (0 = `C Checkpoints`).
    cursor: usize,
    /// The `.` New Agent session target snapshotted at dialog open (R5)
    /// — the modal blocks board selection while open, so the snapshot
    /// stays valid for the dialog's whole lifetime.
    new_agent_target: Option<SessionId>,
    action_tx: Option<UnboundedSender<Action>>,
    pending_action: Option<Action>,
}

impl BoardKeybindingDialog {
    /// Construct a fresh dialog. `new_agent_target` is the `Option`
    /// session target `BoardView::selected_session` produced at open.
    pub fn new(new_agent_target: Option<SessionId>) -> Self {
        Self {
            id: BOARD_KEYBINDING_DIALOG_ID.to_string(),
            cursor: 0,
            new_agent_target,
            action_tx: None,
            pending_action: None,
        }
    }

    /// Optional builder hook — wire the App's action channel so the
    /// dialog can emit follow-up actions in addition to stashing them
    /// in `pending_action` (test seam).
    pub fn with_action_tx(mut self, action_tx: UnboundedSender<Action>) -> Self {
        self.action_tx = Some(action_tx);
        self
    }

    /// Test accessor — the current cursor index.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Test-only: drain any pending action stashed by `handle_event`
    /// when no `action_tx` was attached.
    pub fn take_pending_action(&mut self) -> Option<Action> {
        self.pending_action.take()
    }

    fn emit_action(&mut self, action: Action) {
        if let Some(tx) = self.action_tx.as_ref() {
            let _ = tx.send(action.clone());
        }
        self.pending_action = Some(action);
    }

    fn remove_callback(&self) -> Callback {
        let id = self.id.clone();
        Box::new(move |compositor| {
            let _ = compositor.remove(&id);
        })
    }

    fn move_up(&mut self) {
        // Wrap: Up from the first row goes to the LAST row (R4).
        self.cursor = if self.cursor == 0 {
            ROW_COUNT - 1
        } else {
            self.cursor - 1
        };
    }

    fn move_down(&mut self) {
        // Wrap: Down from the last row goes back to the first (R4).
        self.cursor = (self.cursor + 1) % ROW_COUNT;
    }

    /// R5/R6: resolve the Enter action for the highlighted row.
    fn enter_action(&self) -> Option<Action> {
        match self.cursor {
            0..=5 => {
                let entry = SHORTCUTS.get(self.cursor)?;
                let base = entry.action.clone();
                // R5: the '.' New Agent row emits OpenAgentView with
                // the session target snapshotted at dialog open — the
                // table carries `None` as a placeholder.
                let action = if let crate::views::board::board_shortcuts::BoardShortcutAction::
                    OpenAgentView(_) = base
                {
                    Action::OpenAgentView(self.new_agent_target.clone())
                } else {
                    base.to_action()
                };
                Some(action)
            }
            // R6: the ? Help / Esc Exit rows emit the new variants.
            6 => Some(Action::OpenBoardHelp),
            7 => Some(Action::OpenBoardExitConfirmation),
            _ => None,
        }
    }

    fn enter(&mut self) -> EventResult {
        if let Some(action) = self.enter_action() {
            self.emit_action(action);
        }
        EventResult::Consumed(Some(self.remove_callback()))
    }

    fn cancel(&self) -> EventResult {
        EventResult::Consumed(Some(self.remove_callback()))
    }
}

impl Component for BoardKeybindingDialog {
    fn priority(&self) -> Priority {
        Priority::Foreground
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        // RPC-403 review: swallow pastes so they can never leak into
        // the board hidden behind this modal (no text field here).
        if matches!(event, Event::Paste(_)) {
            return EventResult::consumed();
        }
        if let Event::Key(key) = event {
            // R7 (BUG-161 parity): Shift/Ctrl chords are CONSUMED as
            // no-ops so chords like Shift+Right cannot leak through the
            // Compositor to the BoardView or App-level handler behind.
            if key
                .modifiers
                .contains(crossterm::event::KeyModifiers::SHIFT)
                || key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
            {
                return EventResult::consumed();
            }
            match key.code {
                // R8: Esc ALWAYS closes — never executes the row.
                KeyCode::Esc => return self.cancel(),
                KeyCode::Up => {
                    self.move_up();
                    return EventResult::consumed();
                }
                KeyCode::Down => {
                    self.move_down();
                    return EventResult::consumed();
                }
                KeyCode::Enter if key.modifiers.is_empty() => return self.enter(),
                // R2/R7: 'u'/'U' while open is a consumed no-op
                // (idempotent — never stacks a second layer).
                KeyCode::Char('u') | KeyCode::Char('U') => return EventResult::consumed(),
                // R7: true-modal — every other key is swallowed.
                _ => return EventResult::consumed(),
            }
        }
        // R4: mouse wheel scrolls the cursor like Up/Down.
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
        let rows: Vec<crate::components::dialog_theme::DialogRow> = {
            let mut rows = SHORTCUTS
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    label_description_row(
                        &format!("{} {}", s.key, s.label),
                        s.description,
                        i == self.cursor,
                    )
                })
                .collect::<Vec<_>>();
            rows.push(label_description_row(
                HELP_LABEL,
                "Show the full board keybinding help",
                6 == self.cursor,
            ));
            rows.push(label_description_row(
                EXIT_LABEL,
                "Confirm exiting fspec",
                7 == self.cursor,
            ));
            rows
        };
        let dialog = FspecDialog {
            accent: ACCENT,
            title: TITLE,
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

    fn key(code: KeyCode) -> Event {
        Event::Key(crossterm::event::KeyEvent::new(
            code,
            crossterm::event::KeyModifiers::NONE,
        ))
    }

    #[test]
    fn new_defaults_cursor_to_first_row() {
        let d = BoardKeybindingDialog::new(None);
        assert_eq!(d.id(), BOARD_KEYBINDING_DIALOG_ID);
        assert_eq!(d.priority(), Priority::Foreground);
        assert_eq!(d.cursor(), 0);
    }

    #[test]
    fn up_from_first_row_wraps_to_last_and_down_wraps_back() {
        let mut d = BoardKeybindingDialog::new(None);
        let _ = d.handle_event(&key(KeyCode::Up));
        assert_eq!(d.cursor(), 7);
        let _ = d.handle_event(&key(KeyCode::Down));
        assert_eq!(d.cursor(), 0);
    }

    #[test]
    fn enter_on_row_zero_emits_open_checkpoints_view() {
        let mut d = BoardKeybindingDialog::new(None);
        let _ = d.handle_event(&key(KeyCode::Enter));
        assert!(matches!(
            d.take_pending_action(),
            Some(Action::OpenCheckpointsView)
        ));
    }

    #[test]
    fn enter_on_new_agent_row_carries_the_snapshot() {
        let sid = SessionId::new("s-42");
        let mut d = BoardKeybindingDialog::new(Some(sid.clone()));
        for _ in 0..3 {
            let _ = d.handle_event(&key(KeyCode::Down));
        }
        let _ = d.handle_event(&key(KeyCode::Enter));
        assert!(matches!(
            d.take_pending_action(),
            Some(Action::OpenAgentView(Some(s))) if s == sid
        ));
    }

    #[test]
    fn enter_on_help_and_exit_rows_emit_the_new_variants() {
        let mut d = BoardKeybindingDialog::new(None);
        for _ in 0..6 {
            let _ = d.handle_event(&key(KeyCode::Down));
        }
        let _ = d.handle_event(&key(KeyCode::Enter));
        assert!(matches!(
            d.take_pending_action(),
            Some(Action::OpenBoardHelp)
        ));
        let mut d2 = BoardKeybindingDialog::new(None);
        for _ in 0..7 {
            let _ = d2.handle_event(&key(KeyCode::Down));
        }
        let _ = d2.handle_event(&key(KeyCode::Enter));
        assert!(matches!(
            d2.take_pending_action(),
            Some(Action::OpenBoardExitConfirmation)
        ));
    }

    #[test]
    fn u_and_unhandled_keys_are_consumed_no_op() {
        let mut d = BoardKeybindingDialog::new(None);
        assert!(d.handle_event(&key(KeyCode::Char('u'))).is_consumed());
        assert!(d.handle_event(&key(KeyCode::Char('U'))).is_consumed());
        assert!(d.handle_event(&key(KeyCode::Char('j'))).is_consumed());
        assert!(d.handle_event(&key(KeyCode::Char('f'))).is_consumed());
        assert!(d.handle_event(&key(KeyCode::Char('?'))).is_consumed());
        assert_eq!(d.cursor(), 0);
        assert!(d.take_pending_action().is_none());
    }

    #[test]
    fn esc_closes_without_emitting_any_action() {
        let mut d = BoardKeybindingDialog::new(None);
        assert!(d.handle_event(&key(KeyCode::Esc)).is_consumed());
        assert!(d.take_pending_action().is_none());
    }
}
