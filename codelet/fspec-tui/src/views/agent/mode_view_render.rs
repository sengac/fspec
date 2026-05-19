//! RPC-028 — render helpers for ResumeSessionView and
//! SearchHistoryView. Extracted to keep the parent files under the
//! 300-LoC source-shape budget.

use codelet_rpc_types::SessionInfo;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

pub(super) fn render_title_with_count(area: Rect, buf: &mut Buffer, title: &str, count: usize) {
    let text = format!("{title} ({count} available)");
    let style = Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::BOLD);
    Paragraph::new(Line::from(Span::styled(text, style))).render(area, buf);
}

pub(super) fn render_footer_hint(area: Rect, buf: &mut Buffer, text: &'static str) {
    Paragraph::new(text).render(area, buf);
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
        let row = Rect { x: area.x, y: mid_y, width: area.width, height: 1 };
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
        let marker = if global_idx == selected_index { "▸" } else { " " };
        let label = format!(" {marker} {} ({})", info.id, info.status);
        let style = if global_idx == selected_index {
            Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::default()
        };
        let y = area.y + row_idx as u16;
        let row_area = Rect { x: area.x, y, width: area.width, height: 1 };
        Paragraph::new(Line::from(Span::styled(label, style))).render(row_area, buf);
    }
}
