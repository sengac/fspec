//! CreateSessionDialog — Priority::Foreground modal for picking a new
//! session creation mode (Yes / Yes - Isolated / Cancel).
//!
//! Feature: spec/features/rpc060-isolated-session-dialog.feature
//! Card: RPC-060 (parent RPC-030, phase 7.7).
//!
//! Mirrors `src/components/CreateSessionDialog.tsx` (TUI-090) — three
//! flat options with cyclic Left/Right navigation, cyan accent, and a
//! work-unit-aware title.
//!
//! TUI-112: the button row is ALSO left-click activatable — the shared
//! paint/hit-test builder lives in `super::three_button_dialog`.

use crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use codelet_rpc_types::WorkUnitContext;

use super::dialog_button_hits::LastLayout;
use super::dialog_theme::Accent;
use super::three_button_dialog::render_three_button_dialog;
use super::{Action, Callback, Component, EventResult, Priority};

/// Canonical id used by `Compositor::remove`.
pub const CREATE_SESSION_DIALOG_ID: &str = "create-session-dialog";

/// Single source of truth for the dialog's accent color. Used by both
/// `render()` and the public `accent()` accessor so the visual contract
/// asserted by tests and the contract painted into the buffer cannot
/// drift apart.
const ACCENT: Accent = Accent::Cyan;

/// Three flat options surfaced by the dialog. Discriminant order
/// matches `OPTIONS` in `src/components/CreateSessionDialog.tsx` so
/// Left/Right cycling lands on the same option across both frontends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateSessionOption {
    /// "Yes" — create a normal (non-isolated) session.
    Yes,
    /// "Yes - Isolated" — create a worktree-backed isolated session.
    Isolated,
    /// "Cancel" — close the dialog without creating a session.
    Cancel,
}

const OPTIONS: [CreateSessionOption; 3] = [
    CreateSessionOption::Yes,
    CreateSessionOption::Isolated,
    CreateSessionOption::Cancel,
];

const FOOTER: &str = "← → Select | Enter Confirm | Esc Cancel";

/// Minimum body content width. Matches the visual breadth of the TS Ink
/// CreateSessionDialog rendering at typical terminal widths.
const MIN_WIDTH: u16 = 50;

fn option_label(opt: CreateSessionOption) -> &'static str {
    match opt {
        CreateSessionOption::Yes => "Yes",
        CreateSessionOption::Isolated => "Yes - Isolated",
        CreateSessionOption::Cancel => "Cancel",
    }
}

/// Priority::Foreground modal dialog for picking a session-creation mode.
pub struct CreateSessionDialog {
    id: String,
    selected: CreateSessionOption,
    work_unit: Option<WorkUnitContext>,
    /// Work-unit-aware title ("Work on <id>?" / "Start New Agent?"),
    /// computed once at construction (it never changes afterwards).
    title: String,
    action_tx: Option<UnboundedSender<Action>>,
    pending_action: Option<Action>,
    /// TUI-112: last-rendered button-row geometry for left-click hit-testing.
    last_layout: LastLayout,
}

impl CreateSessionDialog {
    /// Construct a fresh dialog. `preselect=None` defaults to
    /// `CreateSessionOption::Yes`. `work_unit=Some(_)` switches the
    /// title to the context-aware "Work on <id>?" string.
    pub fn new(preselect: Option<CreateSessionOption>, work_unit: Option<WorkUnitContext>) -> Self {
        let title = match work_unit.as_ref() {
            Some(ctx) => format!("Work on {}?", ctx.id),
            None => "Start New Agent?".to_string(),
        };
        Self {
            id: CREATE_SESSION_DIALOG_ID.to_string(),
            selected: preselect.unwrap_or(CreateSessionOption::Yes),
            work_unit,
            title,
            action_tx: None,
            pending_action: None,
            last_layout: LastLayout::new(),
        }
    }

    /// Builder-style action_tx attach for the App's UnboundedSender.
    pub fn with_action_tx(mut self, tx: UnboundedSender<Action>) -> Self {
        self.action_tx = Some(tx);
        self
    }

    /// Test accessor — the currently highlighted option.
    pub fn selected_option(&self) -> CreateSessionOption {
        self.selected
    }

    /// Test accessor — the dialog title that will be rendered. Format:
    /// `"Work on <id>?"` when bound to a work unit, otherwise
    /// `"Start New Agent?"`.
    pub fn title(&self) -> String {
        self.title.clone()
    }

    /// Test accessor — the dialog's accent color. Returns the same
    /// `Accent` value that `render()` paints into the buffer (cyan,
    /// matching `src/components/CreateSessionDialog.tsx`
    /// `borderColor='cyan'`).
    pub fn accent(&self) -> Accent {
        ACCENT
    }

    /// Test-only: drain the most recent pending Action.
    pub fn take_pending_action(&mut self) -> Option<Action> {
        self.pending_action.take()
    }

    /// TUI-112: test accessor — the last-rendered button-row geometry
    /// (frame rect is `None` before the first render, R4).
    pub fn last_layout(&self) -> &LastLayout {
        &self.last_layout
    }

    /// TUI-112: commit a specific option — the shared path for the Enter
    /// key and a left-click on a button. Emits the option's action and
    /// returns the consumed-and-remove result (R1: one click == Enter on
    /// that button).
    fn commit(&mut self, opt: CreateSessionOption) -> EventResult {
        let action = match opt {
            CreateSessionOption::Yes => Action::CreateSessionSubmitted { isolated: false },
            CreateSessionOption::Isolated => Action::CreateSessionSubmitted { isolated: true },
            CreateSessionOption::Cancel => Action::CreateSessionCancelled,
        };
        self.emit_action(action);
        EventResult::Consumed(Some(self.remove_callback()))
    }

    /// TUI-112: route a left-button press. A click on a button commits
    /// that button (R1); a click on the row but off every button, or
    /// outside the dialog frame, is Ignored (R4/R5). Wheel behavior is
    /// handled separately (unchanged).
    fn handle_click(&mut self, col: u16, row: u16) -> EventResult {
        if !self.last_layout.contains(col, row) {
            return EventResult::ignored();
        }
        match self.last_layout.hit(col, row) {
            Some(0) => self.commit(CreateSessionOption::Yes),
            Some(1) => self.commit(CreateSessionOption::Isolated),
            Some(2) => self.commit(CreateSessionOption::Cancel),
            _ => EventResult::ignored(),
        }
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

impl Component for CreateSessionDialog {
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
                    self.emit_action(Action::CreateSessionCancelled);
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
        EventResult::ignored()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let description_text = if self.work_unit.is_some() {
            "Start an AI session for this task"
        } else {
            "Begin a fresh AI conversation, not linked to any task."
        };
        let labels: Vec<&str> = OPTIONS.iter().map(|o| option_label(*o)).collect();
        // TUI-112: paint + capture the button-row geometry in one call
        // (shared builder = single source of truth for pixels + hit test).
        self.last_layout = render_three_button_dialog(
            area,
            buf,
            ACCENT,
            &self.title,
            description_text,
            FOOTER,
            MIN_WIDTH,
            self.selected_index(),
            &labels,
        );
    }
}
