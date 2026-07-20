//! 5-row work-unit details strip — Rust port of the TS sub-widgets
//! `WorkUnitTitle` + `WorkUnitDescription` + `WorkUnitAttachments` +
//! `WorkUnitMetadata`.
//!
//! Feature: spec/features/rpc014-board-grid.feature
//! Card: RPC-014.
//!
//! Reads `selected_work_unit()` from a `&BoardStore` snapshot. When the
//! snapshot returns `None`, paints a centered `No work unit selected`
//! placeholder. Otherwise paints the five rows of the strip in order:
//!
//!   row 0: `{id}: {title}` (plain)
//!   rows 1-2: description (cyan, word-wrapped to 2 lines, ellipsis
//!             if it overflows the 2nd line)
//!   row 3: `Attachments (use the "A" key to view): <basename, …>`
//!   row 4: `Epic: {epic} | Estimate: {n}pts | Status: {status}`

use std::path::Path;

use codelet_rpc_types::WorkUnitInfo;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use unicode_width::UnicodeWidthStr;

/// Render the 5-row work-unit details strip into `area`.
///
/// `area` is the inner rectangle BETWEEN the left and right vertical
/// `│` borders — callers paint those border columns separately.
pub fn render(area: Rect, buf: &mut Buffer, selected: Option<&WorkUnitInfo>) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let Some(unit) = selected else {
        render_placeholder(area, buf);
        return;
    };

    let row_y = area.y;
    // Row 0 — title (matches TS WorkUnitTitle: plain <Text>, no color).
    if area.height >= 1 {
        let title_text = format!("{}: {}", unit.id, unit.title);
        let line = Line::from(Span::raw(truncate_to(title_text, area.width as usize)));
        Paragraph::new(line).render(
            Rect {
                x: area.x,
                y: row_y,
                width: area.width,
                height: 1,
            },
            buf,
        );
    }
    // Rows 1-2 — description (TS WorkUnitDescription: `<Text color="cyan"
    // wrap="wrap">` inside a `height={2}` box).
    if area.height >= 2 {
        // Match the TS calculation:
        //   availableWidth = max(10, terminalWidth - 4)
        // where terminalWidth includes the two side borders. Our `area`
        // is already the inner rect (terminalWidth - 2), so to mirror
        // the TS "-4" we drop 2 more cells.
        let avail = (area.width as usize).saturating_sub(2).max(10);
        let avail = avail.min(area.width as usize);
        let desc = unit.description.clone().unwrap_or_default();
        let normalized = normalize(&desc);
        let cyan = Style::default().fg(Color::Cyan);
        let (line1, line2) = wrap_to_two_lines(&normalized, avail);
        Paragraph::new(Line::from(Span::styled(line1, cyan))).render(
            Rect {
                x: area.x,
                y: row_y + 1,
                width: area.width,
                height: 1,
            },
            buf,
        );
        if area.height >= 3 {
            Paragraph::new(Line::from(Span::styled(line2, cyan))).render(
                Rect {
                    x: area.x,
                    y: row_y + 2,
                    width: area.width,
                    height: 1,
                },
                buf,
            );
        }
    }
    // Row 3 — attachments.
    if area.height >= 4 {
        let attachments_line = build_attachments_line(unit, area.width);
        Paragraph::new(attachments_line).render(
            Rect {
                x: area.x,
                y: row_y + 3,
                width: area.width,
                height: 1,
            },
            buf,
        );
    }
    // Row 4 — metadata.
    if area.height >= 5 {
        let meta_line = build_metadata_line(unit, area.width);
        Paragraph::new(meta_line).render(
            Rect {
                x: area.x,
                y: row_y + 4,
                width: area.width,
                height: 1,
            },
            buf,
        );
    }
}

fn render_placeholder(area: Rect, buf: &mut Buffer) {
    let middle_y = area.y + area.height / 2;
    let text = "No work unit selected";
    let text_w = text.width();
    let centered_x = area
        .x
        .saturating_add(area.width.saturating_sub(text_w as u16) / 2);
    let rect = Rect {
        x: centered_x,
        y: middle_y,
        width: text_w as u16,
        height: 1,
    };
    Paragraph::new(Line::from(Span::raw(text))).render(rect, buf);
}

/// COPY-009: the exact border-free on-screen text of the five strip rows
/// for `selected`, reproducing `render`'s truncation/wrap so the selection
/// reader can copy what is shown. Row order: id:title, description line 1,
/// description line 2, attachments, metadata. Rows past real content are
/// empty. Returns the placeholder line when nothing is selected.
pub(super) fn visible_strip_rows(selected: Option<&WorkUnitInfo>, width: u16) -> Vec<String> {
    let w = width as usize;
    let Some(unit) = selected else {
        return vec!["No work unit selected".to_string()];
    };
    let title = truncate_to(format!("{}: {}", unit.id, unit.title), w);
    let avail = (w.saturating_sub(2)).max(10).min(w);
    let normalized = normalize(&unit.description.clone().unwrap_or_default());
    let (line1, line2) = wrap_to_two_lines(&normalized, avail);
    let attachments = line_to_plain(&build_attachments_line(unit, width));
    let metadata = line_to_plain(&build_metadata_line(unit, width));
    vec![title, line1, line2, attachments, metadata]
}

/// Flatten a styled [`Line`] to its plain concatenated span text.
fn line_to_plain(line: &Line<'static>) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

fn normalize(s: &str) -> String {
    let collapsed = s.replace('\n', " ");
    let mut out = String::with_capacity(collapsed.len());
    let mut prev_space = false;
    for ch in collapsed.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

/// Word-wrap `text` greedily into at most 2 lines, each at most
/// `width` user-perceived characters. If the text overflows the
/// second line, append `…` at the end of line 2 (matching the TS
/// `cli-truncate` `position: 'end'` behaviour).
///
/// Returns `(line1, line2)`. `line2` is empty when the whole text
/// fits on `line1`.
fn wrap_to_two_lines(text: &str, width: usize) -> (String, String) {
    if width == 0 {
        return (String::new(), String::new());
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= width {
        return (text.to_string(), String::new());
    }

    // Find a break index for line 1: prefer the rightmost whitespace
    // within `width` chars; fall back to a hard cut at `width`.
    let mut break_at = width;
    for i in (0..=width).rev() {
        if i < chars.len() && chars[i] == ' ' {
            break_at = i;
            break;
        }
    }
    let line1: String = chars[..break_at].iter().collect();

    // Skip the breaking space (if any) before line 2 begins.
    let line2_start = if break_at < chars.len() && chars[break_at] == ' ' {
        break_at + 1
    } else {
        break_at
    };

    let remaining: Vec<char> = chars[line2_start..].to_vec();
    if remaining.len() <= width {
        return (
            line1.trim_end().to_string(),
            remaining.into_iter().collect(),
        );
    }

    // Line 2 overflows — keep `width - 1` chars + an ellipsis.
    let keep = width.saturating_sub(1);
    let mut line2 = String::with_capacity(width);
    for ch in remaining.into_iter().take(keep) {
        line2.push(ch);
    }
    line2.push('…');
    (line1.trim_end().to_string(), line2)
}

fn truncate_to(s: String, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    if s.width() <= max_chars {
        return s;
    }
    let mut out = String::with_capacity(max_chars);
    for ch in s.chars().take(max_chars.saturating_sub(1)) {
        out.push(ch);
    }
    out.push('…');
    out
}

fn build_attachments_line(unit: &WorkUnitInfo, width: u16) -> Line<'static> {
    let dim = Style::default().fg(Color::DarkGray);
    let prefix = if unit.attachments.is_empty() {
        "Attachments: "
    } else {
        "Attachments (use the \"A\" key to view): "
    };
    if unit.attachments.is_empty() {
        return Line::from(vec![
            Span::styled(prefix.to_string(), dim),
            Span::styled("none".to_string(), dim),
        ]);
    }
    let basenames: Vec<String> = unit
        .attachments
        .iter()
        .map(|p| {
            Path::new(p)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| p.clone())
        })
        .collect();
    let joined = basenames.join(", ");
    let max_chars = width.saturating_sub(prefix.width() as u16) as usize;
    let display = truncate_to(joined, max_chars);
    Line::from(vec![
        Span::styled(prefix.to_string(), dim),
        Span::raw(display),
    ])
}

fn build_metadata_line(unit: &WorkUnitInfo, width: u16) -> Line<'static> {
    let mut fields: Vec<String> = Vec::new();
    if let Some(epic) = unit.epic.as_deref() {
        fields.push(format!("Epic: {epic}"));
    }
    if let Some(estimate) = unit.estimate {
        fields.push(format!("Estimate: {estimate}pts"));
    }
    // Status is always present in WorkUnitInfo.
    fields.push(format!("Status: {}", unit.status));
    let joined = fields.join(" | ");
    let truncated = truncate_to(joined, width as usize);
    Line::from(Span::raw(truncated))
}

#[cfg(test)]
#[path = "details_strip_tests.rs"]
mod tests;
