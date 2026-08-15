//! RPC-020 — File search popup widget.
//!
//! Feature: spec/features/rpc020-slash-and-file-popups.feature
//! Feature: spec/features/rpc027-slash-file-popups.feature
//!
//! Centred floating overlay rendered above AgentView's MultiLineInput
//! when the user types `@` followed by zero-or-more non-space chars.
//! `filter` tracks the text after the `@`; `anchor_offset` records the
//! byte offset of the `@` in the joined buffer so the eventual
//! select+splice can locate and replace the correct `@<filter>`
//! substring.
//!
//! Search results are populated asynchronously: AgentView emits
//! `Action::SearchFiles(prefix)` after each filter change; the App
//! dispatch fires a tokio task calling `backend.search_files`, then
//! emits `Action::FileSearchResults(matches)` which AgentView folds
//! back into this popup via `set_matches`.
//!
//! RPC-027: now renders via the shared dialog_theme renderer so the
//! cyan border, bold "File Search" inner title, two-character row
//! marker, inverse cyan/black selection highlight, and dim centered
//! footer match the TypeScript Ink reference exactly.

use std::cell::Cell;

use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::file_search_popup_rows::build_rows as build_dialog_rows;
use crate::components::dialog_theme::{
    dialog_rect, render_dialog, Accent, DialogRow, FspecDialog,
};
use crate::components::scroll_viewport::{
    ensure_visible, wrap_index, WheelDirection, WheelVelocity,
};
use crate::mouse::rect_contains;
use crate::mouse::scrollbar_drag::{ScrollbarDrag, ScrollbarGeometry};

/// Outcome of routing a single key event through the file search popup.
#[derive(Debug, Clone)]
pub enum FilePopupOutcome {
    /// User picked a path with Enter — splice it into the input,
    /// followed by a single trailing space.
    SelectedEnter(String),
    /// User picked a path with Tab — splice without a trailing space.
    SelectedTab(String),
    Dismiss,
    Continued,
    Ignored,
}

pub struct FileSearchPopup {
    filter: String,
    /// Byte offset of the `@` in the joined input buffer at the moment
    /// the popup opened. Used by AgentView's splice math.
    anchor_offset: usize,
    matches: Vec<String>,
    selected_index: usize,
    scroll_offset: usize,
    last_visible_rows: Cell<usize>,
    wheel: WheelVelocity,
    /// TUI-103: scrollbar click-and-drag state machine.
    scrollbar_drag: ScrollbarDrag,
    /// TUI-103: cached scrollbar gutter rect from last render for hit-testing.
    last_scrollbar_rect: Option<Rect>,
    /// TUI-103: cached body origin rect (dialog body content area) for
    /// converting absolute mouse rows to local scrollbar rows.
    last_body_origin: Option<Rect>,
}

impl FileSearchPopup {
    pub fn new(anchor_offset: usize, filter: &str) -> Self {
        Self {
            filter: filter.to_string(),
            anchor_offset,
            matches: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            last_visible_rows: Cell::new(10),
            wheel: WheelVelocity::new(),
            scrollbar_drag: ScrollbarDrag::new(),
            last_scrollbar_rect: None,
            last_body_origin: None,
        }
    }

    pub fn filter(&self) -> &str {
        &self.filter
    }

    pub fn anchor_offset(&self) -> usize {
        self.anchor_offset
    }

    pub fn matches(&self) -> &[String] {
        &self.matches
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }
    pub fn selected(&self) -> Option<&str> {
        self.matches.get(self.selected_index).map(String::as_str)
    }
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }
    /// TUI-103: cached scrollbar gutter rect from last render.
    pub fn last_scrollbar_rect(&self) -> Option<Rect> {
        self.last_scrollbar_rect
    }
    pub fn visible_rows(&self) -> usize {
        self.last_visible_rows.get().max(1)
    }
    pub fn shows_up_indicator(&self) -> bool {
        self.scroll_offset > 0
    }
    pub fn shows_down_indicator(&self) -> bool {
        self.scroll_offset + self.visible_rows() < self.matches.len()
    }

    /// Update the filter text. AgentView is expected to also emit
    /// `Action::SearchFiles(filter)` so a fresh result set arrives via
    /// `set_matches`.
    pub fn set_filter(&mut self, new_filter: &str) {
        if self.filter != new_filter {
            self.filter = new_filter.to_string();
            self.selected_index = 0;
            self.scroll_offset = 0;
            // TUI-103: reset scrollbar drag state when content changes
            self.scrollbar_drag.reset();
        }
    }

    /// Replace the result list. Selection is clamped to the new length.
    pub fn set_matches(&mut self, matches: Vec<String>) {
        // TUI-103: reset scrollbar drag state when content changes
        self.scrollbar_drag.reset();
        self.matches = matches;
        if self.matches.is_empty() {
            self.selected_index = 0;
            self.scroll_offset = 0;
        } else {
            if self.selected_index >= self.matches.len() {
                self.selected_index = self.matches.len() - 1;
            }
            let vr = self.visible_rows();
            let total = self.matches.len();
            ensure_visible(&mut self.scroll_offset, self.selected_index, vr, total);
        }
    }

    fn move_by(&mut self, delta: i32) {
        if self.matches.is_empty() {
            return;
        }
        self.selected_index = wrap_index(self.selected_index, delta, self.matches.len());
        let (vr, total) = (self.visible_rows(), self.matches.len());
        ensure_visible(&mut self.scroll_offset, self.selected_index, vr, total);
    }

    fn go_end(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        self.selected_index = self.matches.len() - 1;
        let (vr, total) = (self.visible_rows(), self.matches.len());
        ensure_visible(&mut self.scroll_offset, self.selected_index, vr, total);
    }

    /// Route a mouse event hit-tested against the popup's last-rendered
    /// rect. Outside the rect → `Ignored` so the caller can bubble.
    ///
    /// TUI-103: left-button press/drag/release on the scrollbar gutter
    /// column are routed through `ScrollbarDrag` before wheel events.
    pub fn handle_mouse(&mut self, ev: MouseEvent, popup_rect: Rect) -> FilePopupOutcome {
        let inside = ev.column >= popup_rect.x
            && ev.column < popup_rect.x + popup_rect.width
            && ev.row >= popup_rect.y
            && ev.row < popup_rect.y + popup_rect.height;
        if !inside {
            return FilePopupOutcome::Ignored;
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
                    let visible = self.visible_rows();
                    if total > visible {
                        // TUI-103: convert absolute screen row to body-local row
                        #[allow(clippy::expect_used)]
                        let body = self.last_body_origin.expect("body origin must be set when scrollbar rect is set");
                        let local_row = ev.row.saturating_sub(body.y);
                        let local_ev = MouseEvent {
                            row: local_row,
                            ..ev
                        };
                        let geom = ScrollbarGeometry {
                            area_height: body.height as usize,
                            total_items: total,
                            visible_items: visible,
                            current_offset: self.scroll_offset,
                        };
                        if let Some(offset) = self.scrollbar_drag.on_mouse(local_ev, geom) {
                            self.scroll_offset = offset;
                            // Adjust selection to stay visible
                            if self.selected_index >= total {
                                self.selected_index = total - 1;
                            }
                        }
                        return FilePopupOutcome::Continued;
                    }
                }
            }
            // Click outside scrollbar: reset drag state on Up
            if matches!(ev.kind, MouseEventKind::Up(MouseButton::Left)) {
                self.scrollbar_drag.reset();
            }
            return FilePopupOutcome::Ignored;
        }

        match ev.kind {
            MouseEventKind::ScrollUp => {
                let step = self.wheel.step(WheelDirection::Up);
                self.move_by(step);
                FilePopupOutcome::Continued
            }
            MouseEventKind::ScrollDown => {
                let step = self.wheel.step(WheelDirection::Down);
                self.move_by(step);
                FilePopupOutcome::Continued
            }
            _ => FilePopupOutcome::Ignored,
        }
    }

    #[doc(hidden)]
    pub fn set_visible_rows_for_test(&mut self, vr: usize) {
        self.last_visible_rows.set(vr);
    }

    #[doc(hidden)]
    pub fn set_selected_for_test(&mut self, idx: usize) {
        self.selected_index = idx.min(self.matches.len().saturating_sub(1));
        let (vr, total) = (self.visible_rows(), self.matches.len());
        ensure_visible(&mut self.scroll_offset, self.selected_index, vr, total);
    }

    pub fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> FilePopupOutcome {
        if mods.contains(KeyModifiers::SHIFT) || mods.contains(KeyModifiers::CONTROL) {
            return FilePopupOutcome::Ignored;
        }
        match code {
            KeyCode::Esc => FilePopupOutcome::Dismiss,
            KeyCode::Up => {
                self.move_by(-1);
                FilePopupOutcome::Continued
            }
            KeyCode::Down => {
                self.move_by(1);
                FilePopupOutcome::Continued
            }
            KeyCode::PageUp => {
                let step = -(self.visible_rows() as i32);
                self.move_by(step);
                FilePopupOutcome::Continued
            }
            KeyCode::PageDown => {
                let step = self.visible_rows() as i32;
                self.move_by(step);
                FilePopupOutcome::Continued
            }
            KeyCode::Home => {
                self.selected_index = 0;
                self.scroll_offset = 0;
                FilePopupOutcome::Continued
            }
            KeyCode::End => {
                self.go_end();
                FilePopupOutcome::Continued
            }
            KeyCode::Enter => match self.selected() {
                Some(path) => FilePopupOutcome::SelectedEnter(path.to_string()),
                None => FilePopupOutcome::Ignored,
            },
            KeyCode::Tab => match self.selected() {
                Some(path) => FilePopupOutcome::SelectedTab(path.to_string()),
                None => FilePopupOutcome::Ignored,
            },
            _ => FilePopupOutcome::Ignored,
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let vr = (area.height as usize).saturating_sub(8).clamp(1, 20);
        self.last_visible_rows.set(vr);

        let dialog = FspecDialog {
            accent: Accent::Cyan,
            title: "File Search",
            rows: self.build_rows(),
            footer: "↑↓ Navigate │ Tab/Enter Select │ Esc Close",
            min_width: 45,
        };

        // TUI-103: compute the dialog rect so we can derive the body area
        // for scrollbar geometry.
        let d_rect = dialog_rect(area, &dialog);
        let body_origin = Rect {
            x: d_rect.x + 2,
            y: d_rect.y + 4,
            width: d_rect.width.saturating_sub(4).max(1),
            height: d_rect.height.saturating_sub(4).max(1),
        };

        // TUI-103: pre-compute scrollbar rect for hit-testing — spans the
        // dialog body area (rightmost column of body content).
        let show_scrollbar = self.matches.len() > vr;
        let sb_rect = if show_scrollbar {
            let scrollbar_col = body_origin.x + body_origin.width - 1;
            Some(Rect {
                x: scrollbar_col,
                y: body_origin.y,
                width: 1,
                height: body_origin.height,
            })
        } else {
            None
        };

        render_dialog(area, buf, &dialog);

        self.last_scrollbar_rect = sb_rect;
        self.last_body_origin = Some(body_origin);
    }

    pub(super) fn build_rows(&self) -> Vec<DialogRow> {
        build_dialog_rows(
            &self.matches,
            &self.filter,
            self.selected_index,
            self.scroll_offset,
            self.visible_rows(),
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    // Legacy + RPC-028 tests moved to tests/rpc028_popup_scroll.rs so
    // this file stays under the 300-LoC source-shape budget. Only the
    // snapshot test stays inline so the insta snapshot path remains
    // co-located with the renderer.

    #[test]
    fn file_search_popup_rendering_is_byte_equal_across_runs_insta_snapshot() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let mut popup = FileSearchPopup::new(0, "rea");
        popup.set_matches(vec!["README.md".to_string(), "src/readme.rs".to_string()]);
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
        terminal
            .draw(|frame| {
                popup.render(frame.area(), frame.buffer_mut());
            })
            .expect("draw");
        let buf = terminal.backend().buffer().clone();
        let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
        for y in 0..buf.area.height {
            let mut row = String::with_capacity(buf.area.width as usize);
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            rows.push(row);
        }
        insta::assert_yaml_snapshot!("file_search_popup__centered_popup_80x24", rows);
    }
}
