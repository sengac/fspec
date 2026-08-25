//! RPC-020 + RPC-027 — Slash command popup widget.
//!
//! Feature: spec/features/rpc020-slash-and-file-popups.feature
//! Feature: spec/features/rpc027-slash-file-popups.feature
//!
//! Centred floating overlay rendered above AgentView's MultiLineInput
//! when the user types a leading `/`. Filter text tracks the
//! characters after the `/`; ↑/↓ navigate (wrap-around); Enter
//! selects+executes; Tab fills the input without executing; Esc
//! dismisses. RPC-027 renders via the shared dialog_theme renderer.

use std::cell::Cell;

use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::slash_command_popup_rows::build_rows as build_dialog_rows;
use super::slash_commands::{filter_commands, SlashCommand, SlashCommandAction, SLASH_COMMANDS};
use crate::components::dialog_theme::{
    dialog_rect, render_dialog, Accent, DialogRow, FspecDialog,
};
use crate::components::scroll_viewport::{
    ensure_visible, wrap_index, WheelDirection, WheelVelocity,
};
use crate::mouse::rect_contains;
use crate::mouse::scrollbar_drag::{ScrollbarDrag, ScrollbarGeometry};

/// Outcome of routing a single key event through the slash popup.
#[derive(Debug, Clone)]
pub enum PopupOutcome {
    Selected(SlashCommandAction),
    Filled(String),
    Dismiss,
    Continued,
    Ignored,
}

/// Slash command palette state.
pub struct SlashCommandPopup {
    filter: String,
    matches: Vec<&'static SlashCommand>,
    selected_index: usize,
    scroll_offset: usize,
    /// Body rows the most recent `render` decided to paint. Updated on
    /// every render so subsequent key/mouse handlers can produce
    /// correctly-sized scroll steps (mirrors BoardView's
    /// `last_viewport_height` idiom in views/board.rs:58-95).
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

impl Default for SlashCommandPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl SlashCommandPopup {
    /// Construct a fresh popup with an empty filter and the first match
    /// pre-selected.
    pub fn new() -> Self {
        Self {
            filter: String::new(),
            matches: SLASH_COMMANDS.iter().collect(),
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

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    pub fn matches(&self) -> &[&'static SlashCommand] {
        &self.matches
    }

    pub fn selected(&self) -> Option<&'static SlashCommand> {
        self.matches.get(self.selected_index).copied()
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

    /// Body shows the `↑` glyph on its top row when there are rows above
    /// the current viewport.
    pub fn shows_up_indicator(&self) -> bool {
        self.scroll_offset > 0
    }

    /// Body shows the `↓` glyph on its bottom row when there are rows
    /// below the current viewport.
    pub fn shows_down_indicator(&self) -> bool {
        self.scroll_offset + self.visible_rows() < self.matches.len()
    }

    #[doc(hidden)]
    pub fn set_matches_for_test(&mut self, n: usize) {
        // Cycle the registry to fabricate a Vec of N commands.
        self.matches.clear();
        for i in 0..n {
            self.matches.push(&SLASH_COMMANDS[i % SLASH_COMMANDS.len()]);
        }
        self.selected_index = 0;
        self.scroll_offset = 0;
        // TUI-103: reset scrollbar drag state when content changes
        self.scrollbar_drag.reset();
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

    /// Update the filter (text after the leading `/`). Resets the
    /// selection to index 0 so the top match is highlighted.
    pub fn set_filter(&mut self, new_filter: &str) {
        self.filter = new_filter.to_string();
        self.matches = filter_commands(&self.filter);
        self.selected_index = 0;
        self.scroll_offset = 0;
        // TUI-103: reset scrollbar drag state when content changes
        self.scrollbar_drag.reset();
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
    /// rect. Events that fall outside `popup_rect` return
    /// [`PopupOutcome::Ignored`] so the caller can bubble them to the
    /// underlying view.
    ///
    /// TUI-103: left-button press/drag/release on the scrollbar gutter
    /// column are routed through `ScrollbarDrag` before wheel events.
    pub fn handle_mouse(&mut self, ev: MouseEvent, popup_rect: Rect) -> PopupOutcome {
        let inside = ev.column >= popup_rect.x
            && ev.column < popup_rect.x + popup_rect.width
            && ev.row >= popup_rect.y
            && ev.row < popup_rect.y + popup_rect.height;
        if !inside {
            return PopupOutcome::Ignored;
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
                        return PopupOutcome::Continued;
                    }
                }
            }
            // Click outside scrollbar: reset drag state on Up
            if matches!(ev.kind, MouseEventKind::Up(MouseButton::Left)) {
                self.scrollbar_drag.reset();
            }
            return PopupOutcome::Ignored;
        }

        match ev.kind {
            MouseEventKind::ScrollUp => {
                let step = self.wheel.step(WheelDirection::Up);
                self.move_by(step);
                PopupOutcome::Continued
            }
            MouseEventKind::ScrollDown => {
                let step = self.wheel.step(WheelDirection::Down);
                self.move_by(step);
                PopupOutcome::Continued
            }
            _ => PopupOutcome::Ignored,
        }
    }

    /// Route a single key event through the popup. The caller (AgentView)
    /// invokes this BEFORE forwarding to MultiLineInput.
    pub fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> PopupOutcome {
        if mods.contains(KeyModifiers::SHIFT) || mods.contains(KeyModifiers::CONTROL) {
            // Shift+arrow / Ctrl+anything reserved for AgentView's
            // navigation chords — never the popup's.
            return PopupOutcome::Ignored;
        }
        match code {
            KeyCode::Esc => PopupOutcome::Dismiss,
            KeyCode::Up => {
                self.move_by(-1);
                PopupOutcome::Continued
            }
            KeyCode::Down => {
                self.move_by(1);
                PopupOutcome::Continued
            }
            KeyCode::PageUp => {
                let step = -(self.visible_rows() as i32);
                self.move_by(step);
                PopupOutcome::Continued
            }
            KeyCode::PageDown => {
                let step = self.visible_rows() as i32;
                self.move_by(step);
                PopupOutcome::Continued
            }
            KeyCode::Home => {
                self.selected_index = 0;
                self.scroll_offset = 0;
                PopupOutcome::Continued
            }
            KeyCode::End => {
                self.go_end();
                PopupOutcome::Continued
            }
            KeyCode::Enter => match self.selected() {
                Some(cmd) => PopupOutcome::Selected(cmd.action),
                None => PopupOutcome::Dismiss,
            },
            KeyCode::Tab => match self.selected() {
                Some(cmd) => PopupOutcome::Filled(format!("/{}", cmd.name())),
                None => PopupOutcome::Dismiss,
            },
            _ => PopupOutcome::Ignored,
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let vr = (area.height as usize).saturating_sub(8).clamp(1, 20);
        self.last_visible_rows.set(vr);

        let dialog = FspecDialog {
            accent: Accent::Cyan,
            title: "Slash Commands",
            rows: self.build_rows(),
            footer: "↑↓ Navigate │ Tab/Enter Select │ Esc Close",
            min_width: 45,
query_row: None,
        };

        // TUI-103: compute the dialog rect so we can derive the body area
        // for scrollbar geometry. The dialog is shrink-to-content and
        // centered inside `area`.
        let d_rect = dialog_rect(area, &dialog);
        // Body content starts at d_rect.y + 4 (border + padding + title + gap)
        // and spans d_rect.height - 4 rows (minus border + padding).
        let body_origin = Rect {
            x: d_rect.x + 2,
            y: d_rect.y + 4,
            width: d_rect.width.saturating_sub(4).max(1),
            height: d_rect.height.saturating_sub(4).max(1),
        };

        // TUI-103: pre-compute scrollbar rect for hit-testing — spans the
        // dialog body area (rightmost column of body content), NOT the full
        // popup area.
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

    // Unit/legacy tests live in tests/rpc028_popup_scroll.rs as
    // integration tests so this file stays under the 300-LoC
    // source-shape budget. Only the snapshot test stays inline so the
    // insta snapshot path remains co-located with the renderer.

    #[test]
    fn slash_command_popup_rendering_is_byte_equal_across_runs_insta_snapshot() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let mut popup = SlashCommandPopup::new();
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
        insta::assert_yaml_snapshot!("slash_command_popup__centered_popup_80x24", rows);
    }
}
