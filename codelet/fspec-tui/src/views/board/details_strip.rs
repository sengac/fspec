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
//!   row 0: `{id}: {title}` (cyan + bold)
//!   row 1: first line of description, truncated to `width - 4`
//!   row 2: `Attachments (use the "A" key to view): <basename, …>`
//!   row 3: `Epic: {epic} | Estimate: {n}pts | Status: {status}`
//!   row 4: blank padding

use std::path::Path;

use codelet_rpc_types::WorkUnitInfo;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

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
    // Row 0 — title (cyan + bold).
    if area.height >= 1 {
        let title_text = format!("{}: {}", unit.id, unit.title);
        let line = Line::from(Span::styled(
            truncate_to(title_text, area.width as usize),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));
        Paragraph::new(line).render(Rect { x: area.x, y: row_y, width: area.width, height: 1 }, buf);
    }
    // Row 1 — description (single-line, truncated).
    if area.height >= 2 {
        let desc = unit.description.clone().unwrap_or_default();
        let normalized = normalize(&desc);
        let max_chars = area.width.saturating_sub(2) as usize;
        let truncated = truncate_to(normalized, max_chars);
        Paragraph::new(Line::from(Span::raw(truncated))).render(
            Rect { x: area.x, y: row_y + 1, width: area.width, height: 1 },
            buf,
        );
    }
    // Row 2 — attachments.
    if area.height >= 3 {
        let attachments_line = build_attachments_line(unit, area.width);
        Paragraph::new(attachments_line).render(
            Rect { x: area.x, y: row_y + 2, width: area.width, height: 1 },
            buf,
        );
    }
    // Row 3 — metadata.
    if area.height >= 4 {
        let meta_line = build_metadata_line(unit, area.width);
        Paragraph::new(meta_line).render(
            Rect { x: area.x, y: row_y + 3, width: area.width, height: 1 },
            buf,
        );
    }
    // Row 4 — intentional blank padding (no work required).
}

fn render_placeholder(area: Rect, buf: &mut Buffer) {
    let middle_y = area.y + area.height / 2;
    let text = "No work unit selected";
    let centered_x = area
        .x
        .saturating_add(area.width.saturating_sub(text.chars().count() as u16) / 2);
    let rect = Rect {
        x: centered_x,
        y: middle_y,
        width: text.chars().count() as u16,
        height: 1,
    };
    Paragraph::new(Line::from(Span::raw(text))).render(rect, buf);
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

fn truncate_to(s: String, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    if s.chars().count() <= max_chars {
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
    let max_chars = width.saturating_sub(prefix.chars().count() as u16) as usize;
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
