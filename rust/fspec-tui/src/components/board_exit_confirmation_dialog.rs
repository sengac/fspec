//! BoardExitConfirmationDialog — Priority::Critical modal that overlays the
//! BoardView when the user presses ESC. Mirrors the TypeScript Ink
//! `<ConfirmationDialog message="Exit fspec?" />` from
//! `src/tui/components/BoardView.tsx` lines 641-654.
//!
//! Feature: spec/features/boardview-esc-key-exit-confirmation.feature
//! Card: RPC-102.
//!
//! Two flat options [Exit, Cancel] with cyclic Left/Right navigation,
//! yellow accent, "Exit" pre-selected. Enter on Exit commits
//! `Action::Quit`. ESC commits Cancel (closes dialog, stays on board).
//!
//! The TS counterpart uses `confirmMode="visual"` + `riskLevel="medium"` —
//! the visual mode renders a flat two-button layout, identical structure
//! to the AgentView's `ExitConfirmationDialog` but with two options
//! instead of three.

use crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use super::dialog_button_hits::LastLayout;
use super::dialog_theme::Accent;
use super::three_button_dialog::render_three_button_dialog;
use super::{Action, Callback, Component, EventResult, Priority};

/// Canonical id used by `Compositor::push` / `Compositor::remove` and the
/// `compositor.contains(...)` guard in `events.rs` that prevents
/// double-pushing the dialog on rapid ESC presses.
pub const BOARD_EXIT_CONFIRMATION_DIALOG_ID: &str = "board-exit-confirmation-dialog";

/// Accent matches the TS `ConfirmationDialog` riskLevel=medium → yellow
/// border.
const ACCENT: Accent = Accent::Yellow;

/// Two flat options surfaced by the dialog. Discriminant order matches
/// `[Exit, Cancel]` so cyclic Left/Right cycling lands on the same option
/// regardless of starting position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardExitChoice {
    /// "Exit" — quit the application.
    Exit,
    /// "Cancel" — close the dialog, stay on the BoardView.
    Cancel,
}

const OPTIONS: [BoardExitChoice; 2] = [BoardExitChoice::Exit, BoardExitChoice::Cancel];

const TITLE: &str = "Exit fspec?";
const DESCRIPTION: &str = "Are you sure you want to exit?";
const FOOTER: &str = "← → Navigate | Enter Select | Esc Cancel";

/// Minimum body content width — visual breadth at typical terminal widths.
const MIN_WIDTH: u16 = 54;

fn option_label(opt: BoardExitChoice) -> &'static str {
    match opt {
        BoardExitChoice::Exit => "Exit",
        BoardExitChoice::Cancel => "Cancel",
    }
}

/// Priority::Critical modal dialog for the BoardView ESC exit confirmation.
pub struct BoardExitConfirmationDialog {
    id: String,
    selected: BoardExitChoice,
    action_tx: Option<UnboundedSender<Action>>,
    pending_action: Option<Action>,
    /// TUI-112: last-rendered button-row geometry for left-click hit-testing.
    last_layout: LastLayout,
}

impl Default for BoardExitConfirmationDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl BoardExitConfirmationDialog {
    /// Construct a fresh dialog. Default selection is `Exit`
    /// (mirrors TS `confirmMode=visual` with the destructive option
    /// pre-selected — user must explicitly press Esc to cancel).
    pub fn new() -> Self {
        Self {
            id: BOARD_EXIT_CONFIRMATION_DIALOG_ID.to_string(),
            selected: BoardExitChoice::Exit,
            action_tx: None,
            pending_action: None,
            last_layout: LastLayout::new(),
        }
    }

    /// TUI-112: commit a specific choice — the shared path for the Enter
    /// key and a left-click on a button (R1). Exit emits `Action::Quit`;
    /// Cancel is a pure dismiss (no action).
    fn commit(&mut self, choice: BoardExitChoice) -> EventResult {
        if choice == BoardExitChoice::Exit {
            self.emit_action(Action::Quit);
        }
        EventResult::Consumed(Some(self.remove_callback()))
    }

    /// TUI-112: route a left-button press. A click on a button commits
    /// that button (R1); a click off every button or outside the frame is
    /// Ignored (R4/R5).
    fn handle_click(&mut self, col: u16, row: u16) -> EventResult {
        if !self.last_layout.contains(col, row) {
            return EventResult::ignored();
        }
        match self.last_layout.hit(col, row) {
            Some(0) => self.commit(BoardExitChoice::Exit),
            Some(1) => self.commit(BoardExitChoice::Cancel),
            _ => EventResult::ignored(),
        }
    }

    /// Builder — attach the App's UnboundedSender so commit actions can
    /// reach `App::dispatch`.
    pub fn with_action_tx(mut self, tx: UnboundedSender<Action>) -> Self {
        self.action_tx = Some(tx);
        self
    }

    /// Test accessor — currently focused option.
    pub fn selected_choice(&self) -> BoardExitChoice {
        self.selected
    }

    /// Test accessor — the dialog's accent colour.
    pub fn accent(&self) -> Accent {
        ACCENT
    }

    /// Test-only: drain the most recent pending Action stashed by
    /// `handle_event` when no `action_tx` was attached.
    pub fn take_pending_action(&mut self) -> Option<Action> {
        self.pending_action.take()
    }

    /// TUI-112: test accessor — the last-rendered button-row geometry
    /// (frame rect is `None` before the first render, R4).
    pub fn last_layout(&self) -> &LastLayout {
        &self.last_layout
    }

    fn move_left(&mut self) {
        let idx = self.selected_index();
        let next = if idx == 0 { OPTIONS.len() - 1 } else { idx - 1 };
        self.selected = OPTIONS[next];
    }

    fn move_right(&mut self) {
        let next = (self.selected_index() + 1) % OPTIONS.len();
        self.selected = OPTIONS[next];
    }

    fn selected_index(&self) -> usize {
        OPTIONS
            .iter()
            .position(|o| *o == self.selected)
            .unwrap_or(0)
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
}

impl Component for BoardExitConfirmationDialog {
    fn priority(&self) -> Priority {
        Priority::Critical
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Esc => {
                    // ESC == Cancel (parity with TS onCancel callback).
                    // Dialog removes itself; should_quit stays false.
                    return EventResult::Consumed(Some(self.remove_callback()));
                }
                KeyCode::Left => {
                    self.move_left();
                    return EventResult::consumed();
                }
                KeyCode::Right => {
                    self.move_right();
                    return EventResult::consumed();
                }
                KeyCode::Enter => {
                    return self.commit(self.selected);
                }
                _ => {}
            }
        }
        if let Event::Mouse(m) = event {
            match m.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    return self.handle_click(m.column, m.row);
                }
                MouseEventKind::ScrollLeft | MouseEventKind::ScrollUp => {
                    self.move_left();
                    return EventResult::consumed();
                }
                MouseEventKind::ScrollRight | MouseEventKind::ScrollDown => {
                    self.move_right();
                    return EventResult::consumed();
                }
                _ => {}
            }
        }
        // RPC-403 review: Critical modal — consume (swallow) pastes so
        // they can never leak into the board hidden behind this
        // dialog. No text field here, so nothing is inserted.
        if matches!(event, Event::Paste(_)) {
            return EventResult::consumed();
        }
        EventResult::ignored()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Mirror ExitConfirmationDialog's flat-button rendering style,
        // dropped to two options instead of three — the shared
        // three-button builder is the single source of truth for the
        // painted row and the TUI-112 click hit-test rects.
        let labels: Vec<&str> = OPTIONS.iter().map(|o| option_label(*o)).collect();
        self.last_layout = render_three_button_dialog(
            area,
            buf,
            ACCENT,
            TITLE,
            DESCRIPTION,
            FOOTER,
            MIN_WIDTH,
            self.selected_index(),
            &labels,
        );
    }
}
