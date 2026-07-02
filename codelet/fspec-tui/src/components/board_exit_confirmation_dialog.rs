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

use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;

use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog};
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
                    match self.selected {
                        BoardExitChoice::Exit => {
                            // Emit Quit; App::dispatch sets should_quit=true.
                            self.emit_action(Action::Quit);
                        }
                        BoardExitChoice::Cancel => {
                            // Pure dismiss — no action emitted.
                        }
                    }
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
        // dropped to two options instead of three.
        let dim_style = Style::default()
            .add_modifier(Modifier::DIM)
            .bg(Color::Black);
        let description_row = DialogRow {
            spans: vec![Span::styled(DESCRIPTION.to_string(), dim_style)],
            selectable: false,
            selected: false,
        };

        let selected_style = Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);
        let unselected_style = Style::default().fg(Color::Gray).bg(Color::Black);

        // Layout: leading space, ` Exit `, 2-space gap, ` Cancel `, trailing.
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

        // Centre the button row within the body content width.
        let raw_w: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        let body_w = [
            TITLE.chars().count(),
            DESCRIPTION.chars().count(),
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
            title: TITLE,
            rows: vec![description_row, button_row],
            footer: FOOTER,
            min_width: MIN_WIDTH,
        };
        render_dialog(area, buf, &dialog);
    }
}
