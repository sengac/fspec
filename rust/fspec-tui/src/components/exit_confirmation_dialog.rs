//! ExitConfirmationDialog — Priority::Critical modal for the AgentView ESC
//! cascade level 7 (RPC-098).
//!
//! Feature: spec/features/agentview-esc-exit-confirmation-dialog.feature
//! Card: RPC-098 (parent: RPC-002 rust-frontend epic).
//!
//! Mirrors `src/components/ThreeButtonDialog.tsx` as used by
//! `src/tui/components/AgentView.tsx` lines 4391-4426 + 5502-5515 (TUI-045 /
//! TUI-046): flat options with cyclic Left/Right navigation, yellow accent,
//! first option pre-selected. Enter commits `Action::AgentExitChoice
//! { choice }`. ESC commits Cancel.
//!
//! BUG-205: the original third option 'Detach' was removed — it only
//! dispatched `Action::BackToBoard`, which plain navigation (menu bar,
//! Shift+Left, mux board-pane focus) already does. The dialog now offers
//! exactly [Close Session, Cancel] with Close Session pre-selected,
//! matching the board 'Exit fspec?' convention.
//!
//! Description text is conditional on `is_busy`:
//! - `true`  → "The agent is currently running. Choose how to exit."
//! - `false` → "Choose how to exit the session."

use crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use super::dialog_button_hits::LastLayout;
use super::dialog_theme::Accent;
use super::three_button_dialog::render_three_button_dialog;
use super::{Action, Callback, Component, EventResult, Priority};

/// Canonical id used by `Compositor::remove`.
pub const EXIT_CONFIRMATION_DIALOG_ID: &str = "exit-confirmation-dialog";

/// Single source of truth for the dialog's accent colour. Matches the TS
/// `<ThreeButtonDialog borderColor="yellow" />` choice.
const ACCENT: Accent = Accent::Yellow;

/// Two flat options surfaced by the dialog. Discriminant order matches the
/// board `BoardExitConfirmationDialog` convention: the destructive option
/// first, pre-selected (BUG-205 removed the original 'Detach' option — it
/// only navigated back to the Board, which plain navigation already does).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitChoice {
    /// "Close Session" — terminate the backend session.
    CloseSession,
    /// "Cancel" — close the dialog, stay on AgentView.
    Cancel,
}

const OPTIONS: [ExitChoice; 2] = [ExitChoice::CloseSession, ExitChoice::Cancel];

const TITLE: &str = "Exit Session?";
const DESCRIPTION_BUSY: &str = "The agent is currently running. Choose how to exit.";
const DESCRIPTION_IDLE: &str = "Choose how to exit the session.";
const FOOTER: &str = "← → Navigate | Enter Select | Esc Cancel";

/// Minimum body content width. Mirrors the visual breadth of the TS Ink
/// ThreeButtonDialog at typical terminal widths.
const MIN_WIDTH: u16 = 54;

fn option_label(opt: ExitChoice) -> &'static str {
    match opt {
        ExitChoice::CloseSession => "Close Session",
        ExitChoice::Cancel => "Cancel",
    }
}

/// Priority::Critical modal dialog for the AgentView ESC exit confirmation.
pub struct ExitConfirmationDialog {
    id: String,
    is_busy: bool,
    selected: ExitChoice,
    action_tx: Option<UnboundedSender<Action>>,
    pending_action: Option<Action>,
    /// TUI-112: last-rendered button-row geometry for left-click hit-testing.
    last_layout: LastLayout,
}

impl ExitConfirmationDialog {
    /// Construct a fresh dialog. `is_busy=true` switches the description
    /// to the active-stream variant. Default selection is `CloseSession`
    /// (BUG-205: destructive option pre-selected, matching the board
    /// 'Exit fspec?' convention).
    pub fn new(is_busy: bool) -> Self {
        Self {
            id: EXIT_CONFIRMATION_DIALOG_ID.to_string(),
            is_busy,
            selected: ExitChoice::CloseSession,
            action_tx: None,
            pending_action: None,
            last_layout: LastLayout::new(),
        }
    }

    /// TUI-112: commit a specific choice — the shared path for the Enter
    /// key and a left-click on a button (R1).
    fn commit(&mut self, choice: ExitChoice) -> EventResult {
        self.emit_action(Action::AgentExitChoice { choice });
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
            Some(0) => self.commit(ExitChoice::CloseSession),
            Some(1) => self.commit(ExitChoice::Cancel),
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
    pub fn selected_choice(&self) -> ExitChoice {
        self.selected
    }

    /// Test accessor — whether this instance renders the busy description.
    pub fn is_busy(&self) -> bool {
        self.is_busy
    }

    /// Test accessor — the dialog's accent colour. Returns the same `Accent`
    /// value that `render()` paints (Yellow, mirroring the TS
    /// `borderColor='yellow'`).
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

    fn description_text(&self) -> &'static str {
        if self.is_busy {
            DESCRIPTION_BUSY
        } else {
            DESCRIPTION_IDLE
        }
    }
}

impl Component for ExitConfirmationDialog {
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
                    self.emit_action(Action::AgentExitChoice {
                        choice: ExitChoice::Cancel,
                    });
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
                KeyCode::Enter => return self.commit(self.selected),
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
        // they can never leak into the agent input hidden behind this
        // dialog. No text field here, so nothing is inserted.
        if matches!(event, Event::Paste(_)) {
            return EventResult::consumed();
        }
        EventResult::ignored()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // EXACT TS Ink parity (src/components/ThreeButtonDialog.tsx) —
        // the button row is painted by the shared three-button builder
        // (super::three_button_dialog) so the painted pixels and the
        // click hit-test rects come from one source of truth (TUI-112).
        let description_text = self.description_text();
        let labels: Vec<&str> = OPTIONS.iter().map(|o| option_label(*o)).collect();
        self.last_layout = render_three_button_dialog(
            area,
            buf,
            ACCENT,
            TITLE,
            description_text,
            FOOTER,
            MIN_WIDTH,
            self.selected_index(),
            &labels,
        );
    }
}
