//! TUI-116 — the stdin notice row and the separate stdin answer box.
//!
//! Feature: spec/features/stdin-notice-and-answer-box.feature
//!
//! A quiet command's stdin request no longer takes over the composer. The
//! focused pane paints one notice row on top of the composer (or the busy
//! stack):
//!
//! ```text
//!   ⌨ git commit may be waiting for input · Ctrl+O answer · Ctrl+X dismiss
//! ```
//!
//! Ctrl+O focuses the answer box, which has its OWN `MultiLineInput`, so the
//! chat draft can never be written to the command:
//!
//! ```text
//!   ⌨ git commit · your answer goes to its stdin
//!   > <answer box input, placeholder "Type to send to the command…">
//!   Enter send · Esc chat · Ctrl+C interrupt · Ctrl+D end input
//! ```
//!
//! Only the command display is shown; no command output is surfaced.
//!
//! TUI-117: the box is per session (`stdin_box_sessions.rs`), and a failed
//! send paints a red error row between the input and the hint.
//!
//! TUI-131 (spec/features/long-text-wraps-instead-of-being-cut-off.feature):
//! the notice, the header, the send error and the hint wrap to the pane
//! width (`stdin_answer_box/notice.rs`), and the input area is sized with
//! the wrapped rows, so the key hints are never pushed off screen.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget};

use codelet_rpc_types::{ExecStdinRequest, SessionId};

use super::multiline_input::MultiLineInput;
use super::text_wrap::wrapped_lines;
use super::{multiline_input_render, AgentView};

mod notice;

use notice::rows_u16;
pub(super) use notice::{below_notice, notice_rows, offset_below, paint_notice};

/// Dim tail of the answer box header, after the bold command display.
const BOX_HEADER_TAIL: &str = "your answer goes to its stdin";
/// Dim hint row under the answer box.
const BOX_HINT: &str = "Enter send · Esc chat · Ctrl+C interrupt · Ctrl+D end input";
/// Answer box placeholder.
const BOX_PLACEHOLDER: &str = "Type to send to the command…";

/// TUI-131: the answer box's header, send error and hint rows wrapped to
/// `width`, so the box is sized and painted from the same rows.
struct BoxRows {
    header: Vec<Line<'static>>,
    error: Vec<Line<'static>>,
    hint: Vec<Line<'static>>,
}

impl BoxRows {
    fn new(command: &str, error: Option<&str>, width: u16) -> Self {
        let width = usize::from(width);
        let dim = Style::default().add_modifier(Modifier::DIM);
        let header = (BOX_HEADER_TAIL.to_string(), dim);
        Self {
            header: notice::command_rows(command, " · ", &[header], width),
            error: error.map_or_else(Vec::new, |error| {
                wrapped_lines(error, width, Style::default().fg(Color::Red))
            }),
            hint: wrapped_lines(BOX_HINT, width, dim),
        }
    }
}

/// Paint `lines` into `rows` rows of `area` starting at row `y`.
fn paint_rows(area: Rect, buf: &mut Buffer, y: u16, rows: u16, lines: Vec<Line<'static>>) {
    if rows > 0 {
        Paragraph::new(lines).render(
            Rect {
                y,
                height: rows,
                ..area
            },
            buf,
        );
    }
}

impl AgentView {
    /// Input-area rows of `session`'s answer box for `command` while it is
    /// focused: the header, input, send-error and hint rows, each wrapped
    /// to the padded width (TUI-131). `None` while the box is not focused.
    pub(super) fn focused_stdin_box_height(
        &self,
        session: &SessionId,
        command: &str,
        area_width: u16,
    ) -> Option<u16> {
        if !self.stdin_box.is_focused_for(session) {
            return None;
        }
        let state = self.stdin_box.get(session)?;
        let body_width = multiline_input_render::input_body_width(area_width);
        let padded_width = area_width.saturating_sub(2 * area_width.min(1));
        let rows = BoxRows::new(command, state.error(), padded_width);
        Some(
            state
                .input
                .visible_rows_for_width(body_width)
                .saturating_add(rows_u16(&rows.header))
                .saturating_add(rows_u16(&rows.error))
                .saturating_add(rows_u16(&rows.hint)),
        )
    }

    /// Paint the focused answer box into `padded` and anchor the hardware
    /// cursor on its input row.
    pub(super) fn paint_stdin_box(
        &mut self,
        input_area: Rect,
        padded: Rect,
        buf: &mut Buffer,
        session: &SessionId,
        request: &ExecStdinRequest,
    ) {
        if padded.width == 0 || padded.height == 0 {
            return;
        }
        let state = self.stdin_box.session_mut(session);
        let rows = BoxRows::new(&request.command, state.error(), padded.width);
        // The header first, then the hint and the error while an input
        // row is left for each (TUI-117 degrade order on a short area).
        let header_rows = rows_u16(&rows.header).min(padded.height);
        let rest = padded.height - header_rows;
        let hint_rows = rows_u16(&rows.hint).min(rest.saturating_sub(1));
        let error_rows = rows_u16(&rows.error).min(rest.saturating_sub(1 + hint_rows));
        let input_rows = rest - hint_rows - error_rows;
        paint_rows(padded, buf, padded.y, header_rows, rows.header);
        let input_rect = Rect {
            y: padded.y.saturating_add(header_rows),
            height: input_rows,
            ..padded
        };
        let body_width = multiline_input_render::input_body_width(input_area.width);
        state.input.sync_viewport(body_width, input_rows);
        if input_rows > 0 {
            state
                .input
                .render_with_prompt(input_rect, buf, BOX_PLACEHOLDER);
        }
        let error_y = input_rect.y.saturating_add(input_rows);
        paint_rows(padded, buf, error_y, error_rows, rows.error);
        let hint_y = error_y.saturating_add(error_rows);
        paint_rows(padded, buf, hint_y, hint_rows, rows.hint);
        self.last_hitl_input_offset = Some(header_rows);
    }

    /// True while the answer box holds focus for the painted request.
    pub(crate) fn stdin_box_focused(&self) -> bool {
        self.last_exec_stdin
            .as_ref()
            .is_some_and(|(session, _)| self.stdin_box.is_focused_for(session))
    }

    /// The input that owns the hardware cursor: the answer box while it is
    /// focused, the chat composer otherwise.
    pub(super) fn focused_input(&self) -> &MultiLineInput {
        let focused_box = self
            .last_exec_stdin
            .as_ref()
            .filter(|(session, _)| self.stdin_box.is_focused_for(session))
            .and_then(|(session, _)| self.stdin_box.get(session));
        match focused_box {
            Some(state) => &state.input,
            None => &self.input,
        }
    }
}
