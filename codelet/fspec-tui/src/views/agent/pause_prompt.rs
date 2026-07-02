//! RPC-406 — inline tool-approval pause prompt painted into the input
//! area. Mirrors `src/tui/components/InputTransition.tsx:467-533`.
//!
//! Feature: spec/features/inline-tool-approval-pause-prompt.feature
//!
//! Triple kind (1+N header rows + options row):
//!   ⏸ {prompt} ({details})                       ← ⏸+tool cyan, details dim,
//!                                                   wrapped at the body width
//!   [Allow Once] [Allow Session] [Deny] (hint)   ← green/blue/red, sel inverse
//!
//! Confirm kind (1+N header rows, optional details row, Y/N row):
//!   ⏸ {prompt}                                   ← yellow, wrapped
//!     {details}                                  ← dim, own line, optional
//!   [Y] Approve [N] Deny (Esc to cancel)         ← green / red / dim
//!
//! Long headers WRAP instead of clipping (TS Ink `<Text>` wraps by
//! default): `wrap_styled` slices the styled header at the body width
//! (char-count proxy, consistent with `text_wrap.rs`), and
//! `prompt_height` counts the wrapped rows so the RPC-405 auto-grow
//! seam gives the prompt enough input-area rows for BOTH kinds.
//!
//! The wire `prompt` already carries `"{tool_name}: {message}"`
//! (`sessions/src/conversions.rs:52-57`); `tool_call_id` carries the
//! gated command / file path (`details`). The wire `PauseKind`
//! collapses Continue into Confirm — Confirm rendering applies.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use codelet_rpc_types::{PauseKind, PauseState};

/// The three triple-prompt options in selection order.
const TRIPLE_OPTIONS: [(&str, Color); 3] = [
    ("Allow Once", Color::Green),
    ("Allow Session", Color::Blue),
    ("Deny", Color::Red),
];

const TRIPLE_HINT: &str = " (←/→ Navigate | Enter Select | Esc Deny)";

/// Input-area height needed by the prompt at `width` (the padded
/// input-area body width the prompt renders into): the wrapped header
/// rows plus the options row (Triple) or plus the optional details
/// line and the Y/N row (Confirm).
pub fn prompt_height(state: &PauseState, width: u16) -> u16 {
    let header_rows = wrap_styled(header_spans(state), width).len() as u16;
    match state.kind {
        PauseKind::Triple => header_rows.saturating_add(1),
        PauseKind::Confirm => {
            let details_rows = u16::from(state.tool_call_id.is_some());
            header_rows.saturating_add(details_rows).saturating_add(1)
        }
    }
}

/// Paint the inline pause prompt into `area` (the padded input-area
/// body). `selection` is the triple-prompt selection (ignored for
/// Confirm).
pub fn render_pause_prompt(area: Rect, buf: &mut Buffer, state: &PauseState, selection: usize) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    match state.kind {
        PauseKind::Triple => render_triple(area, buf, state, selection),
        PauseKind::Confirm => render_confirm(area, buf, state),
    }
}

/// The styled header spans for either kind: Triple is a cyan
/// `⏸ {prompt}` plus dim ` ({details})`; Confirm is a yellow
/// `⏸ {prompt}` (its details render on their own row).
fn header_spans(state: &PauseState) -> Vec<Span<'static>> {
    match state.kind {
        PauseKind::Triple => {
            let mut spans = vec![Span::styled(
                format!("⏸ {}", state.prompt),
                Style::default().fg(Color::Cyan),
            )];
            if let Some(details) = &state.tool_call_id {
                spans.push(Span::styled(
                    format!(" ({details})"),
                    Style::default().add_modifier(Modifier::DIM),
                ));
            }
            spans
        }
        PauseKind::Confirm => vec![Span::styled(
            format!("⏸ {}", state.prompt),
            Style::default().fg(Color::Yellow),
        )],
    }
}

/// Slice styled spans into rows of at most `width` chars, preserving
/// each char's style across the break (no characters are dropped —
/// char count is the visual-width proxy, per `text_wrap.rs`).
fn wrap_styled(spans: Vec<Span<'static>>, width: u16) -> Vec<Line<'static>> {
    let width = usize::from(width.max(1));
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current: Vec<Span<'static>> = Vec::new();
    let mut used = 0usize;
    for span in spans {
        let style = span.style;
        let mut piece = String::new();
        for ch in span.content.chars() {
            if used == width {
                if !piece.is_empty() {
                    current.push(Span::styled(std::mem::take(&mut piece), style));
                }
                lines.push(Line::from(std::mem::take(&mut current)));
                used = 0;
            }
            piece.push(ch);
            used += 1;
        }
        if !piece.is_empty() {
            current.push(Span::styled(piece, style));
        }
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(Line::from(current));
    }
    lines
}

fn row(area: Rect, i: u16) -> Rect {
    Rect {
        x: area.x,
        y: area.y.saturating_add(i),
        width: area.width,
        height: 1,
    }
}

fn paint_line(area: Rect, buf: &mut Buffer, i: u16, line: Line<'_>) {
    if i < area.height {
        Paragraph::new(line).render(row(area, i), buf);
    }
}

/// Paint the wrapped header rows; returns the first row index below
/// the header.
fn paint_header(area: Rect, buf: &mut Buffer, state: &PauseState) -> u16 {
    let lines = wrap_styled(header_spans(state), area.width);
    let rows = lines.len() as u16;
    for (i, line) in lines.into_iter().enumerate() {
        paint_line(area, buf, i as u16, line);
    }
    rows
}

/// TS InputTransition.tsx:490-521 — cyan wrapped header + options row.
fn render_triple(area: Rect, buf: &mut Buffer, state: &PauseState, selection: usize) {
    let next = paint_header(area, buf, state);

    let mut options: Vec<Span<'static>> = Vec::new();
    for (idx, (label, color)) in TRIPLE_OPTIONS.iter().enumerate() {
        if idx > 0 {
            options.push(Span::raw(" "));
        }
        let mut style = Style::default().fg(*color);
        if idx == selection {
            style = style.add_modifier(Modifier::REVERSED);
        }
        options.push(Span::styled(format!("[{label}]"), style));
    }
    options.push(Span::styled(
        TRIPLE_HINT,
        Style::default().add_modifier(Modifier::DIM),
    ));
    paint_line(area, buf, next, Line::from(options));
}

/// TS InputTransition.tsx:468-489 — yellow wrapped header, optional
/// dim details line, Y/N row.
fn render_confirm(area: Rect, buf: &mut Buffer, state: &PauseState) {
    let mut next = paint_header(area, buf, state);
    if let Some(details) = &state.tool_call_id {
        paint_line(
            area,
            buf,
            next,
            Line::from(Span::styled(
                format!("  {details}"),
                Style::default().add_modifier(Modifier::DIM),
            )),
        );
        next += 1;
    }
    paint_line(
        area,
        buf,
        next,
        Line::from(vec![
            Span::styled("[Y] Approve", Style::default().fg(Color::Green)),
            Span::raw(" "),
            Span::styled("[N] Deny", Style::default().fg(Color::Red)),
            Span::styled(
                " (Esc to cancel)",
                Style::default().add_modifier(Modifier::DIM),
            ),
        ]),
    );
}
