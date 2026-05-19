//! RPC-026 — SearchHistoryView: full-screen history palette.
//!
//! Feature: spec/features/rpc026-resume-and-search-mode-views.feature
//!
//! Mode view rendered when the user types `/search` or presses Ctrl+R
//! — replaces the legacy popup-style search palette. Paints into the
//! ENTIRE area Rect, hiding AgentView's normal layout. Mirrors TS
//! AgentView.tsx search mode (lines 5002-5191).
//!
//! Behaviour:
//!   * Typing chars / Backspace updates the query and emits
//!     `FilterChanged(query)` so the App layer can spawn
//!     `backend.persistence_search_history(query)`.
//!   * ↑/↓ navigate with wrap-around AND scroll-window updates.
//!   * Enter emits `Selected(text)` with the highlighted match's text.
//!   * Esc emits `Dismiss`.
//!
//! Forbidden imports (per source-shape regression): this widget does
//! NOT depend on the legacy floating-popup machinery.

use codelet_rpc_types::HistoryMatch;
use crossterm::event::{KeyCode, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph, Widget};

use crate::components::scroll_viewport::{
    ensure_visible, wrap_index, WheelDirection, WheelVelocity,
};

const CHROME_ROWS: u16 = 3;

/// Outcome of routing a single key event through the search view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchHistoryViewOutcome {
    FilterChanged(String),
    Selected(String),
    Dismiss,
    Continued,
    Ignored,
}

/// Full-screen history search palette.
pub struct SearchHistoryView {
    query: String,
    matches: Vec<HistoryMatch>,
    selected_index: usize,
    scroll_offset: usize,
    wheel: WheelVelocity,
}

impl Default for SearchHistoryView {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchHistoryView {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            matches: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            wheel: WheelVelocity::new(),
        }
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    pub fn matches(&self) -> &[HistoryMatch] {
        &self.matches
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn selected(&self) -> Option<&HistoryMatch> {
        self.matches.get(self.selected_index)
    }

    /// Replace the filter text. Selection + scroll are reset.
    pub fn set_query(&mut self, new_query: &str) {
        self.query = new_query.to_string();
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    /// Replace the match list. Selection is clamped; scroll is reset
    /// to keep the top row visible after refresh.
    pub fn set_matches(&mut self, matches: Vec<HistoryMatch>) {
        self.matches = matches;
        if self.matches.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= self.matches.len() {
            self.selected_index = self.matches.len() - 1;
        }
        self.scroll_offset = 0;
    }

    fn adjust_scroll(&mut self, visible_rows: usize) {
        ensure_visible(
            &mut self.scroll_offset,
            self.selected_index,
            visible_rows,
            self.matches.len(),
        );
    }

    fn move_by(&mut self, delta: i32, visible_rows: usize) {
        if self.matches.is_empty() {
            return;
        }
        self.selected_index = wrap_index(self.selected_index, delta, self.matches.len());
        self.adjust_scroll(visible_rows);
    }

    /// Route a mouse event hit-tested against the view's `body_rect`.
    pub fn handle_mouse(
        &mut self,
        ev: MouseEvent,
        body_rect: Rect,
        visible_rows: usize,
    ) -> SearchHistoryViewOutcome {
        let inside = ev.column >= body_rect.x
            && ev.column < body_rect.x + body_rect.width
            && ev.row >= body_rect.y
            && ev.row < body_rect.y + body_rect.height;
        if !inside {
            return SearchHistoryViewOutcome::Ignored;
        }
        match ev.kind {
            MouseEventKind::ScrollUp => {
                let step = self.wheel.step(WheelDirection::Up);
                self.move_by(step, visible_rows);
                SearchHistoryViewOutcome::Continued
            }
            MouseEventKind::ScrollDown => {
                let step = self.wheel.step(WheelDirection::Down);
                self.move_by(step, visible_rows);
                SearchHistoryViewOutcome::Continued
            }
            _ => SearchHistoryViewOutcome::Ignored,
        }
    }

    /// Route a single key event through the view. `visible_rows` mirrors
    /// `ResumeSessionView::handle_key` semantics.
    pub fn handle_key(
        &mut self,
        code: KeyCode,
        mods: KeyModifiers,
        visible_rows: usize,
    ) -> SearchHistoryViewOutcome {
        if mods.contains(KeyModifiers::CONTROL) || mods.contains(KeyModifiers::ALT) {
            return SearchHistoryViewOutcome::Ignored;
        }
        match code {
            KeyCode::Esc => SearchHistoryViewOutcome::Dismiss,
            KeyCode::Up => {
                self.move_by(-1, visible_rows);
                SearchHistoryViewOutcome::Continued
            }
            KeyCode::Down => {
                self.move_by(1, visible_rows);
                SearchHistoryViewOutcome::Continued
            }
            KeyCode::PageUp => {
                self.move_by(-(visible_rows.max(1) as i32), visible_rows);
                SearchHistoryViewOutcome::Continued
            }
            KeyCode::PageDown => {
                self.move_by(visible_rows.max(1) as i32, visible_rows);
                SearchHistoryViewOutcome::Continued
            }
            KeyCode::Home => {
                self.selected_index = 0;
                self.scroll_offset = 0;
                SearchHistoryViewOutcome::Continued
            }
            KeyCode::End => {
                if !self.matches.is_empty() {
                    self.selected_index = self.matches.len() - 1;
                    self.adjust_scroll(visible_rows);
                }
                SearchHistoryViewOutcome::Continued
            }
            KeyCode::Enter => match self.selected() {
                Some(m) => SearchHistoryViewOutcome::Selected(m.text.clone()),
                None => SearchHistoryViewOutcome::Ignored,
            },
            KeyCode::Backspace => {
                if self.query.is_empty() {
                    SearchHistoryViewOutcome::Continued
                } else {
                    self.query.pop();
                    self.selected_index = 0;
                    self.scroll_offset = 0;
                    SearchHistoryViewOutcome::FilterChanged(self.query.clone())
                }
            }
            KeyCode::Char(c) => {
                self.query.push(c);
                self.selected_index = 0;
                self.scroll_offset = 0;
                SearchHistoryViewOutcome::FilterChanged(self.query.clone())
            }
            _ => SearchHistoryViewOutcome::Ignored,
        }
    }

    fn render_title(&self, area: Rect, buf: &mut Buffer) {
        let cursor_style = Style::default().add_modifier(Modifier::REVERSED);
        let spans = vec![
            Span::raw("(search): "),
            Span::raw(self.query.clone()),
            Span::styled(" ", cursor_style),
        ];
        Paragraph::new(Line::from(spans)).render(area, buf);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Enter Select | ↑↓ Navigate | Esc Cancel").render(area, buf);
    }

    fn render_body(&self, area: Rect, buf: &mut Buffer) {
        if self.matches.is_empty() {
            let placeholder = if self.query.is_empty() {
                "(type to search history)".to_string()
            } else {
                format!("(no history matches \"{}\")", self.query)
            };
            let mid_y = area.y.saturating_add(area.height / 2);
            let row = Rect { x: area.x, y: mid_y, width: area.width, height: 1 };
            Paragraph::new(placeholder)
                .alignment(Alignment::Center)
                .render(row, buf);
            return;
        }
        let visible_rows = area.height as usize;
        if visible_rows == 0 {
            return;
        }
        let end = (self.scroll_offset + visible_rows).min(self.matches.len());
        for (row_idx, m) in self.matches[self.scroll_offset..end].iter().enumerate() {
            let global_idx = self.scroll_offset + row_idx;
            let marker = if global_idx == self.selected_index { "▸" } else { " " };
            let label = format!(" {marker} {}", m.text);
            let style = if global_idx == self.selected_index {
                Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
            } else {
                Style::default()
            };
            let y = area.y + row_idx as u16;
            let row_area = Rect { x: area.x, y, width: area.width, height: 1 };
            Paragraph::new(Line::from(Span::styled(label, style))).render(row_area, buf);
        }
    }

    /// Paint the view into the FULL area Rect. First statement is
    /// `Clear.render(area, buf)` so the underlying AgentView pixels
    /// are fully overwritten.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);
        self.render_title(split[0], buf);
        self.render_body(split[2], buf);
        self.render_footer(split[3], buf);
    }

    /// Heuristic visible-row hint used by AgentView when computing
    /// `handle_key`'s `visible_rows` argument.
    pub fn visible_rows_for(area: Rect) -> usize {
        area.height.saturating_sub(CHROME_ROWS) as usize
    }
}
