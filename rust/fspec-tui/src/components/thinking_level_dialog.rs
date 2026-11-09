//! ThinkingLevelDialog — Priority::Foreground modal for picking the
//! per-session thinking/reasoning level.
//!
//! Feature: spec/features/rpc022-thinking-level-dialog.feature
//! Feature: spec/features/rpc027-help-disconnect-thinking-dialogs.feature
//! Feature: spec/features/thinking-dialog-offers-only-the-models-levels.feature
//!
//! RPC-027: renders via the shared dialog_theme renderer with the
//! yellow accent, with the 'D Set Default' keybinding.
//!
//! MODEL-011: the rows are the levels the session's current model offers
//! (the daemon's [`ThinkingOptions`]), in canonical order, then BUG-455's
//! Model default; nothing sits behind a More… row. A numeric row (a token
//! budget or a numeric effort range) opens a number field inside the
//! dialog. A model with no reasoning control says so and offers only
//! Model default.

use crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use codelet_rpc_types::{SessionId, ThinkingChoice, ThinkingLevel, ThinkingOptions};

use super::dialog_button_hits::{row_list_layout, LastLayout};
use super::dialog_theme::{render_dialog, Accent, FspecDialog};
use super::{Action, Callback, Component, EventResult, Priority};

mod actions;
mod paint;

/// Canonical id used by `Compositor::remove`.
pub const THINKING_LEVEL_DIALOG_ID: &str = "thinking-level-dialog";

const MODEL_DEFAULT_LABEL: &str = "Model default";
const MODEL_DEFAULT_DESCRIPTION: &str = "No thinking parameter; the model decides";
const FOOTER: &str = "↑↓ Navigate │ Enter Select │ D Set Default │ Esc Close";
const FIELD_FOOTER: &str = "Type a number │ Enter Set │ Esc Back";

/// MODEL-011: the in-dialog number field of a numeric row.
struct NumberField {
    /// Index of the numeric row in `choices`.
    row: usize,
    text: String,
    refusal: Option<String>,
}

/// Priority::Foreground modal dialog for picking a thinking level.
pub struct ThinkingLevelDialog {
    id: String,
    session_id: SessionId,
    /// `provider/model`, as refusals name it.
    model: String,
    /// The offered levels; Model default is the row after them.
    choices: Vec<ThinkingChoice>,
    /// Why the model has no reasoning control, when it has none.
    no_control: Option<String>,
    current: ThinkingLevel,
    selected_index: usize,
    /// TUI-094: row index of the persisted default level, if any.
    default_index: Option<usize>,
    field: Option<NumberField>,
    action_tx: Option<UnboundedSender<Action>>,
    pending_action: Option<Action>,
    /// TUI-112: last-rendered row geometry for left-click hit-testing.
    last_layout: LastLayout,
}

impl ThinkingLevelDialog {
    /// A dialog bound to `session_id` offering `options`, with the row of
    /// `current_level` pre-selected (the first row when the model does not
    /// offer it).
    pub fn new(
        session_id: SessionId,
        current_level: ThinkingLevel,
        options: ThinkingOptions,
    ) -> Self {
        let (model, choices, no_control) = match options {
            ThinkingOptions::NoReasoningControl { model, reason } => {
                (model, Vec::new(), Some(reason))
            }
            ThinkingOptions::Levels { model, choices, .. } => (model, choices, None),
        };
        let mut dialog = Self {
            id: THINKING_LEVEL_DIALOG_ID.to_string(),
            session_id,
            model,
            choices,
            no_control,
            current: current_level,
            selected_index: 0,
            default_index: None,
            field: None,
            action_tx: None,
            pending_action: None,
            last_layout: LastLayout::new(),
        };
        dialog.selected_index = dialog.current_index().unwrap_or(0);
        dialog
    }

    /// TUI-094: thread the persisted default level into the dialog. `None`
    /// (or a level the model does not offer) leaves no `(default)` marker.
    pub fn with_default_level(mut self, default: Option<ThinkingLevel>) -> Self {
        self.default_index = default.and_then(|level| self.row_of(&level));
        self
    }

    pub fn with_action_tx(mut self, action_tx: UnboundedSender<Action>) -> Self {
        self.action_tx = Some(action_tx);
        self
    }

    fn row_count(&self) -> usize {
        self.choices.len() + 1
    }

    /// The row holding `level`: Model default is the last row.
    fn row_of(&self, level: &ThinkingLevel) -> Option<usize> {
        if *level == ThinkingLevel::ModelDefault {
            return Some(self.choices.len());
        }
        self.choices.iter().position(|choice| choice.holds(level))
    }

    /// The dialog's row labels, in order.
    pub fn row_labels(&self) -> Vec<String> {
        self.choices
            .iter()
            .map(ThinkingChoice::label)
            .chain(std::iter::once(MODEL_DEFAULT_LABEL.to_string()))
            .collect()
    }

    /// The row of the session's current level; `None` when the model does
    /// not offer it.
    pub fn current_index(&self) -> Option<usize> {
        self.row_of(&self.current)
    }

    /// Test accessor — currently highlighted row index.
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// TUI-095: the row carrying the `(default)` marker.
    pub fn default_index(&self) -> Option<usize> {
        self.default_index
    }

    /// The level the highlighted row stands for; `None` on a numeric row.
    pub fn selected_level(&self) -> Option<ThinkingLevel> {
        match self.choices.get(self.selected_index) {
            Some(choice) => choice.level(),
            None => Some(ThinkingLevel::ModelDefault),
        }
    }

    /// The open number field's text and refusal; `None` when closed.
    pub fn number_field(&self) -> Option<(String, Option<String>)> {
        self.field
            .as_ref()
            .map(|field| (field.text.clone(), field.refusal.clone()))
    }

    /// Test-only: drain any pending action stashed by `handle_event`.
    pub fn take_pending_action(&mut self) -> Option<Action> {
        self.pending_action.take()
    }

    /// TUI-112: test accessor — the last-rendered row geometry.
    pub fn last_layout(&self) -> &LastLayout {
        &self.last_layout
    }

    fn move_up(&mut self) {
        let count = self.row_count();
        self.selected_index = (self.selected_index + count - 1) % count;
    }

    fn move_down(&mut self) {
        self.selected_index = (self.selected_index + 1) % self.row_count();
    }

    fn emit_action(&mut self, action: Action) {
        if let Some(tx) = self.action_tx.as_ref() {
            let _ = tx.send(action.clone());
        }
        self.pending_action = Some(action);
    }

    fn close(&self) -> EventResult {
        let id = self.id.clone();
        let callback: Callback = Box::new(move |compositor| {
            let _ = compositor.remove(&id);
        });
        EventResult::Consumed(Some(callback))
    }
}

impl Component for ThinkingLevelDialog {
    fn priority(&self) -> Priority {
        Priority::Foreground
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        if let Event::Key(key) = event {
            if self.field.is_some() {
                return self.field_key(key.code);
            }
            return match key.code {
                KeyCode::Esc => self.close(),
                KeyCode::Up => {
                    self.move_up();
                    EventResult::consumed()
                }
                KeyCode::Down => {
                    self.move_down();
                    EventResult::consumed()
                }
                KeyCode::Enter => self.activate(),
                KeyCode::Char('d') | KeyCode::Char('D') => self.set_default(),
                _ => EventResult::ignored(),
            };
        }
        // RPC-028: mouse-wheel moves the selection like the arrow keys;
        // TUI-112: a left-click on a row selects AND activates it.
        if let Event::Mouse(m) = event {
            if self.field.is_some() {
                return EventResult::ignored();
            }
            match m.kind {
                MouseEventKind::Down(MouseButton::Left) => return self.click(m.column, m.row),
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
        let (rows, query, clickable) = self.dialog_rows();
        let dialog = FspecDialog {
            accent: Accent::Yellow,
            title: "Thinking Level",
            rows,
            footer: if self.field.is_some() {
                FIELD_FOOTER
            } else {
                FOOTER
            },
            min_width: 50,
            query_row: query.as_deref(),
        };
        // TUI-112: cache the row geometry for left-click hit-testing,
        // derived from the SAME descriptor that is painted.
        self.last_layout = row_list_layout(area, &dialog, clickable);
        render_dialog(area, buf, &dialog);
    }
}
