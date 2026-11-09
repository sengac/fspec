//! TUI-131 — the diff pane shared by the Changed Files and Checkpoints
//! views: the diff lines wrapped to the pane width, scrolled in wrapped
//! rows.
//!
//! Feature: spec/features/long-text-wraps-instead-of-being-cut-off.feature
//!
//! A diff line wider than the pane wraps onto more rows
//! (`text_wrap::wrap_rows_hanging`), its leading `+`/`-` marker and
//! indentation kept on the first row. The pane's layout — the wrap width
//! (one column narrower when the scrollbar shows) and the wrapped rows —
//! is computed in one place, [`diff_rows_in`], which both the painter and
//! the views' scroll math ([`max_diff_scroll`], [`diff_row_count`]) use,
//! so a scroll offset always counts the rows that are painted. Copying
//! (TUI-130 `C`) still reads the unwrapped lines.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use unicode_width::UnicodeWidthStr;

use super::diff_render::{diff_style, sanitized_kind};
use super::{pane_header, render_pane_scrollbar};
use crate::views::agent::text_wrap::wrap_rows_hanging;

/// One diff line wrapped to `width`: its leading marker and indentation
/// (`+    `) stay on the first row, the rest wraps after it.
fn wrapped_diff_line(text: &str, width: usize, out: &mut Vec<Line<'static>>) {
    let (sanitized, kind) = sanitized_kind(text);
    let style = diff_style(kind);
    let body = sanitized.trim_start_matches(['+', '-', ' ']);
    let prefix = &sanitized[..sanitized.len() - body.len()];
    let prefix_width = prefix.width();
    let (prefix, body) = if prefix_width < width {
        (prefix, body)
    } else {
        ("", sanitized.as_str())
    };
    let first_width = width.saturating_sub(prefix.width());
    for (index, (row, _)) in wrap_rows_hanging(body, first_width, width)
        .into_iter()
        .enumerate()
    {
        let row = if index == 0 {
            format!("{prefix}{row}")
        } else {
            row
        };
        out.push(Line::from(Span::styled(row, style)));
    }
}

/// `lines` wrapped to `width`, one styled `Line` per painted row.
fn diff_rows(lines: &[String], width: usize) -> Vec<Line<'static>> {
    let mut rows = Vec::with_capacity(lines.len());
    for line in lines {
        wrapped_diff_line(line, width, &mut rows);
    }
    rows
}

/// The rows the diff pane paints for `lines` in `content`, and whether
/// they overflow it (the scrollbar then takes the last column, so the
/// rows are wrapped one column narrower).
fn diff_rows_in(lines: &[String], content: Rect) -> (Vec<Line<'static>>, bool) {
    let visible = usize::from(content.height);
    let rows = diff_rows(lines, usize::from(content.width));
    if rows.len() <= visible {
        return (rows, false);
    }
    let narrower = usize::from(content.width.saturating_sub(1));
    (diff_rows(lines, narrower), true)
}

/// The number of rows `lines` take in the diff pane's `content` rect
/// (the last render's). Before the first render (`None`) each line
/// counts one row.
pub fn diff_row_count(lines: &[String], content: Option<Rect>) -> usize {
    match content {
        Some(content) => diff_rows_in(lines, content).0.len(),
        None => lines.len(),
    }
}

/// The diff pane's largest scroll offset, in wrapped rows: the last page
/// ends on the last row. Before the first render (no `content`, or a
/// zero-height one) the pane height is unknown, so the offset may reach
/// the last line.
pub fn max_diff_scroll(lines: &[String], content: Option<Rect>) -> usize {
    match content.filter(|content| content.height > 0) {
        Some(content) => {
            diff_row_count(lines, Some(content)).saturating_sub(usize::from(content.height))
        }
        None => lines.len().saturating_sub(1),
    }
}

/// Paint the diff pane into `area`: the focus-aware header, the wrapped
/// rows from `scroll` (clamped to [`max_diff_scroll`]) and the scrollbar
/// when they overflow. Returns `(content_rect, scrollbar_rect)`.
pub fn render_diff_pane(
    area: Rect,
    buf: &mut Buffer,
    lines: &[String],
    scroll: usize,
    focused: bool,
) -> (Rect, Option<Rect>) {
    let content = pane_header(area, buf, "Diff", focused);
    let visible = usize::from(content.height);
    let (rows, overflow) = diff_rows_in(lines, content);
    let total = rows.len();
    let scroll = scroll.min(total.saturating_sub(visible));
    let list_width = if overflow {
        content.width.saturating_sub(1)
    } else {
        content.width
    };
    let shown: Vec<Line<'static>> = rows.into_iter().skip(scroll).take(visible).collect();
    let list_area = Rect {
        width: list_width,
        ..content
    };
    Paragraph::new(shown).render(list_area, buf);
    if !overflow {
        return (content, None);
    }
    render_pane_scrollbar(content, buf, list_width, scroll, visible, total);
    let scrollbar = Rect {
        x: content.x + list_width,
        width: 1,
        ..content
    };
    (content, Some(scrollbar))
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
    fn a_long_line_wraps_and_keeps_its_marker_and_indent_on_the_first_row() {
        let rows = diff_rows(&["+    alpha beta gamma delta".to_string()], 16);
        assert_eq!(texts(&rows), vec!["+    alpha beta", "gamma delta"]);
    }

    #[test]
    fn scroll_counts_wrapped_rows() {
        let lines = vec![format!("+{}", "word ".repeat(20)), "-old".to_string()];
        let content = Rect::new(0, 0, 20, 3);
        let rows = diff_row_count(&lines, Some(content));
        assert!(rows > lines.len());
        assert_eq!(max_diff_scroll(&lines, Some(content)), rows - 3);
        assert_eq!(max_diff_scroll(&lines, None), 1);
    }
}
