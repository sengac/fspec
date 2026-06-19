//! ExitConfirmationDialog — Priority::Critical modal for the AgentView ESC
//! cascade level 7 (RPC-098).
//!
//! Feature: spec/features/agentview-esc-exit-confirmation-dialog.feature
//! Card: RPC-098 (parent: RPC-002 rust-frontend epic).
//!
//! Mirrors `src/components/ThreeButtonDialog.tsx` as used by
//! `src/tui/components/AgentView.tsx` lines 4391-4426 + 5502-5515 (TUI-045 /
//! TUI-046): three flat options [Detach, Close Session, Cancel] with cyclic
//! Left/Right navigation, yellow accent, Detach pre-selected. Enter commits
//! `Action::AgentExitChoice { choice }`. ESC commits Cancel.
//!
//! Description text is conditional on `is_busy`:
//! - `true`  → "The agent is currently running. Choose how to exit."
//! - `false` → "Choose how to exit the session."

use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;

use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog};
use super::{Action, Callback, Component, EventResult, Priority};

/// Canonical id used by `Compositor::remove`.
pub const EXIT_CONFIRMATION_DIALOG_ID: &str = "exit-confirmation-dialog";

/// Single source of truth for the dialog's accent colour. Matches the TS
/// `<ThreeButtonDialog borderColor="yellow" />` choice.
const ACCENT: Accent = Accent::Yellow;

/// Three flat options surfaced by the dialog. Discriminant order matches
/// `options=['Detach', 'Close Session', 'Cancel']` in
/// `src/tui/components/AgentView.tsx:5505` so cyclic Left/Right cycling
/// lands on the same option across both frontends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitChoice {
    /// "Detach" — leave the session running in background.
    Detach,
    /// "Close Session" — terminate the backend session.
    CloseSession,
    /// "Cancel" — close the dialog, stay on AgentView.
    Cancel,
}

const OPTIONS: [ExitChoice; 3] = [
    ExitChoice::Detach,
    ExitChoice::CloseSession,
    ExitChoice::Cancel,
];

const TITLE: &str = "Exit Session?";
const DESCRIPTION_BUSY: &str = "The agent is currently running. Choose how to exit.";
const DESCRIPTION_IDLE: &str = "Choose how to exit the session.";
const FOOTER: &str = "← → Navigate | Enter Select | Esc Cancel";

/// Minimum body content width. Mirrors the visual breadth of the TS Ink
/// ThreeButtonDialog at typical terminal widths.
const MIN_WIDTH: u16 = 54;

fn option_label(opt: ExitChoice) -> &'static str {
    match opt {
        ExitChoice::Detach => "Detach",
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
}

impl ExitConfirmationDialog {
    /// Construct a fresh dialog. `is_busy=true` switches the description
    /// to the active-stream variant. Default selection is `Detach`
    /// (`defaultSelectedIndex=0` per TS contract).
    pub fn new(is_busy: bool) -> Self {
        Self {
            id: EXIT_CONFIRMATION_DIALOG_ID.to_string(),
            is_busy,
            selected: ExitChoice::Detach,
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
                KeyCode::Enter => {
                    let choice = self.selected;
                    self.emit_action(Action::AgentExitChoice { choice });
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
        // EXACT TS Ink parity (src/components/ThreeButtonDialog.tsx):
        // selected button = bg=Blue/fg=White/bold on " <label> ";
        // unselected = fg=Gray; centred three-button row; ASCII pipe
        // footer; NO marker glyphs.
        let description_text = self.description_text();
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

        // Layout mirrors TS marginX={1}: 1 leading space, ` Detach `,
        // 2 spaces, ` Close Session `, 2 spaces, ` Cancel `, 1 trailing.
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

        // Centre the button row within the body content width — mirrors
        // dialog_theme::inner_content_width's max() computation.
        let raw_w: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        let body_w = [
            TITLE.chars().count(),
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
            title: TITLE,
            rows: vec![description_row, button_row],
            footer: FOOTER,
            min_width: MIN_WIDTH,
        };
        render_dialog(area, buf, &dialog);
    }
}
