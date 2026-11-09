//! TUI-131 — the exec-stdin notice and answer-box header rows, wrapped.
//!
//! Feature: spec/features/long-text-wraps-instead-of-being-cut-off.feature
//!
//! `⌨ <command>` and its tail segments (status, key hints) stay on one
//! row when they fit, exactly as TUI-116 painted them. When they do not,
//! the command wraps onto its own rows and the segments follow, joined by
//! ` · ` while they fit on a row and each wrapped when it is wider than
//! the row, so a long command or probe error never pushes
//! "Ctrl+O answer · Ctrl+X dismiss" off screen. The row counts the input
//! area is sized with come from the same lines that are painted.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use unicode_width::UnicodeWidthStr;

use crate::views::agent::text_wrap::{wrap_rows_hanging, wrapped_lines};

/// The notice's key hints.
const NOTICE_KEYS: &str = "Ctrl+O answer · Ctrl+X dismiss";
/// The notice's status while the request is current.
const WAITING: &str = "may be waiting for input";
/// Joins the tail segments on one row.
const JOIN: &str = " · ";

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// The number of `lines`, as a row count.
pub(super) fn rows_u16(lines: &[Line<'_>]) -> u16 {
    u16::try_from(lines.len()).unwrap_or(u16::MAX)
}

/// `⌨ ` (magenta) and the bold `command`, then `segments`: on one row
/// (`joiner` before the first segment, ` · ` between the others) when it
/// fits `width`, else the command wrapped onto its own rows and the
/// segments after it (see the module doc).
pub(crate) fn command_rows(
    command: &str,
    joiner: &str,
    segments: &[(String, Style)],
    width: usize,
) -> Vec<Line<'static>> {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let glyph = Span::styled("⌨ ", Style::default().fg(Color::Magenta));
    let tail_width: usize = segments.iter().map(|(text, _)| text.width()).sum::<usize>()
        + joiner.width()
        + JOIN.width() * segments.len().saturating_sub(1);
    if 2 + command.width() + tail_width <= width {
        let mut spans = vec![glyph, Span::styled(command.to_string(), bold)];
        for (index, (text, style)) in segments.iter().enumerate() {
            let before = if index == 0 { joiner } else { JOIN };
            spans.push(Span::styled(format!("{before}{text}"), *style));
        }
        return vec![Line::from(spans)];
    }
    let mut lines: Vec<Line<'static>> = wrap_rows_hanging(command, width.saturating_sub(2), width)
        .into_iter()
        .enumerate()
        .map(|(index, (row, _))| match index {
            0 => Line::from(vec![glyph.clone(), Span::styled(row, bold)]),
            _ => Line::from(Span::styled(row, bold)),
        })
        .collect();
    let mut current: Vec<Span<'static>> = Vec::new();
    let mut used = 0;
    for (text, style) in segments {
        let text_width = text.width();
        if !current.is_empty() && used + JOIN.width() + text_width <= width {
            current.push(Span::styled(JOIN, *style));
            current.push(Span::styled(text.clone(), *style));
            used += JOIN.width() + text_width;
            continue;
        }
        if !current.is_empty() {
            lines.push(Line::from(std::mem::take(&mut current)));
        }
        if text_width <= width {
            current.push(Span::styled(text.clone(), *style));
            used = text_width;
        } else {
            lines.extend(wrapped_lines(text, width, *style));
            used = 0;
        }
    }
    if !current.is_empty() {
        lines.push(Line::from(current));
    }
    lines
}

/// The notice for `command` at `width`. BUG-364: a failed probe
/// (`probe_error`) marks it as possibly stale, in red, before the keys.
fn notice_lines(command: &str, probe_error: Option<&str>, width: u16) -> Vec<Line<'static>> {
    let status = match probe_error {
        Some(error) => (
            format!("may be stale · probe failed: {error}"),
            Style::default().fg(Color::Red),
        ),
        None => (WAITING.to_string(), dim()),
    };
    let segments = [status, (NOTICE_KEYS.to_string(), dim())];
    command_rows(command, " ", &segments, usize::from(width))
}

/// The rows the notice takes at `width` (the padded input-area width).
pub(crate) fn notice_rows(command: &str, probe_error: Option<&str>, width: u16) -> u16 {
    rows_u16(&notice_lines(command, probe_error, width))
}

/// Paint the notice into the top rows of `area`; returns the rows it
/// took (at most `area.height`).
pub(crate) fn paint_notice(
    area: Rect,
    buf: &mut Buffer,
    command: &str,
    probe_error: Option<&str>,
) -> u16 {
    let lines = notice_lines(command, probe_error, area.width);
    let rows = rows_u16(&lines).min(area.height);
    Paragraph::new(lines).render(
        Rect {
            height: rows,
            ..area
        },
        buf,
    );
    rows
}

/// `area` minus its top `rows` (the rows under the notice).
pub(crate) fn below_notice(area: Rect, rows: u16) -> Rect {
    let rows = rows.min(area.height);
    Rect {
        y: area.y.saturating_add(rows),
        height: area.height - rows,
        ..area
    }
}

/// Shift a composer cursor offset down by the `rows` painted above it.
pub(crate) fn offset_below(offset: Option<u16>, rows: u16) -> Option<u16> {
    match (offset, rows) {
        (offset, 0) => offset,
        (Some(offset), rows) => Some(offset.saturating_add(rows)),
        (None, rows) => Some(rows),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(lines: &[Line<'static>]) -> Vec<String> {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn a_short_notice_stays_on_one_row() {
        let lines = notice_lines("git commit", None, 78);
        assert_eq!(
            texts(&lines),
            vec!["⌨ git commit may be waiting for input · Ctrl+O answer · Ctrl+X dismiss"]
        );
    }

    #[test]
    fn a_long_probe_error_wraps_and_the_keys_keep_their_own_row() {
        let error = "transport closed ".repeat(6);
        let lines = notice_lines("git commit", Some(error.trim_end()), 40);
        let rows = texts(&lines);
        assert_eq!(rows.first().map(String::as_str), Some("⌨ git commit"));
        assert_eq!(rows.last().map(String::as_str), Some(NOTICE_KEYS));
        assert!(rows.iter().all(|row| row.width() <= 40), "{rows:?}");
    }
}
