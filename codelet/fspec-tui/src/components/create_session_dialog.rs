//! CreateSessionDialog — Priority::Foreground modal for picking a new
//! session creation mode (Yes / Yes - Isolated / Cancel).
//!
//! Feature: spec/features/rpc060-isolated-session-dialog.feature
//! Card: RPC-060 (parent RPC-030, phase 7.7).
//!
//! Mirrors `src/components/CreateSessionDialog.tsx` (TUI-090) — three
//! flat options with cyclic Left/Right navigation, cyan accent, and a
//! work-unit-aware title.

use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;

use codelet_rpc_types::WorkUnitContext;

use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog};
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
    action_tx: Option<UnboundedSender<Action>>,
    pending_action: Option<Action>,
}

impl CreateSessionDialog {
    /// Construct a fresh dialog. `preselect=None` defaults to
    /// `CreateSessionOption::Yes`. `work_unit=Some(_)` switches the
    /// title to the context-aware "Work on <id>?" string.
    pub fn new(preselect: Option<CreateSessionOption>, work_unit: Option<WorkUnitContext>) -> Self {
        Self {
            id: CREATE_SESSION_DIALOG_ID.to_string(),
            selected: preselect.unwrap_or(CreateSessionOption::Yes),
            work_unit,
            action_tx: None,
            pending_action: None,
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
        match self.work_unit.as_ref() {
            Some(ctx) => format!("Work on {}?", ctx.id),
            None => "Start New Agent?".to_string(),
        }
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

    fn move_left(&mut self) {
        let idx = OPTIONS
            .iter()
            .position(|o| *o == self.selected)
            .unwrap_or(0);
        let next = if idx == 0 { OPTIONS.len() - 1 } else { idx - 1 };
        self.selected = OPTIONS[next];
    }

    fn move_right(&mut self) {
        let idx = OPTIONS
            .iter()
            .position(|o| *o == self.selected)
            .unwrap_or(0);
        let next = (idx + 1) % OPTIONS.len();
        self.selected = OPTIONS[next];
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
                    let action = match self.selected {
                        CreateSessionOption::Yes => {
                            Action::CreateSessionSubmitted { isolated: false }
                        }
                        CreateSessionOption::Isolated => {
                            Action::CreateSessionSubmitted { isolated: true }
                        }
                        CreateSessionOption::Cancel => Action::CreateSessionCancelled,
                    };
                    self.emit_action(action);
                    return EventResult::Consumed(Some(self.remove_callback()));
                }
                _ => {}
            }
        }
        if let Event::Mouse(m) = event {
            match m.kind {
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
        // EXACT TS Ink parity (src/components/CreateSessionDialog.tsx):
        // selected button = bg=Blue/fg=White/bold on " <label> ";
        // unselected = fg=Gray; centered three-button row; ASCII pipe
        // footer; NO ▸/○ marker glyphs.
        let description_text = if self.work_unit.is_some() {
            "Start an AI session for this task"
        } else {
            "Begin a fresh AI conversation, not linked to any task."
        };
        let dim_style = Style::default()
            .add_modifier(Modifier::DIM)
            .bg(Color::Black);
        let description_row = DialogRow {
            spans: vec![Span::styled(description_text.to_string(), dim_style)],
            selectable: false,
            selected: false,
        };

        let selected_style = Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);
        let unselected_style = Style::default().fg(Color::Gray).bg(Color::Black);

        // Layout mirrors TS marginX={1}: 1 leading space, ` Yes `,
        // 2 spaces, ` Yes - Isolated `, 2 spaces, ` Cancel `, 1 trailing.
        let mut spans: Vec<Span<'static>> = Vec::new();
        spans.push(Span::styled(" ".to_string(), dim_style));
        for (i, opt) in OPTIONS.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("  ".to_string(), dim_style));
            }
            let label = format!(" {} ", option_label(*opt));
            let style = if *opt == self.selected {
                selected_style
            } else {
                unselected_style
            };
            spans.push(Span::styled(label, style));
        }
        spans.push(Span::styled(" ".to_string(), dim_style));

        // Center the button row within the body content width — mirrors
        // dialog_theme::inner_content_width's max() computation so the
        // leading pad equals (final_body_w - raw_row_w)/2.
        let title_text = self.title();
        let raw_w: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        let body_w = [
            title_text.chars().count(),
            description_text.chars().count(),
            raw_w,
            FOOTER.chars().count(),
            MIN_WIDTH as usize,
        ]
        .into_iter()
        .max()
        .unwrap_or(MIN_WIDTH as usize);
        if body_w > raw_w {
            let pad = (body_w - raw_w) / 2;
            if pad > 0 {
                spans.insert(
                    0,
                    Span::styled(" ".repeat(pad), Style::default().bg(Color::Black)),
                );
            }
        }

        let button_row = DialogRow {
            spans,
            selectable: false,
            selected: false,
        };

        let dialog = FspecDialog {
            accent: ACCENT,
            title: title_text.as_str(),
            rows: vec![description_row, button_row],
            footer: FOOTER,
            min_width: MIN_WIDTH,
        };
        render_dialog(area, buf, &dialog);
    }
}
