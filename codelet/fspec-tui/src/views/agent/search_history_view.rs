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
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::components::scroll_viewport::{
    ensure_visible, wrap_index, WheelDirection, WheelVelocity,
};
use crate::mouse::rect_contains;
use crate::mouse::scrollbar_drag::{ScrollbarDrag, ScrollbarGeometry};
use crate::views::full_screen_shell::render_full_screen_scaffold_with_title;

use super::search_history_view_render::{render_body, render_title};

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
    /// TUI-103: scrollbar click-and-drag state machine.
    scrollbar_drag: ScrollbarDrag,
    /// TUI-103: cached scrollbar gutter rect from last render for hit-testing.
    last_scrollbar_rect: Option<Rect>,
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
            scrollbar_drag: ScrollbarDrag::new(),
            last_scrollbar_rect: None,
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
        // TUI-103: reset scrollbar drag state when content changes
        self.scrollbar_drag.reset();
    }

    /// Replace the match list. Selection is clamped; scroll is reset
    /// to keep the top row visible after refresh.
    pub fn set_matches(&mut self, matches: Vec<HistoryMatch>) {
        // TUI-103: reset scrollbar drag state when content changes
        self.scrollbar_drag.reset();
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
    ///
    /// TUI-103: left-button press/drag/release on the scrollbar gutter
    /// column are routed through `ScrollbarDrag` before wheel events.
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

        // TUI-103: handle scrollbar click-and-drag for left-button events
        if matches!(
            ev.kind,
            MouseEventKind::Down(MouseButton::Left)
                | MouseEventKind::Drag(MouseButton::Left)
                | MouseEventKind::Up(MouseButton::Left)
        ) {
            if let Some(sb_rect) = self.last_scrollbar_rect {
                if rect_contains(sb_rect, ev.column, ev.row) {
                    let total = self.matches.len();
                    if total > visible_rows {
                        let geom = ScrollbarGeometry {
                            area_height: visible_rows,
                            total_items: total,
                            visible_items: visible_rows,
                            current_offset: self.scroll_offset,
                        };
                        if let Some(offset) = self.scrollbar_drag.on_mouse(ev, geom) {
                            self.scroll_offset = offset;
                            // Adjust selection to stay visible
                            if self.selected_index >= total {
                                self.selected_index = total - 1;
                            }
                        }
                        return SearchHistoryViewOutcome::Continued;
                    }
                }
            }
            // Click outside scrollbar: reset drag state on Up
            if matches!(ev.kind, MouseEventKind::Up(MouseButton::Left)) {
                self.scrollbar_drag.reset();
            }
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
            // RPC-064: vim-style j/k navigation. Lowercase `j`/`k` with
            // no modifiers move the selection ±1 (wrapping) without
            // appending to the query buffer. Uppercase `J`/`K` (which
            // carry KeyModifiers::SHIFT for the char) still flow into
            // the generic `KeyCode::Char(c)` branch so they're typed
            // into the filter — only the lowercase forms hijack the
            // selection cursor.
            KeyCode::Char('j') if mods.is_empty() => {
                self.move_by(1, visible_rows);
                SearchHistoryViewOutcome::Continued
            }
            KeyCode::Char('k') if mods.is_empty() => {
                self.move_by(-1, visible_rows);
                SearchHistoryViewOutcome::Continued
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

    /// Paint the view into the FULL area Rect. Delegates the shared
    /// scaffold (`Clear`, the 4-constraint title/separator/body/footer
    /// split, and the optional overlay) to the full-screen shell
    /// (RPC-339), supplying its editable-query title via a title closure
    /// so the `(search): <query>` line and inverse cursor are preserved.
    ///
    /// TUI-103: caches the scrollbar gutter rect for hit-testing.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // TUI-103: pre-compute scrollbar rect for hit-testing
        let visible_rows = Self::visible_rows_for(area);
        let show_scrollbar = self.matches.len() > visible_rows;
        let body_area = Rect {
            x: area.x,
            y: area.y.saturating_add(2), // title + separator
            width: area.width,
            height: area.height.saturating_sub(3), // title + separator + footer
        };
        let sb_rect = if show_scrollbar {
            let content_width = body_area.width.saturating_sub(1);
            Some(Rect {
                x: body_area.x + content_width,
                y: body_area.y,
                width: 1,
                height: body_area.height,
            })
        } else {
            None
        };

        render_full_screen_scaffold_with_title(
            area,
            buf,
            |title_area, buf| render_title(self, title_area, buf),
            "Enter Select | ↑↓ Navigate | Esc Cancel",
            |body_area, buf| render_body(self, body_area, buf),
            None,
        );

        self.last_scrollbar_rect = sb_rect;
    }

    /// Heuristic visible-row hint used by AgentView when computing
    /// `handle_key`'s `visible_rows` argument.
    pub fn visible_rows_for(area: Rect) -> usize {
        area.height.saturating_sub(CHROME_ROWS) as usize
    }
}
