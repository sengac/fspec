//! MuxConfigDialog — Priority::Foreground modal for configuring the mux
//! grid layout (MUX-004).
//!
//! Feature: spec/features/mux-config-dialog.feature
//!
//! Mounted by the bare `/mux` slash command and the `/mux` slash-popup
//! pick via `App::handle_open_mux_config_dialog` (in
//! `app/dispatch_mux_config.rs`). The dialog seeds a DRAFT copy of the
//! live [`MuxConfig`] and edits it in place; the live grid is untouched
//! until commit (R5 cancel-safe).
//!
//! Body rows (R3, in order): `Enabled` (On/Off), `Orientation`
//! (Horizontal/Vertical), then one row per configured pane in grid
//! order showing the pane kind. A field cursor (usize over the
//! 2 + n_panes rows) drives Up/Down/wheel; Left/Right cycle the
//! highlighted row's value (R4).
//!
//! Commit (R5):
//! - `Enter` → `Action::MuxConfigApplied(draft)` + close.
//! - `s`     → `Action::MuxConfigAppliedAndSaved(draft)` + close.
//! - `Esc`   → close without applying or saving.
//!
//! Renders via the shared `dialog_theme::render_dialog` with
//! `Accent::Cyan`.
//!
//! Card: MUX-004.

use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use crate::views::multiplex::{MuxConfig, MuxOrientation, MuxPaneKind};

use super::mux_config_dialog_rows::{
    build_rows, MUX_CONFIG_DIALOG_FOOTER, MUX_CONFIG_DIALOG_TITLE,
};
use super::dialog_theme::{render_dialog, Accent, FspecDialog};
use super::{Action, Callback, Component, EventResult, Priority};

/// Canonical id used by `Compositor::remove` AND
/// `Compositor::contains` to address the dialog idempotently (R2).
pub const MUX_CONFIG_DIALOG_ID: &str = "mux-config-dialog";

/// Minimum / maximum pane count the dialog enforces (R4/R5).
const MIN_PANES: usize = 2;
const MAX_PANES: usize = 4;

/// Priority::Foreground modal dialog for configuring the mux layout.
pub struct MuxConfigDialog {
    id: String,
    /// The draft copy being edited — the live config until commit.
    draft: MuxConfig,
    /// Field cursor: 0 = Enabled, 1 = Orientation, 2+i = Pane i+1.
    cursor: usize,
    action_tx: Option<UnboundedSender<Action>>,
    pending_action: Option<Action>,
}

impl MuxConfigDialog {
    /// Construct a fresh dialog seeded from the live `config`.
    pub fn new(config: MuxConfig) -> Self {
        Self {
            id: MUX_CONFIG_DIALOG_ID.to_string(),
            draft: config,
            // Cursor starts on the Enabled row (index 0); the dialog
            // always has ≥ 3 rows (2 fixed + ≥ 1 pane).
            cursor: 0,
            action_tx: None,
            pending_action: None,
        }
    }

    /// Optional builder hook — wire the App's action channel so the
    /// dialog can emit follow-up actions in addition to stashing them in
    /// `pending_action`.
    pub fn with_action_tx(mut self, action_tx: UnboundedSender<Action>) -> Self {
        self.action_tx = Some(action_tx);
        self
    }

    /// Current draft config (test + dispatcher introspection).
    pub fn draft(&self) -> &MuxConfig {
        &self.draft
    }

    /// Test accessor — the current field cursor index.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Total number of rows the cursor spans (2 + n_panes).
    fn row_count(&self) -> usize {
        2 + self.draft.panes.len()
    }

    /// Test-only: drain any pending action stashed by `handle_event`.
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
        // Wrap: Up from the first row goes to the LAST pane row (R4).
        let n = self.row_count();
        self.cursor = if self.cursor == 0 { n - 1 } else { self.cursor - 1 };
    }

    fn move_down(&mut self) {
        // Wrap: Down from the last pane row goes back to Enabled (R4).
        let n = self.row_count();
        self.cursor = (self.cursor + 1) % n;
    }

    fn cycle_value(&mut self, forward: bool) {
        match self.cursor {
            0 => {
                // Enabled: On <-> Off (Left and Right both toggle).
                self.draft.enabled = !self.draft.enabled;
            }
            1 => {
                // Orientation: Horizontal <-> Vertical (Left and Right
                // both toggle).
                self.draft.orientation = match self.draft.orientation {
                    MuxOrientation::Horizontal => MuxOrientation::Vertical,
                    MuxOrientation::Vertical => MuxOrientation::Horizontal,
                };
            }
            _ => {
                // Pane row: cycle kind Board -> Agent -> Files ->
                // Checkpoints -> Board (forward) or the reverse.
                let idx = self.cursor - 2;
                let kinds: [MuxPaneKind; 4] = [
                    MuxPaneKind::Board,
                    MuxPaneKind::Agent,
                    MuxPaneKind::ChangedFiles,
                    MuxPaneKind::Checkpoints,
                ];
                let cur = self.draft.panes.get(idx).copied().unwrap_or_default();
                let pos = kinds.iter().position(|k| *k == cur).unwrap_or(0);
                let next = if forward {
                    (pos + 1) % kinds.len()
                } else {
                    (pos + kinds.len() - 1) % kinds.len()
                };
                if let Some(slot) = self.draft.panes.get_mut(idx) {
                    *slot = kinds[next];
                }
            }
        }
    }

    /// R4: append a new pane row (kind Board, max 4 panes).
    fn append_pane(&mut self) {
        if self.draft.panes.len() >= MAX_PANES {
            return;
        }
        self.draft.panes.push(MuxPaneKind::Board);
        // Move the cursor onto the new pane row.
        self.cursor = self.row_count() - 1;
    }

    /// R4: remove the highlighted pane row (min 2 panes). No-op when the
    /// cursor is not on a pane row or the minimum is reached.
    fn remove_pane(&mut self) {
        if self.draft.panes.len() <= MIN_PANES {
            return;
        }
        if self.cursor < 2 {
            return;
        }
        let idx = self.cursor - 2;
        if self.draft.panes.len() > idx {
            self.draft.panes.remove(idx);
        }
        // Keep the cursor on a valid (previous) row.
        self.cursor = self.cursor.min(self.row_count() - 1);
    }

    fn apply(&mut self, save: bool) -> EventResult {
        let draft = self.draft.clone();
        let action = if save {
            Action::MuxConfigAppliedAndSaved(draft)
        } else {
            Action::MuxConfigApplied(draft)
        };
        self.emit_action(action);
        EventResult::Consumed(Some(self.remove_callback()))
    }

    fn cancel(&self) -> EventResult {
        EventResult::Consumed(Some(self.remove_callback()))
    }
}

impl Component for MuxConfigDialog {
    fn priority(&self) -> Priority {
        Priority::Foreground
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        if let Event::Key(key) = event {
            // Only consume the documented keybindings (R4); everything
            // else is ignored so the compositor/Navigator can act.
            match key.code {
                KeyCode::Up => {
                    self.move_up();
                    return EventResult::consumed();
                }
                KeyCode::Down => {
                    self.move_down();
                    return EventResult::consumed();
                }
                KeyCode::Left => {
                    self.cycle_value(false);
                    return EventResult::consumed();
                }
                KeyCode::Right => {
                    self.cycle_value(true);
                    return EventResult::consumed();
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.append_pane();
                    return EventResult::consumed();
                }
                KeyCode::Backspace => {
                    self.remove_pane();
                    return EventResult::consumed();
                }
                KeyCode::Enter if key.modifiers.is_empty() => return self.apply(false),
                KeyCode::Char('s') | KeyCode::Char('S') => return self.apply(true),
                KeyCode::Esc => return self.cancel(),
                _ => {}
            }
            return EventResult::ignored();
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
        let rows = build_rows(&self.draft, self.cursor);
        let dialog = FspecDialog {
            accent: Accent::Cyan,
            title: MUX_CONFIG_DIALOG_TITLE,
            rows,
            footer: MUX_CONFIG_DIALOG_FOOTER,
            min_width: 46,
            query_row: None,
        };
        render_dialog(area, buf, &dialog);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crossterm::event::KeyModifiers;
    use crate::views::multiplex::MuxConfig;

    fn two_pane() -> MuxConfig {
        MuxConfig {
            orientation: MuxOrientation::Horizontal,
            splits: vec![50],
            panes: vec![MuxPaneKind::Board, MuxPaneKind::Agent],
            focused_pane: 1,
            enabled: false,
        }
    }

    #[test]
    fn new_seeds_draft_and_defaults_cursor_to_enabled() {
        let d = MuxConfigDialog::new(two_pane());
        assert_eq!(d.id(), MUX_CONFIG_DIALOG_ID);
        assert_eq!(d.priority(), Priority::Foreground);
        assert_eq!(d.cursor(), 0);
        assert!(!d.draft().enabled);
        assert_eq!(d.draft().panes.len(), 2);
    }

    #[test]
    fn up_from_enabled_wraps_to_last_pane_row() {
        let mut d = MuxConfigDialog::new(two_pane());
        // cursor 0 (Enabled); Up wraps to row_count-1 = 3 (Pane 2)
        let _ = d.handle_event(&crossterm::event::Event::Key(
            crossterm::event::KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
        ));
        assert_eq!(d.cursor(), 3);
    }

    #[test]
    fn down_from_last_pane_row_wraps_to_enabled() {
        let mut d = MuxConfigDialog::new(two_pane());
        d.cursor = 3;
        let _ = d.handle_event(&crossterm::event::Event::Key(
            crossterm::event::KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
        ));
        assert_eq!(d.cursor(), 0);
    }

    #[test]
    fn append_pane_respects_max_four() {
        let mut d = MuxConfigDialog::new(two_pane());
        // Add up to 4.
        for _ in 0..2 {
            let _ = d.handle_event(&crossterm::event::Event::Key(
                crossterm::event::KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
            ));
        }
        assert_eq!(d.draft().panes.len(), 4);
        // A fifth is rejected.
        let _ = d.handle_event(&crossterm::event::Event::Key(
            crossterm::event::KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
        ));
        assert_eq!(d.draft().panes.len(), 4);
    }

    #[test]
    fn remove_pane_respects_min_two() {
        let mut d = MuxConfigDialog::new(two_pane());
        // On a pane row (cursor 2 = Pane 1).
        d.cursor = 2;
        let _ = d.handle_event(&crossterm::event::Event::Key(
            crossterm::event::KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
        ));
        // Minimum of 2 holds.
        assert_eq!(d.draft().panes.len(), 2);
    }

    #[test]
    fn enter_emits_applied_and_esc_emits_nothing() {
        let mut d = MuxConfigDialog::new(two_pane());
        let _ = d.handle_event(&crossterm::event::Event::Key(
            crossterm::event::KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        ));
        assert!(matches!(
            d.take_pending_action(),
            Some(Action::MuxConfigApplied(_))
        ));
        let mut d2 = MuxConfigDialog::new(two_pane());
        let _ = d2.handle_event(&crossterm::event::Event::Key(
            crossterm::event::KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        ));
        assert!(d2.take_pending_action().is_none());
    }
}
