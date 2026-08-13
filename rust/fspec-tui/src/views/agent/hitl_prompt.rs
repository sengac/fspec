//! RPC-411 — inline HITL prompt painted into the input area. Mirrors
//! `src/tui/components/InputTransition.tsx:385-463`.
//!
//! Feature: spec/features/inline-hitl-prompt.feature
//!
//! Options mode (wrapped header + one row per option + Other + footer):
//!   ⏸ [n/m] Header: Question?          ← ⏸+[n/m] magenta, header bold
//!    ● Option A — description          ← radio+label green (selected)
//!    ○ Option B — description          ← radio+label white, desc dim
//!    ○ Other...                        ← label dim + italic
//!    (↑/↓ Navigate | Enter Select | Esc Cancel)   ← dim
//!
//! Freeform / Other mode (header + optional hint + the SHARED input):
//!   ⏸ [n/m] Header: Question? (Enter Submit | Esc ...)  ← hint dim
//!     ⚠ Please type a response or press Esc to go back  ← yellow
//!   > <shared MultiLineInput, placeholder "Type your answer...">
//!
//! `[n/m]` renders ONLY when the request has more than one question.
//! The shared composer TextArea is rendered via `render_with_prompt`
//! — its draft state is never touched by the paint pass.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::store::agent_view::hitl_state::HitlPromptState;

use super::multiline_input::MultiLineInput;

/// Freeform placeholder (TS InputTransition.tsx:418).
pub const HITL_PLACEHOLDER: &str = "Type your answer...";
/// Yellow empty-submit hint (TS InputTransition.tsx:411-413).
pub const EMPTY_HINT: &str = "  ⚠ Please type a response or press Esc to go back";
/// Dim options-mode footer (TS InputTransition.tsx:459-461).
const OPTIONS_FOOTER: &str = " (↑/↓ Navigate | Enter Select | Esc Cancel)";

/// Header spans: magenta `⏸ `, magenta `[n/m] ` (multi-question only),
/// bold header, `: `, plain question, plus the dim mode hint in
/// freeform/Other mode.
fn header_spans(state: &HitlPromptState) -> Vec<Span<'static>> {
    let mut spans = vec![Span::styled("⏸ ", Style::default().fg(Color::Magenta))];
    let total = state.request.questions.len();
    if total > 1 {
        spans.push(Span::styled(
            format!("[{}/{}] ", state.question_index + 1, total),
            Style::default().fg(Color::Magenta),
        ));
    }
    let (header, question) = match state.current_question() {
        Some(q) => (q.header.clone(), q.question.clone()),
        None => (String::new(), String::new()),
    };
    spans.push(Span::styled(
        header,
        Style::default().add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::raw(": "));
    spans.push(Span::raw(question));
    if state.freeform_active() {
        let hint = if state.other_active {
            " (Enter Submit | Esc Back to options)"
        } else {
            " (Enter Submit | Esc Cancel)"
        };
        spans.push(Span::styled(
            hint,
            Style::default().add_modifier(Modifier::DIM),
        ));
    }
    spans
}

/// Slice styled spans into rows of at most `width` chars, preserving
/// style across the break (char-count proxy, per `pause_prompt.rs`).
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

fn header_rows(state: &HitlPromptState, width: u16) -> u16 {
    wrap_styled(header_spans(state), width).len() as u16
}

/// Input-area height needed at `width` (the padded body width):
/// options mode = wrapped header + options + Other + footer;
/// freeform/Other mode = wrapped header (+ hint row) + input rows.
pub fn prompt_height(state: &HitlPromptState, width: u16, input: &MultiLineInput) -> u16 {
    let header = header_rows(state, width);
    if state.freeform_active() {
        let hint = u16::from(state.show_empty_hint);
        let body_width = width.saturating_sub(super::multiline_input_render::PROMPT_WIDTH);
        let input_rows = input.visible_rows_for_width(body_width);
        header.saturating_add(hint).saturating_add(input_rows)
    } else {
        let options = state
            .current_question()
            .map(|q| q.options.len() as u16)
            .unwrap_or(0);
        // options + virtual Other... + footer
        header
            .saturating_add(options)
            .saturating_add(1)
            .saturating_add(1)
    }
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

/// Paint the wrapped header rows; returns the first row below them.
fn paint_header(area: Rect, buf: &mut Buffer, state: &HitlPromptState) -> u16 {
    let lines = wrap_styled(header_spans(state), area.width);
    let rows = lines.len() as u16;
    for (i, line) in lines.into_iter().enumerate() {
        paint_line(area, buf, i as u16, line);
    }
    rows
}

/// One option row: ` ● Label — description` / ` ○ Label` (TS
/// InputTransition.tsx:440-456). `Other...` is dim + italic.
fn option_line(
    label: &str,
    description: Option<&str>,
    selected: bool,
    is_other: bool,
) -> Line<'static> {
    let radio_style = if selected {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::White)
    };
    let radio = if selected { " ● " } else { " ○ " };
    let mut spans = vec![Span::styled(radio.to_string(), radio_style)];
    let label_style = if is_other {
        // Selected still tints green; label always dim + italic.
        radio_style.add_modifier(Modifier::DIM | Modifier::ITALIC)
    } else {
        radio_style
    };
    spans.push(Span::styled(label.to_string(), label_style));
    if let Some(desc) = description {
        if !desc.is_empty() {
            spans.push(Span::styled(
                format!(" — {desc}"),
                Style::default().add_modifier(Modifier::DIM),
            ));
        }
    }
    Line::from(spans)
}

/// Paint the inline HITL prompt into `area` (the padded input-area
/// body). Freeform/Other mode paints the SHARED composer input below
/// the header (and optional hint row) via `render_with_prompt`.
/// Returns the y offset (rows from area top) where the shared input
/// starts, or `None` in options mode (no hardware cursor).
pub fn render_hitl_prompt(
    area: Rect,
    buf: &mut Buffer,
    state: &HitlPromptState,
    input: &MultiLineInput,
) -> Option<u16> {
    if area.width == 0 || area.height == 0 {
        return None;
    }
    let mut next = paint_header(area, buf, state);
    if state.freeform_active() {
        if state.show_empty_hint {
            paint_line(
                area,
                buf,
                next,
                Line::from(Span::styled(EMPTY_HINT, Style::default().fg(Color::Yellow))),
            );
            next += 1;
        }
        let input_area = Rect {
            x: area.x,
            y: area.y.saturating_add(next),
            width: area.width,
            height: area.height.saturating_sub(next.min(area.height)),
        };
        input.render_with_prompt(input_area, buf, HITL_PLACEHOLDER);
        return Some(next);
    }
    if let Some(q) = state.current_question() {
        for (idx, option) in q.options.iter().enumerate() {
            paint_line(
                area,
                buf,
                next,
                option_line(
                    &option.label,
                    Some(&option.description),
                    idx == state.selected_option,
                    false,
                ),
            );
            next += 1;
        }
        paint_line(
            area,
            buf,
            next,
            option_line("Other...", None, state.other_selected(), true),
        );
        next += 1;
    }
    paint_line(
        area,
        buf,
        next,
        Line::from(Span::styled(
            OPTIONS_FOOTER,
            Style::default().add_modifier(Modifier::DIM),
        )),
    );
    None
}
