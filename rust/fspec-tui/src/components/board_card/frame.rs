//! BOARD-024 — full-screen details frame: titled border, scrolled page,
//! the status rows (loading / visible errors) and the controls hint.
//!
//! TUI-131 (spec/features/long-text-wraps-instead-of-being-cut-off.feature):
//! the status and the hint wrap to the frame width; the page gets the rest.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, Paragraph, Widget};

use super::sanitize::sanitize_line;
use super::{BoardCardDialog, Status};
use crate::views::agent::text_wrap::wrapped_lines;

const FOOTER: &str =
    "Tab/Shift-Tab select link or attachment │ Enter open │ Esc back │ ↑↓ PgUp PgDn Home End wheel scroll │ c copy │ y copy id";

fn rows_u16(lines: &[Line<'_>]) -> u16 {
    u16::try_from(lines.len()).unwrap_or(u16::MAX)
}

impl BoardCardDialog {
    pub(super) fn render_details(&mut self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let heading = match self.pages.last() {
            Some(page) if page.title.is_empty() => sanitize_line(&page.id),
            Some(page) => format!(
                "{} · {}",
                sanitize_line(&page.id),
                sanitize_line(&page.title)
            ),
            None => sanitize_line(&self.card_id),
        };
        let block = Block::bordered()
            .border_style(Style::default().fg(Color::Yellow))
            .title(Line::from(format!(" {heading} ")));
        let inner = block.inner(area);
        block.render(area, buf);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        // TUI-131: the status row and the footer wrap (`text_wrap`) onto
        // the rows they need; the page gets the rows left (it re-wraps
        // itself when its size changes). The status keeps one row, blank
        // while idle, so the page does not jump when a status appears.
        let width = usize::from(inner.width);
        let (text, style) = match &self.status {
            Some(Status::Loading(id)) => (
                format!("Loading {} details…", sanitize_line(id)),
                Style::default().fg(Color::Gray),
            ),
            Some(Status::Error(message)) => {
                (sanitize_line(message), Style::default().fg(Color::Red))
            }
            None => (String::new(), Style::default()),
        };
        let footer = wrapped_lines(FOOTER, width, Style::default().fg(Color::DarkGray));
        let status = wrapped_lines(&text, width, style);
        let footer_rows = rows_u16(&footer).min(inner.height);
        let status_rows = rows_u16(&status).min(inner.height - footer_rows);
        let content_rows = inner.height - footer_rows - status_rows;
        let content = Rect::new(inner.x, inner.y, inner.width, content_rows);
        if let Some(page) = self.pages.last_mut() {
            if content.height > 0 {
                page.render(content, buf);
            }
        }
        let status_area = Rect::new(inner.x, inner.y + content_rows, inner.width, status_rows);
        Paragraph::new(status).render(status_area, buf);
        let footer_y = status_area.y + status_rows;
        let footer_area = Rect::new(inner.x, footer_y, inner.width, footer_rows);
        Paragraph::new(footer).render(footer_area, buf);
    }
}
