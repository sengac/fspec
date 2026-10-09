//! MENU-005 — MenuBarHelpDialog: the 'u' menu-bar help overlay.
//!
//! Feature: spec/features/u-menu-bar-help-re-purpose-help-content-snapshot-shape-test-migration-tag.feature
//!
//! BOARD-023's 'u' actions popup re-purposed as the menu-bar help
//! overlay: the body rows are generated from the FULL MenuCategories
//! registry (`components/menu_bar/items::CATEGORIES` — every
//! category's entries in order = 10 rows after MENU-008), so the dialog, the
//! dropdown, the bar and the bare-key docs all read the one CATEGORIES
//! table and can never drift (MENU-005 R1; BOARD-023 R11 carried
//! over). The deleted BOARD-023 `board_shortcuts::SHORTCUTS` shim is
//! no longer a row source anywhere.
//!
//! Stability (R2): the compositor id stays `"board-actions-dialog"`
//! and the Action variant stays `OpenBoardKeybindingDialog` this
//! release (both are bus/stable tokens — the rename is next release);
//! the title reads `'Menu bar'`. Priority::Foreground, Accent::Yellow,
//! the footer and the true-modal key blocking (BOARD-023 R7/R8) are
//! unchanged.
//!
//! Snapshot semantics (R3, re-purposed by BUG-203): the dialog is
//! payload-free — Enter on the `.` New Agent row resolves through
//! `MenuAction::to_action` (BUG-199 / BUG-203: the row ALWAYS mounts
//! the CreateSessionDialog via the shared RPC-060 helper; it never
//! re-enters / resumes a session — that is the Shift+Right CYCLE
//! gesture's `OpenAgentView` job). Enter on ANY registry row emits
//! that entry's Action (resolved via `MenuAction::to_action`) and
//! closes the dialog — so the registry's Help/Exit rows execute
//! OpenBoardHelp / OpenBoardExitConfirmation exactly as before.
//!
//! Navigation: `↑`/`↓` move the cursor with wrap-around (BOARD-023
//! R4); the mouse wheel drives the same movement (MuxConfigDialog R4
//! parity).
//!
//! `Esc` → close WITHOUT executing the highlighted row (BOARD-023 R8).
//!
//! True-modal key blocking (BOARD-023 R7, BUG-161 parity with
//! WorkUnitSearchDialog): every key this dialog does not explicitly
//! handle — including `'u'`/`'U'` (R2 idempotent no-op), `j/k/h/l`,
//! `[`/`]`, Shift/Ctrl chords, and pastes — is CONSUMED as a no-op so
//! the view behind it stays frozen.
//!
//! Renders via the shared `dialog_theme::render_dialog` with
//! `Accent::Yellow` (informational popups: attachment picker, thinking
//! level, exit confirmation). The title reads "Menu bar" (R2).
//!
//! Card: MENU-005.

use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use super::dialog_theme::{render_dialog, Accent, FspecDialog};
use super::dialog_theme_rows::label_description_row;
use super::menu_bar::items::{MenuAction, CATEGORIES, HELP, KANBAN, SETTINGS, TOOLS};
use super::{Action, Callback, Component, EventResult, Priority};

/// Stable compositor id (BOARD-023 — kept unchanged this release,
/// MENU-005 R2). Used by `Compositor::remove` AND `Compositor::contains`
/// to address the dialog idempotently.
pub const MENU_BAR_HELP_DIALOG_ID: &str = "board-actions-dialog";

const ACCENT: Accent = Accent::Yellow;
const TITLE: &str = "Menu bar";
const FOOTER: &str = "↑↓ Navigate │ Enter Execute │ Esc Close";
const MIN_WIDTH: u16 = 48;

/// R1: the body rows are the FULL MenuCategories registry — every
/// category's entries in order (MENU-008: Kanban's 4 + Tools' 2 +
/// Settings' 2 + Help's 2 = 11). The `menu001_menu_bar` registry tests
/// pin CATEGORIES to exactly these four categories (in this order) with
/// these entry counts, so a registry change fails there first.
const ROW_COUNT: usize = KANBAN.len() + TOOLS.len() + SETTINGS.len() + HELP.len();

/// The per-row actions in registry order (the Enter resolver, R3).
fn registry_actions() -> Vec<MenuAction> {
    CATEGORIES
        .iter()
        .flat_map(|c| c.entries.iter().map(|e| e.action))
        .collect()
}

/// Priority::Foreground modal dialog listing the full menu registry
/// with a one-line description per row.
pub struct MenuBarHelpDialog {
    id: String,
    /// Cursor: index into the registry rows (0 = `. New Agent`).
    cursor: usize,
    action_tx: Option<UnboundedSender<Action>>,
    pending_action: Option<Action>,
}

impl MenuBarHelpDialog {
    /// Construct a fresh dialog. BUG-203: the `.` New Agent row is
    /// payload-free — the registry mapping ALWAYS mounts the
    /// CreateSessionDialog (the shared RPC-060 helper), so no session
    /// target is snapshotted at open time.
    pub fn new() -> Self {
        Self {
            id: MENU_BAR_HELP_DIALOG_ID.to_string(),
            cursor: 0,
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

    /// R3: resolve the Enter action for the highlighted row — the
    /// registry entry's `MenuAction` (payload-free: BUG-203 removed
    /// the NewAgent session-target substitution — the row ALWAYS
    /// mounts the CreateSessionDialog, it never re-enters a session).
    fn enter_action(&self) -> Option<Action> {
        Some(registry_actions().get(self.cursor)?.to_action())
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

impl Default for MenuBarHelpDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for MenuBarHelpDialog {
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
        let rows: Vec<crate::components::dialog_theme::DialogRow> = CATEGORIES
            .iter()
            .flat_map(|c| c.entries.iter())
            .enumerate()
            .map(|(i, entry)| {
                label_description_row(
                    &format!("{} {}", entry.key, entry.label),
                    entry.description,
                    i == self.cursor,
                )
            })
            .collect();
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
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn key(code: KeyCode) -> Event {
        Event::Key(crossterm::event::KeyEvent::new(
            code,
            crossterm::event::KeyModifiers::NONE,
        ))
    }

    #[test]
    fn new_defaults_cursor_to_first_registry_row() {
        let d = MenuBarHelpDialog::new();
        // R2: the stable BOARD-023 compositor id is kept unchanged.
        assert_eq!(d.id(), MENU_BAR_HELP_DIALOG_ID);
        assert_eq!(d.id(), "board-actions-dialog");
        assert_eq!(d.priority(), Priority::Foreground);
        assert_eq!(d.cursor(), 0);
        // R1: the FULL registry body — MENU-008: 4 Kanban + 2 Tools +
        // 2 Settings + 2 Help = 10 rows.
        assert_eq!(ROW_COUNT, 10);
    }

    #[test]
    fn up_from_first_row_wraps_to_last_and_down_wraps_back() {
        let mut d = MenuBarHelpDialog::new();
        let _ = d.handle_event(&key(KeyCode::Up));
        assert_eq!(d.cursor(), 9);
        let _ = d.handle_event(&key(KeyCode::Down));
        assert_eq!(d.cursor(), 0);
    }

    #[test]
    fn enter_on_row_zero_emits_open_create_session_dialog() {
        // BUG-203: the `.` New Agent row is payload-free — it ALWAYS
        // mounts the CreateSessionDialog (BUG-199 agent-bar parity),
        // it never re-enters / resumes a session.
        let mut d = MenuBarHelpDialog::new();
        let _ = d.handle_event(&key(KeyCode::Enter));
        assert!(
            matches!(
                d.take_pending_action(),
                Some(Action::OpenCreateSessionDialog { preselect: None })
            ),
            "row 0 (. New Agent) must emit OpenCreateSessionDialog {{ preselect: None }}"
        );
    }

    #[test]
    fn enter_on_checkpoints_row_emits_open_checkpoints_view() {
        let mut d = MenuBarHelpDialog::new();
        for _ in 0..5 {
            let _ = d.handle_event(&key(KeyCode::Down)); // cursor 0 → 5 (C Checkpoints, Tools)
        }
        let _ = d.handle_event(&key(KeyCode::Enter));
        assert!(matches!(
            d.take_pending_action(),
            Some(Action::OpenCheckpointsView)
        ));
    }

    #[test]
    fn enter_on_help_and_exit_rows_emit_the_new_variants() {
        let mut d = MenuBarHelpDialog::new();
        for _ in 0..8 {
            let _ = d.handle_event(&key(KeyCode::Down)); // cursor → 8 (? Help)
        }
        let _ = d.handle_event(&key(KeyCode::Enter));
        assert!(matches!(
            d.take_pending_action(),
            Some(Action::OpenBoardHelp)
        ));
        let mut d2 = MenuBarHelpDialog::new();
        for _ in 0..9 {
            let _ = d2.handle_event(&key(KeyCode::Down)); // cursor → 9 (Esc Exit)
        }
        let _ = d2.handle_event(&key(KeyCode::Enter));
        assert!(matches!(
            d2.take_pending_action(),
            Some(Action::OpenBoardExitConfirmation)
        ));
    }

    #[test]
    fn u_and_unhandled_keys_are_consumed_no_op() {
        let mut d = MenuBarHelpDialog::new();
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
        let mut d = MenuBarHelpDialog::new();
        assert!(d.handle_event(&key(KeyCode::Esc)).is_consumed());
        assert!(d.take_pending_action().is_none());
    }

    #[test]
    fn menu_bar_help_dialog_rendering_is_byte_equal_across_runs_insta_snapshot() {
        let mut dialog = MenuBarHelpDialog::new();
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
        insta::assert_yaml_snapshot!("menu_bar_help_dialog__centered_popup_80x24", rows);
    }
}
