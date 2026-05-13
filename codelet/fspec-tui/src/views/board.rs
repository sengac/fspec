//! BoardView — first landing view; minimal placeholder Kanban skeleton.
//!
//! Feature: spec/features/rpc012-board-agent-navigation.feature
//! Card: RPC-012 (replaces RPC-009 `WorkUnitsListView`).
//!
//! Renders the seven canonical columns as headers + a vertical stack of
//! `{id}` per column. The rich `UnifiedBoardLayout` port (per-column
//! viewport scroll, work-unit details strip, last-changed `⏩`, mouse,
//! `[/]` priority reorder UX, etc.) is a downstream slice.
//!
//! BoardView reads from a `&BoardStore` passed in via `render_with_store`
//! — it does NOT own any work-units state. Keyboard handling emits
//! `Action::EnterWorkUnit(id)` / `Action::OpenAgentView(target)` /
//! `Action::ReorderUp` / `Action::ReorderDown` onto the action bus that
//! `App::dispatch` consumes.

use std::sync::Arc;

use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use tokio::sync::mpsc::UnboundedSender;

use crate::components::{Action, EventResult};
use crate::store::{BoardStore, COLUMN_ORDER};
use crate::theme::Theme;

/// BoardView holds NO work-units state — only the action bus + theme.
pub struct BoardView {
    pub theme: Arc<Theme>,
    pub action_tx: Option<UnboundedSender<Action>>,
}

impl BoardView {
    pub fn new(theme: Arc<Theme>, action_tx: UnboundedSender<Action>) -> Self {
        Self {
            theme,
            action_tx: Some(action_tx),
        }
    }

    fn emit(&self, action: Action) {
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(action);
        }
    }

    /// Handle a keyboard event against the supplied store snapshot.
    /// The store is &-borrow only; mutation flows through App::dispatch
    /// in response to the emitted action.
    pub fn handle_event(&self, event: &Event, store: &BoardStore) -> EventResult {
        let Event::Key(key) = event else {
            return EventResult::ignored();
        };

        // Shift+Right → open AgentView (with or without an attached session).
        if key.code == KeyCode::Right && key.modifiers.contains(KeyModifiers::SHIFT) {
            let target = self.selected_session(store);
            self.emit(Action::OpenAgentView(target));
            return EventResult::consumed();
        }

        // Enter → hand off to AgentView for the focused work unit.
        if key.code == KeyCode::Enter {
            if let Some(unit) = store.selected_work_unit() {
                self.emit(Action::EnterWorkUnit(unit.id.clone()));
                return EventResult::consumed();
            }
            return EventResult::ignored();
        }

        // Arrow keys / j/k navigation — emit ColumnFocus/SelectionMove
        // actions for App::dispatch to apply against the mutable store.
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.emit(Action::FocusPrevColumn);
                return EventResult::consumed();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.emit(Action::FocusNextColumn);
                return EventResult::consumed();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.emit(Action::SelectNext);
                return EventResult::consumed();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.emit(Action::SelectPrev);
                return EventResult::consumed();
            }
            KeyCode::Char('[') => {
                self.emit(Action::ReorderUp);
                return EventResult::consumed();
            }
            KeyCode::Char(']') => {
                self.emit(Action::ReorderDown);
                return EventResult::consumed();
            }
            _ => {}
        }

        EventResult::ignored()
    }

    fn selected_session(&self, store: &BoardStore) -> Option<SessionId> {
        let unit = store.selected_work_unit()?;
        store.session_for(&unit.id).cloned()
    }

    /// Render the placeholder Kanban skeleton against the supplied store.
    pub fn render_with_store(
        &self,
        area: Rect,
        buf: &mut Buffer,
        store: &BoardStore,
    ) {
        let outer = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Board ({}) ", store.total_count()))
            .border_style(Style::default().fg(self.theme.border));
        let inner = outer.inner(area);
        outer.render(area, buf);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        // Equal-width columns. Any leftover width goes to the right edge
        // (intentional: this is a placeholder skeleton).
        let cols = COLUMN_ORDER.len() as u16;
        let col_width = inner.width.saturating_div(cols).max(1);
        let constraints: Vec<Constraint> =
            (0..cols).map(|_| Constraint::Length(col_width)).collect();
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(inner);

        for (idx, column_name) in COLUMN_ORDER.iter().enumerate() {
            let area = chunks[idx];
            self.render_column(*column_name, idx, area, buf, store);
        }
    }

    fn render_column(
        &self,
        column: &str,
        col_idx: usize,
        area: Rect,
        buf: &mut Buffer,
        store: &BoardStore,
    ) {
        let is_focused = store.focused_column_index() == col_idx;
        let header_style = if is_focused {
            Style::default().fg(Color::Cyan).bold()
        } else {
            Style::default().fg(self.theme.dim)
        };
        let header = Line::from(vec![Span::styled(
            column.to_uppercase(),
            header_style,
        )]);

        // Build cells: header + each work unit by id.
        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(header);
        lines.push(Line::from(Span::raw("")));

        let units = store.column_units(column);
        let selected_idx = store.selected_index_for(column);
        for (row, unit) in units.iter().enumerate() {
            let id_text = if let Some(points) = unit.estimate {
                format!("{} [{}]", unit.id, points)
            } else {
                unit.id.clone()
            };
            let style = if is_focused && row == selected_idx {
                Style::default().bg(Color::Green).fg(Color::Black).bold()
            } else if unit.work_type == "bug" {
                Style::default().fg(Color::Red)
            } else if unit.work_type == "task" {
                Style::default().fg(Color::Blue)
            } else {
                Style::default().fg(self.theme.fg)
            };
            lines.push(Line::from(Span::styled(id_text, style)));
        }

        Paragraph::new(lines).render(area, buf);
    }
}

