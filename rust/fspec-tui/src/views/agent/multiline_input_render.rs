//! RPC-405 — wrap-aware rendering + viewport for [`MultiLineInput`].
//!
//! Feature: spec/features/agent-input-soft-wrap-auto-grow.feature
//!
//! The render layer no longer delegates to tui-textarea's Widget impl
//! (which is wrap-hostile: 1 logical line = 1 visual row + horizontal
//! `Paragraph::scroll`). Instead the buffer is segmented into visual
//! rows by display width (`multiline_wrap`), a visual-row viewport
//! follows the cursor (`sync_viewport`, called by the AgentView
//! BEFORE painting), and only the visible segments are painted —
//! the head of the text is never pushed off-screen.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use super::multiline_input::MultiLineInput;
use super::multiline_wrap::{
    clamp_scroll_top, cursor_visual_position, next_scroll_top, total_visual_rows, wrap_lines,
};

/// RPC-404 — horizontal padding inside the input box border (1 col on
/// each side of the body).
pub(crate) const INPUT_PAD_X: u16 = 1;
/// RPC-404 — display width of the green "> " prompt prefix.
pub(crate) const PROMPT_WIDTH: u16 = 2;

/// RPC-404 — the ONE definition of the input body width: the padded
/// input-area width minus both side pads and the prompt. Layout
/// (`AgentView::render_with_store`), viewport sync, and the hardware
/// cursor (`hardware_cursor_in`) MUST all use this so wrap geometry
/// never diverges between paint and cursor placement.
pub(crate) fn input_body_width(area_width: u16) -> u16 {
    area_width.saturating_sub(2 * INPUT_PAD_X + PROMPT_WIDTH)
}

impl MultiLineInput {
    /// RPC-405 — wrap-aware height for the AgentView layout: total
    /// wrapped visual rows at `body_width`, clamped to
    /// `[1, max_visible_rows]`. MUST be fed the SAME body width the
    /// renderer paints with (input area minus 2x1 padding minus the
    /// 2-col "> " prompt) or rows will misalign.
    pub fn visible_rows_for_width(&self, body_width: u16) -> u16 {
        let total = total_visual_rows(self.lines(), body_width);
        let total = u16::try_from(total).unwrap_or(u16::MAX);
        total.clamp(1, self.max_visible_rows())
    }

    /// RPC-405 — logical→visual cursor mapping at `body_width`:
    /// (visual row index across the buffer, display column within
    /// that row). Consumed by RPC-404 hardware-cursor positioning.
    pub fn cursor_visual(&self, body_width: u16) -> (usize, u16) {
        cursor_visual_position(self.lines(), self.cursor(), body_width)
    }

    /// RPC-404 — map the logical cursor to hardware-cursor terminal
    /// coordinates inside the input box `area`. Uses the SAME wrap
    /// geometry as the render path: body width = area width minus 2x1
    /// padding minus the 2-col "> " prompt, x = left pad + prompt +
    /// display column, y = visual row relative to `scroll_top`. The
    /// result is clamped to `area` so the cursor can never escape the
    /// input viewport. Must be queried AFTER `sync_viewport`/render so
    /// `scroll_top` is current.
    pub fn hardware_cursor_in(&self, area: Rect) -> (u16, u16) {
        let body_width = input_body_width(area.width);
        let (vrow, vcol) = self.cursor_visual(body_width);
        let rel_row = vrow.saturating_sub(self.scroll_top());
        let rel_row = u16::try_from(rel_row).unwrap_or(u16::MAX);
        let x = area
            .x
            .saturating_add(INPUT_PAD_X)
            .saturating_add(PROMPT_WIDTH)
            .saturating_add(vcol);
        let y = area.y.saturating_add(rel_row);
        let max_x = area.x.saturating_add(area.width.saturating_sub(1));
        let max_y = area.y.saturating_add(area.height.saturating_sub(1));
        (x.clamp(area.x, max_x), y.clamp(area.y, max_y))
    }

    /// RPC-405 — run the cursor-follow algorithm in visual-row space
    /// and clamp to the content. The AgentView calls this from
    /// `render_with_store` BEFORE the immutable paint so `render`
    /// stays `&self`. RPC-429: caches `body_width` into
    /// `last_body_width` so the Up/Down boundary check can use visual
    /// row geometry.
    pub fn sync_viewport(&mut self, body_width: u16, height: u16) {
        self.last_body_width = Some(body_width);
        let height = height as usize;
        let total = total_visual_rows(self.lines(), body_width);
        let (cursor_row, _) = self.cursor_visual(body_width);
        let top = next_scroll_top(self.scroll_top(), cursor_row, height);
        self.set_scroll_top(clamp_scroll_top(top, total, height));
    }

    /// Paint the wrapped buffer into `area`: `area.height` visual
    /// rows starting at `scroll_top`, one plain `Line` per row (no
    /// horizontal scrolling — segments already fit `area.width`).
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let rows = wrap_lines(self.lines(), area.width);
        for (i, vrow) in rows
            .iter()
            .skip(self.scroll_top())
            .take(area.height as usize)
            .enumerate()
        {
            let row_area = Rect {
                x: area.x,
                y: area.y.saturating_add(i as u16),
                width: area.width,
                height: 1,
            };
            Paragraph::new(Line::from(vrow.text.clone())).render(row_area, buf);
        }
        // COPY-007: paint the REVERSED selection highlight over the body
        // rows AFTER the text. Spans are body-relative (prompt/pad
        // already excluded), so the `> ` columns are never highlighted.
        let spans = self.selection_highlight_spans(area.width);
        super::scrollback_paint::paint_selection_highlight(area, buf, &spans, area.width);
    }

    /// Paint the input box body: green "> " prompt prefix on the top
    /// row, then either the wrapped buffer content or a dim
    /// placeholder hint when the buffer is empty. Used by AgentView
    /// so the orchestrator stays under its 300-LoC ceiling.
    ///
    /// NOTE: `area` here is the ALREADY-PADDED body rect (the caller
    /// has stripped the 2×[`INPUT_PAD_X`] side pads), so only
    /// [`PROMPT_WIDTH`] is carved off before painting — together they
    /// reproduce [`input_body_width`].
    pub fn render_with_prompt(&self, area: Rect, buf: &mut Buffer, placeholder: &str) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let prompt_area = Rect {
            x: area.x,
            y: area.y,
            width: PROMPT_WIDTH.min(area.width),
            height: 1,
        };
        Paragraph::new(Line::from(Span::styled(
            "> ",
            Style::default().fg(Color::Green),
        )))
        .render(prompt_area, buf);

        let body_x = area.x.saturating_add(PROMPT_WIDTH);
        let body_width = area.width.saturating_sub(PROMPT_WIDTH);
        if body_width == 0 {
            return;
        }
        let body_area = Rect {
            x: body_x,
            y: area.y,
            width: body_width,
            height: area.height,
        };
        if self.is_empty() {
            let hint = Span::styled(placeholder, Style::default().fg(Color::DarkGray));
            Paragraph::new(Line::from(hint)).render(body_area, buf);
        } else {
            self.render(body_area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{input_body_width, INPUT_PAD_X, PROMPT_WIDTH};

    /// RPC-404 geometry pin: on a 60-wide input area the body is
    /// 60 − 2×1 pad − 2 prompt = 56 columns.
    #[test]
    fn input_body_width_of_60_is_56() {
        assert_eq!(input_body_width(60), 56);
        assert_eq!(2 * INPUT_PAD_X + PROMPT_WIDTH, 4);
    }
}
