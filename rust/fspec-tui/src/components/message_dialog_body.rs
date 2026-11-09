//! TUI-131 — the wrapped, scrollable message body shared by the
//! `ErrorDialog` and the `NotificationDialog`.
//!
//! Feature: spec/features/long-text-wraps-instead-of-being-cut-off.feature
//!
//! The message wraps to the dialog width ([`wrapped_rows`]), so a line
//! break starts a row and nothing is cut at the edge. While the dialog
//! fits the area it is the usual shrink-to-content dialog. When its rows
//! do not fit, it fills the area the `SessionStorageDialog` way
//! ([`fill_rect`], [`body_content_rows`], a scrollbar in the gutter): the
//! body scrolls with Up/Down/PgUp/PgDn/Home/End and the wheel, and the
//! footer stays on screen.

use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use super::dialog_theme::{render_dialog, render_dialog_at, Accent};
use super::dialog_theme_rows::{body_content_rows, build_dialog, dialog_body_width, wrapped_rows};
use super::help_dialog_scroll::{fill_rect, gutter_rect};
use super::list_scrollbar::render_list_scrollbar;

/// Rows the dialog takes besides its body rows and footer lines: border
/// (2), padding (2), title (1), the gap after the title (1) and the gap
/// before the footer (1).
const CHROME_ROWS: usize = 7;

/// How the message dialog looks: everything but the message.
pub(crate) struct MessageLook<'a> {
    pub accent: Accent,
    pub title: &'a str,
    pub style: Style,
    pub footer: &'a str,
    pub min_width: u16,
}

/// The scroll position of a message body, sized at the last render.
#[derive(Debug, Default)]
pub(crate) struct MessageBody {
    offset: usize,
    visible: usize,
    total: usize,
}

impl MessageBody {
    fn scroll(&mut self, delta: isize) {
        self.offset = self
            .offset
            .saturating_add_signed(delta)
            .min(self.total.saturating_sub(self.visible));
    }

    /// Scroll for a scroll key or a wheel event; `true` when `event` was
    /// one (the dialog consumes it).
    pub(crate) fn handle_scroll(&mut self, event: &Event) -> bool {
        let page = self.visible.max(1) as isize;
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Up => self.scroll(-1),
                KeyCode::Down => self.scroll(1),
                KeyCode::PageUp => self.scroll(-page),
                KeyCode::PageDown => self.scroll(page),
                KeyCode::Home => self.offset = 0,
                KeyCode::End => self.offset = self.total.saturating_sub(self.visible),
                _ => return false,
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => self.scroll(-3),
                MouseEventKind::ScrollDown => self.scroll(3),
                _ => return false,
            },
            _ => return false,
        }
        true
    }

    /// Paint `message` in a dialog inside `area`: shrink-to-content while
    /// it fits, otherwise filling the area with the body scrolled to the
    /// current offset.
    pub(crate) fn render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        message: &str,
        look: &MessageLook,
    ) {
        let footer_rows = look.footer.lines().count().max(1);
        let rows = wrapped_rows(message, look.style, dialog_body_width(area));
        if rows.len() + CHROME_ROWS + footer_rows <= usize::from(area.height) {
            self.offset = 0;
            self.total = rows.len();
            self.visible = rows.len();
            let dialog = build_dialog(look.accent, look.title, rows, look.footer, look.min_width);
            render_dialog(area, buf, &dialog);
            return;
        }
        let rect = fill_rect(area);
        self.visible = body_content_rows(rect.height, footer_rows as u16, false).max(1);
        // The body less the one-column scrollbar gutter.
        let width = usize::from(rect.width.saturating_sub(5)).max(1);
        let rows = wrapped_rows(message, look.style, width);
        self.total = rows.len();
        self.offset = self.offset.min(self.total.saturating_sub(self.visible));
        let window = rows
            .into_iter()
            .skip(self.offset)
            .take(self.visible)
            .collect();
        let dialog = build_dialog(look.accent, look.title, window, look.footer, look.min_width);
        render_dialog_at(rect, buf, &dialog);
        if self.total > self.visible {
            if let Some(gutter) = gutter_rect(rect, self.visible) {
                render_list_scrollbar(gutter, buf, self.offset, self.visible, self.total);
            }
        }
    }
}
