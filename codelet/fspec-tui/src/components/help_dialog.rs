//! Critical-priority Help dialog (RPC-008 rule [15]).
//!
//! Feature: spec/features/fspec-tui-help-dialog.feature
//! Feature: spec/features/rpc027-help-disconnect-thinking-dialogs.feature
//! Feature: spec/features/scrollable-space-filling-help-dialog-with-scrollbar.feature
//!
//! Triggered by the `?` key at App-level (NOT inside HelloComponent —
//! the App layer pushes this onto the compositor). Body lists exactly
//! the `?`, ESC, and `q` keybindings. ESC returns
//! `EventResult::Consumed(Some(callback))` where the callback removes
//! the dialog by id.
//!
//! RPC-027: renders via the shared dialog_theme renderer so the cyan
//! border, bold inner title, and dim centered footer match the
//! TypeScript Ink reference exactly.
//!
//! RPC-396: the dialog now FILLS the terminal (minus a small margin) and
//! SCROLLS its content (↑/↓ line, PageUp/PageDown page, Home/End, mouse
//! wheel) rendering the shared `render_list_scrollbar` in a reserved
//! 1-column gutter whenever the content overflows the visible window.
//!
//! TUI-101: scrollbar click-and-drag navigation via `ScrollbarDrag`.

use crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Span;

use super::dialog_theme::{render_dialog_at, Accent, DialogRow};
use super::dialog_theme_rows::build_dialog;
use super::help_content::{agent_help_lines, board_help_lines};
use super::help_dialog_scroll::{
    content_rows, fill_rect, gutter_rect, max_offset, wheel_direction,
};
use super::list_scrollbar::render_list_scrollbar;
use crate::mouse::rect_contains;
use crate::mouse::scrollbar_drag::{ScrollbarDrag, ScrollbarGeometry};
use super::scroll_viewport::WheelVelocity;
use super::{Callback, Component, EventResult, Priority};

/// Critical-priority modal dialog listing view-specific keybindings.
///
/// RPC-397: the content is parameterized per view — [`HelpDialog::for_board`]
/// lists board keybindings only, [`HelpDialog::for_agent`] lists agent
/// keybindings plus the full slash-command registry. The id stays
/// `"help-dialog"` so the compositor `.contains` guards are unaffected.
pub struct HelpDialog {
    id: String,
    lines: Vec<String>,
    scroll_offset: usize,
    visible_rows: usize,
    wheel: WheelVelocity,
    /// TUI-101: scrollbar click-and-drag state machine.
    scrollbar_drag: ScrollbarDrag,
    /// TUI-101: cached gutter rect from last render for hit-testing.
    last_gutter: Option<Rect>,
}

impl Default for HelpDialog {
    fn default() -> Self {
        Self::for_board()
    }
}

impl HelpDialog {
    /// Construct a HelpDialog with the canonical id `"help-dialog"`.
    ///
    /// Retained for existing callers; delegates to [`HelpDialog::for_board`]
    /// (the board `?` shortcut is the historical default entry point).
    pub fn new() -> Self {
        Self::for_board()
    }

    /// RPC-397: board variant — board keybindings only, NO slash commands.
    pub fn for_board() -> Self {
        Self::with_lines(board_help_lines())
    }

    /// RPC-397: agent variant — agent keybindings PLUS the full slash
    /// command list with descriptions.
    pub fn for_agent() -> Self {
        Self::with_lines(agent_help_lines())
    }

    /// Shared constructor: the two variants differ only in `lines`.
    fn with_lines(lines: Vec<String>) -> Self {
        Self {
            id: "help-dialog".to_string(),
            lines,
            scroll_offset: 0,
            visible_rows: 0,
            wheel: WheelVelocity::new(),
            scrollbar_drag: ScrollbarDrag::new(),
            last_gutter: None,
        }
    }

    /// RPC-396 test seam: construct a HelpDialog with an injected content
    /// vec so scroll/sizing behaviour can be exercised deterministically,
    /// independent of the real view content.
    #[doc(hidden)]
    pub fn with_lines_for_test(lines: Vec<String>) -> Self {
        Self::with_lines(lines)
    }

    /// Current scroll offset (index of the first visible content line).
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Body height measured on the last `render` — the number of content
    /// rows the visible window can show.
    pub fn visible_rows(&self) -> usize {
        self.visible_rows
    }

    fn max_offset(&self) -> usize {
        max_offset(self.lines.len(), self.visible_rows)
    }

    fn dismiss(&self) -> EventResult {
        let id = self.id.clone();
        let callback: Callback = Box::new(move |compositor| {
            let _ = compositor.remove(&id);
        });
        EventResult::Consumed(Some(callback))
    }
}

impl Component for HelpDialog {
    fn priority(&self) -> Priority {
        Priority::Critical
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        let max = self.max_offset();
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Esc => return self.dismiss(),
                KeyCode::Up => {
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
                    return EventResult::consumed();
                }
                KeyCode::Down => {
                    self.scroll_offset = (self.scroll_offset + 1).min(max);
                    return EventResult::consumed();
                }
                KeyCode::PageUp => {
                    self.scroll_offset = self.scroll_offset.saturating_sub(self.visible_rows);
                    return EventResult::consumed();
                }
                KeyCode::PageDown => {
                    self.scroll_offset = (self.scroll_offset + self.visible_rows).min(max);
                    return EventResult::consumed();
                }
                KeyCode::Home => {
                    self.scroll_offset = 0;
                    return EventResult::consumed();
                }
                KeyCode::End => {
                    self.scroll_offset = max;
                    return EventResult::consumed();
                }
                _ => {}
            }
        }
        // TUI-101: scrollbar click-and-drag navigation.
        if let Event::Mouse(mouse_event) = event {
            let max = self.max_offset();
            let total = self.lines.len();
            let visible = self.visible_rows;

            // Only handle left button events when scrollbar is visible.
            if total > visible {
                // Hit-test against the cached gutter rect.
                if let Some(gutter) = self.last_gutter {
                    let inside = rect_contains(gutter, mouse_event.column, mouse_event.row);

                    match mouse_event.kind {
                        MouseEventKind::Down(MouseButton::Left)
                        | MouseEventKind::Drag(MouseButton::Left)
                        | MouseEventKind::Up(MouseButton::Left) => {
                            if inside {
                                let geom = ScrollbarGeometry {
                                    area_height: visible,
                                    total_items: total,
                                    visible_items: visible,
                                    current_offset: self.scroll_offset,
                                };
                                if let Some(offset) = self.scrollbar_drag.on_mouse(*mouse_event, geom) {
                                    self.scroll_offset = offset.min(max);
                                }
                                return EventResult::consumed();
                            } else {
                                // Reset drag state when clicking outside scrollbar
                                if matches!(mouse_event.kind, MouseEventKind::Up(MouseButton::Left)) {
                                    self.scrollbar_drag.reset();
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            return EventResult::ignored();
        }
        // RPC-396: mouse wheel scrolls the content. The `Event::Mouse`
        // match lives in `help_dialog_scroll::wheel_direction` so this
        // dialog shell stays `Event::Key`-only (RPC-023 source-shape
        // guard) while still gaining wheel scrolling.
        if let Some(dir) = wheel_direction(event) {
            let step = self.wheel.step(dir);
            let proposed = self.scroll_offset as i64 + step as i64;
            self.scroll_offset = proposed.clamp(0, max as i64) as usize;
            return EventResult::consumed();
        }
        // RPC-403 review: Critical modal — consume (swallow) pastes so
        // they can never leak into the agent input hidden behind this
        // dialog. No text field here, so nothing is inserted.
        if matches!(event, Event::Paste(_)) {
            return EventResult::consumed();
        }
        EventResult::ignored()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let rect = fill_rect(area);
        let total = self.lines.len();
        self.visible_rows = content_rows(rect);
        // Clamp the offset against the freshly-measured window.
        self.scroll_offset = self.scroll_offset.min(self.max_offset());

        let visible = self.visible_rows;
        let end = (self.scroll_offset + visible).min(total);
        let rows: Vec<DialogRow> = self.lines[self.scroll_offset..end]
            .iter()
            .map(|line| DialogRow {
                spans: vec![Span::raw(line.clone())],
                selectable: false,
                selected: false,
            })
            .collect();

        let dialog = build_dialog(Accent::Cyan, "Help", rows, "ESC to close", 30);
        render_dialog_at(rect, buf, &dialog);

        // Overflow → paint the shared proportional scrollbar in a reserved
        // 1-column gutter. When everything fits, paint nothing.
        if total > visible {
            if let Some(gutter) = gutter_rect(rect, visible) {
                // TUI-101: cache the gutter rect for mouse hit-testing.
                self.last_gutter = Some(gutter);
                render_list_scrollbar(gutter, buf, self.scroll_offset, visible, total);
            }
        } else {
            self.last_gutter = None;
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use super::*;

    fn render_help_dialog_80x24() -> Buffer {
        let mut dialog = HelpDialog::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
        terminal
            .draw(|frame| {
                dialog.render(frame.area(), frame.buffer_mut());
            })
            .expect("draw");
        terminal.backend().buffer().clone()
    }

    fn buffer_text(buf: &Buffer) -> String {
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn help_dialog_is_critical_priority_with_canonical_id() {
        let dialog = HelpDialog::new();
        assert_eq!(dialog.priority(), Priority::Critical);
        assert_eq!(dialog.id(), "help-dialog");
    }

    #[test]
    fn help_dialog_body_lists_the_board_keybindings() {
        // RPC-397: the default (board) HelpDialog now lists accurate
        // board keybindings. At 80x24 only the top of the list is
        // visible (the dialog scrolls — RPC-396), so we assert the
        // top-of-list board hints AND the absence of the old
        // misleading "q       Quit" line.
        let buf = render_help_dialog_80x24();
        let text = buffer_text(&buf);
        for needle in &["Navigate", "New Agent", "Reorder"] {
            assert!(
                text.contains(needle),
                "buffer must contain {needle}: {text}"
            );
        }
        assert!(
            !text.contains("q       Quit"),
            "board help must NOT contain the old \"q       Quit\" line: {text}"
        );
        // Full accurate content (incl. the Ctrl+D quit line) is asserted
        // against a 200x60 backend in tests/help_dialog_content_rpc397.rs
        // where the whole list fits without scrolling.
    }

    #[test]
    fn help_dialog_rendering_is_byte_equal_across_runs_insta_snapshot() {
        let buf = render_help_dialog_80x24();
        let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
        for y in 0..buf.area.height {
            let mut row = String::with_capacity(buf.area.width as usize);
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            rows.push(row);
        }
        insta::assert_yaml_snapshot!("help_dialog__centered_popup_80x24", rows);
    }
}
