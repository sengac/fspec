//! TOOL-022 P2 — inline exec-stdin prompt painted into the input area.
//!
//! Feature: spec/features/exec-stdin-prompt.feature
//!
//! Freeform-only overlay (mirrors the HITL freeform branch but WITHOUT
//! the options machine):
//!
//! ```text
//!   ⌨ git commit has been quiet for 5s — waiting for input?   ← magenta
//!                                                                          glyph, bold
//!                                                                          command, dim
//!                                                                          tail
//!   > <shared MultiLineInput, placeholder "Type to send to the command…">
//!   (Enter Send | Esc Dismiss)                                           ← dim
//! ```
//!
//! Only command display + quiet_seconds are shown — NO output content
//! is ever surfaced (the request carries no hint/content field). The
//! shared composer TextArea is rendered via `render_with_prompt` — its
//! draft state is never touched by the paint pass (only the deliberate
//! submit capture in `exec_stdin_keys.rs` mutates it).
//!
//! Precedence (enforced in `input_area.rs`): HITL > exec-stdin > pause
//! > composer.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use codelet_rpc_types::ExecStdinRequest;

use super::multiline_input::MultiLineInput;

/// Shared-input placeholder for the exec-stdin prompt.
pub const EXEC_STDIN_PLACEHOLDER: &str = "Type to send to the command…";
/// Dim footer under the shared input.
const EXEC_STDIN_FOOTER: &str = " (Enter Send | Esc Dismiss)";

/// Prompt-row spans: magenta `⌨ `, bold command display, dim
/// " has been quiet for {N}s — waiting for input?".
fn header_spans(request: &ExecStdinRequest) -> Vec<Span<'static>> {
    vec![
        Span::styled("⌨ ", Style::default().fg(Color::Magenta)),
        Span::styled(
            request.command.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                " has been quiet for {}s — waiting for input?",
                request.quiet_seconds
            ),
            Style::default().add_modifier(Modifier::DIM),
        ),
    ]
}

/// Paint the inline exec-stdin prompt into `area` (the padded
/// input-area body). Layout: row 0 = header, rows 1..h-2 = the SHARED
/// composer input, last row = dim footer. Returns the y offset (rows
/// from area top) where the shared input starts.
pub fn render_exec_stdin_prompt(
    area: Rect,
    buf: &mut Buffer,
    request: &ExecStdinRequest,
    input: &MultiLineInput,
) -> Option<u16> {
    if area.width == 0 || area.height == 0 {
        return None;
    }
    // Row 0: header.
    Paragraph::new(Line::from(header_spans(request)))
        .render(area, buf);
    // Rows 1..h-1: the shared MultiLineInput (placeholder "Type to
    // send to the command…"). The last row is reserved for the dim
    // footer (only when there is room for it).
    let footer_rows = u16::from(area.height > 2);
    let input_height = area.height.saturating_sub(1 + footer_rows);
    if input_height > 0 {
        let input_area = Rect {
            x: area.x,
            y: area.y.saturating_add(1),
            width: area.width,
            height: input_height,
        };
        input.render_with_prompt(input_area, buf, EXEC_STDIN_PLACEHOLDER);
    }
    if area.height > 2 {
        let footer = Rect {
            x: area.x,
            y: area.y.saturating_add(area.height - 1),
            width: area.width,
            height: 1,
        };
        Paragraph::new(Line::from(Span::styled(
            EXEC_STDIN_FOOTER,
            Style::default().add_modifier(Modifier::DIM),
        )))
        .render(footer, buf);
    }
    Some(1)
}

/// Input-area height the exec-stdin overlay needs at `width` (the
/// padded body width, same as `hitl_prompt::prompt_height`): header
/// + shared input rows + footer row.
pub fn prompt_height(_request: &ExecStdinRequest, width: u16, input: &MultiLineInput) -> u16 {
    let body_width = width.saturating_sub(super::multiline_input_render::PROMPT_WIDTH);
    let input_rows = input.visible_rows_for_width(body_width);
    1_u16.saturating_add(input_rows).saturating_add(1) // header + input + footer
}
