//! RPC-028 — render helpers for ResumeSessionView and
//! SearchHistoryView. Extracted to keep the parent files under the
//! 300-LoC source-shape budget.
//!
//! RPC-054 (revision): `render_title_with_count` and
//! `render_footer_hint` are promoted to `pub(crate)` so the
//! ProviderSettingsView can reuse them. The title helper takes a
//! `suffix` parameter so different views can label the count
//! correctly (sessions: "available", providers: "configured").

use codelet_rpc_types::SessionInfo;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

pub(crate) fn render_title_with_count(
    area: Rect,
    buf: &mut Buffer,
    title: &str,
    count: usize,
    suffix: &str,
) {
    let text = format!("{title} ({count} {suffix})");
    let style = Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::BOLD);
    Paragraph::new(Line::from(Span::styled(text, style))).render(area, buf);
}

/// RPC-350 R1 — provider-specific two-span title: the name segment in
/// **bold yellow** and the ` ({count} {suffix})` segment in dim gray
/// (`Color::DarkGray`). Mirrors `ProviderSettingsPanel.tsx:550-555`
/// (`<Text bold color="yellow">…</Text><Text dimColor> (N items)</Text>`).
///
/// This is intentionally SEPARATE from [`render_title_with_count`] so the
/// shared blue-bold title used by ResumeSession / SearchHistory / model
/// views is never affected (RPC-350 R5 guard). Wired into the provider
/// view via `render_full_screen_scaffold_with_title`.
pub(crate) fn render_two_span_title(
    area: Rect,
    buf: &mut Buffer,
    name: &str,
    count: usize,
    suffix: &str,
) {
    let name_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let count_style = Style::default().fg(Color::DarkGray);
    let line = Line::from(vec![
        Span::styled(name.to_string(), name_style),
        Span::styled(format!(" ({count} {suffix})"), count_style),
    ]);
    Paragraph::new(line).render(area, buf);
}

pub(crate) fn render_footer_hint(area: Rect, buf: &mut Buffer, text: &str) {
    Paragraph::new(text.to_string()).render(area, buf);
}

pub(super) fn render_session_rows(
    area: Rect,
    buf: &mut Buffer,
    sessions: &[SessionInfo],
    selected_index: usize,
    scroll_offset: usize,
) {
    if sessions.is_empty() {
        let mid_y = area.y.saturating_add(area.height / 2);
        let row = Rect {
            x: area.x,
            y: mid_y,
            width: area.width,
            height: 1,
        };
        Paragraph::new("(no sessions to resume)")
            .alignment(Alignment::Center)
            .render(row, buf);
        return;
    }
    let visible_rows = area.height as usize;
    if visible_rows == 0 {
        return;
    }
    let end = (scroll_offset + visible_rows).min(sessions.len());
    for (row_idx, info) in sessions[scroll_offset..end].iter().enumerate() {
        let global_idx = scroll_offset + row_idx;
        let marker = if global_idx == selected_index {
            "▸"
        } else {
            " "
        };
        let label = format!(" {marker} {} ({})", info.id, info.status);
        let style = if global_idx == selected_index {
            Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::default()
        };
        let y = area.y + row_idx as u16;
        let row_area = Rect {
            x: area.x,
            y,
            width: area.width,
            height: 1,
        };
        Paragraph::new(Line::from(Span::styled(label, style))).render(row_area, buf);
    }
}
