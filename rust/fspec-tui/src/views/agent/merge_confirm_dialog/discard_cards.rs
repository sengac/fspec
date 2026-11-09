//! WT-018 — DiscardCardsDialog: before a worktree is discarded, the cards
//! its session created, moved or edited on the shared board.
//!
//! Feature: spec/features/discarding-a-worktree-names-the-cards-its-session-changed.feature
//!
//! Opened by the merge dialog's Discard when the session changed cards
//! (a session that changed none discards at once, as before). One row per
//! card: its id, title and current status, and what the session did to
//! it. Three buttons: "Discard and move back" (the worktree is removed,
//! then every moved card goes back to its status before the session),
//! "Discard and keep statuses" (the worktree is removed, no card changes)
//! and "Cancel" (nothing happens). Nothing on the board changes without
//! one of the two discard choices.

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;

use codelet_rpc_types::{SessionCardChange, SessionCardChangeKind, SessionId};

use crate::components::dialog_button_hits::{separated_button_layout, LastLayout};
use crate::components::dialog_theme::{
    render_dialog, Accent, DialogRow, FspecDialog, FOOTER_SEPARATOR,
};
use crate::components::{Action, Callback, Component, EventResult};

/// Compositor stable id for the discard confirmation.
pub const DISCARD_CARDS_DIALOG_ID: &str = "discard-cards-dialog";

const BUTTONS: [&str; 3] = [
    "Discard and move back",
    "Discard and keep statuses",
    "Cancel",
];

/// What one card's row says the session did to it.
#[must_use]
pub fn change_text(change: &SessionCardChangeKind) -> String {
    match change {
        SessionCardChangeKind::Created => "created and kept".to_string(),
        SessionCardChangeKind::Moved { before } => {
            format!("moved, before the session: {before}")
        }
        SessionCardChangeKind::ChangedSince => "changed by someone else since".to_string(),
        SessionCardChangeKind::Edited => "edited".to_string(),
    }
}

/// One card's row: id, title, current status and what the session did.
fn card_line(card: &SessionCardChange) -> String {
    format!(
        "{}  {}  {} · {}",
        card.work_unit_id,
        card.title,
        card.status,
        change_text(&card.change)
    )
}

/// The discard confirmation listing a session's card changes.
pub struct DiscardCardsDialog {
    session_id: SessionId,
    changes: Vec<SessionCardChange>,
    focused: usize,
    last_render_area: std::cell::Cell<Option<Rect>>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl DiscardCardsDialog {
    /// A dialog for `session_id` listing `changes`; focus starts on
    /// "Discard and move back".
    pub fn new(session_id: SessionId, changes: Vec<SessionCardChange>) -> Self {
        Self {
            session_id,
            changes,
            focused: 0,
            last_render_area: std::cell::Cell::new(None),
            action_tx: None,
        }
    }

    /// Wire the App's action channel so the choice reaches `App::dispatch`.
    pub fn with_action_tx(mut self, action_tx: UnboundedSender<Action>) -> Self {
        self.action_tx = Some(action_tx);
        self
    }

    /// Index of the focused button (0 move back, 1 keep, 2 cancel).
    pub fn focused_button(&self) -> usize {
        self.focused
    }

    fn action_for(&self, index: usize) -> Action {
        match index {
            0 | 1 => Action::DiscardSessionCards {
                session_id: self.session_id.clone(),
                move_back: index == 0,
            },
            _ => Action::CancelDiscardCards,
        }
    }

    fn emit(&mut self, index: usize) -> EventResult {
        if let Some(tx) = self.action_tx.as_ref() {
            let _ = tx.send(self.action_for(index));
        }
        EventResult::Consumed(Some(remove_callback()))
    }

    fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> EventResult {
        if mods.contains(KeyModifiers::CONTROL) || mods.contains(KeyModifiers::ALT) {
            return EventResult::ignored();
        }
        let count = BUTTONS.len();
        match code {
            KeyCode::Esc => self.emit(count - 1),
            KeyCode::Enter => self.emit(self.focused),
            KeyCode::Left | KeyCode::BackTab => {
                self.focused = (self.focused + count - 1) % count;
                EventResult::consumed()
            }
            KeyCode::Tab if mods.contains(KeyModifiers::SHIFT) => {
                self.focused = (self.focused + count - 1) % count;
                EventResult::consumed()
            }
            KeyCode::Right | KeyCode::Tab => {
                self.focused = (self.focused + 1) % count;
                EventResult::consumed()
            }
            _ => EventResult::ignored(),
        }
    }

    fn last_layout(&self) -> LastLayout {
        let Some(area) = self.last_render_area.get() else {
            return LastLayout::new();
        };
        let descriptor = self.build_descriptor();
        let button_row = descriptor.rows.len() - 1;
        separated_button_layout(area, &descriptor, button_row, &BUTTONS)
    }

    fn build_descriptor(&self) -> FspecDialog<'_> {
        let text_row = |text: String| DialogRow {
            spans: vec![Span::raw(text)],
            selectable: false,
            selected: false,
        };
        let mut rows = vec![text_row(self.heading()), text_row(String::new())];
        rows.extend(self.changes.iter().map(|card| text_row(card_line(card))));
        rows.push(text_row(String::new()));
        rows.push(self.button_row());
        FspecDialog {
            accent: Accent::Yellow,
            title: "Discard Worktree",
            rows,
            footer: "Tab / ←→: focus  Enter: confirm  Esc: cancel",
            min_width: 60,
            query_row: None,
        }
    }

    fn heading(&self) -> String {
        format!(
            "Session {} changed these cards on the board:",
            self.session_id.value
        )
    }

    fn button_row(&self) -> DialogRow {
        let accent = Accent::Yellow.color();
        let mut spans: Vec<Span<'static>> = Vec::new();
        for (index, label) in BUTTONS.iter().enumerate() {
            if index > 0 {
                spans.push(Span::raw(FOOTER_SEPARATOR.to_string()));
            }
            let style = if index == self.focused {
                Style::default()
                    .bg(accent)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            spans.push(Span::styled(format!(" {label} "), style));
        }
        DialogRow {
            spans,
            selectable: false,
            selected: false,
        }
    }
}

impl Component for DiscardCardsDialog {
    fn id(&self) -> &str {
        DISCARD_CARDS_DIALOG_ID
    }

    fn priority(&self) -> crate::components::Priority {
        crate::components::Priority::Foreground
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.handle_key(key.code, key.modifiers)
            }
            Event::Mouse(mouse)
                if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) =>
            {
                let layout = self.last_layout();
                match layout.hit(mouse.column, mouse.row) {
                    Some(index) if layout.contains(mouse.column, mouse.row) => self.emit(index),
                    _ => EventResult::ignored(),
                }
            }
            _ => EventResult::ignored(),
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.last_render_area.set(Some(area));
        render_dialog(area, buf, &self.build_descriptor());
    }

    fn copy_text(&self) -> Option<String> {
        let mut lines = vec![self.heading()];
        lines.extend(self.changes.iter().map(card_line));
        Some(lines.join("\n"))
    }
}

fn remove_callback() -> Callback {
    Box::new(|compositor| {
        let _ = compositor.remove(DISCARD_CARDS_DIALOG_ID);
    })
}
