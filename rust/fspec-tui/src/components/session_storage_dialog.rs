use codelet_rpc_types::SessionManifestIssue;
use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::{buffer::Buffer, layout::Rect, text::Span};
use tokio::sync::mpsc::UnboundedSender;

use super::dialog_theme::{render_dialog_at, Accent, DialogRow};
use super::dialog_theme_rows::{body_content_rows, build_dialog};
use super::help_dialog_scroll::{fill_rect, gutter_rect};
use super::list_scrollbar::render_list_scrollbar;
use super::{Action, Component, EventResult, Priority};
use crate::terminal::sanitize::sanitize_for_terminal;
use crate::views::agent::text_wrap::wrap_to_width;

pub const SESSION_STORAGE_DIALOG_ID: &str = "session-storage-dialog";

pub struct SessionStorageDialog {
    message: String,
    has_errors: bool,
    offset: usize,
    visible: usize,
    total: usize,
    sender: UnboundedSender<Action>,
}

impl SessionStorageDialog {
    pub fn new(
        issues: &[SessionManifestIssue],
        errors: &[String],
        sender: UnboundedSender<Action>,
    ) -> Self {
        let mut sections = errors.to_vec();
        sections.extend(issues.iter().map(|issue| {
            format!(
                "Path: {}\nSession: {}\nCategory: {}\nCause: {}\nRecovery: {}",
                issue.path.display(),
                issue.session_id.as_deref().unwrap_or("unknown"),
                issue.category,
                issue.reason,
                issue.recovery,
            )
        }));
        Self {
            message: sanitize_for_terminal(&sections.join("\n\n")),
            has_errors: !errors.is_empty(),
            offset: 0,
            visible: 1,
            total: 0,
            sender,
        }
    }

    fn scroll(&mut self, delta: isize) {
        self.offset = self
            .offset
            .saturating_add_signed(delta)
            .min(self.total.saturating_sub(self.visible));
    }
}

impl Component for SessionStorageDialog {
    fn priority(&self) -> Priority {
        Priority::Critical
    }

    fn id(&self) -> &str {
        SESSION_STORAGE_DIALOG_ID
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Esc => {
                    let sender = self.sender.clone();
                    return EventResult::Consumed(Some(Box::new(move |compositor| {
                        let _ = compositor.remove(SESSION_STORAGE_DIALOG_ID);
                        let _ = sender.send(Action::SessionStorageDismissed);
                    })));
                }
                KeyCode::Up => self.scroll(-1),
                KeyCode::Down => self.scroll(1),
                KeyCode::PageUp => self.scroll(-(self.visible as isize)),
                KeyCode::PageDown => self.scroll(self.visible as isize),
                KeyCode::Home => self.offset = 0,
                KeyCode::End => self.offset = self.total.saturating_sub(self.visible),
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => self.scroll(-3),
                MouseEventKind::ScrollDown => self.scroll(3),
                _ => {}
            },
            _ => {}
        }
        EventResult::consumed()
    }

    fn render(&mut self, area: Rect, buffer: &mut Buffer) {
        let rect = fill_rect(area);
        self.visible = body_content_rows(rect.height, 1, false).max(1);
        let width = rect.width.saturating_sub(5).max(1) as usize;
        let lines = wrap_to_width(&self.message, width);
        self.total = lines.len();
        self.offset = self.offset.min(self.total.saturating_sub(self.visible));
        let rows = lines
            .iter()
            .skip(self.offset)
            .take(self.visible)
            .map(|line| DialogRow {
                spans: vec![Span::raw(line.clone())],
                selectable: false,
                selected: false,
            })
            .collect();
        let (accent, title) = if self.has_errors {
            (Accent::Red, "Session error")
        } else {
            (Accent::Yellow, "Session history warnings")
        };
        let dialog = build_dialog(accent, title, rows, "↑/↓ PgUp/PgDn · ESC to dismiss", 30);
        render_dialog_at(rect, buffer, &dialog);
        if self.total > self.visible {
            if let Some(gutter) = gutter_rect(rect, self.visible) {
                render_list_scrollbar(gutter, buffer, self.offset, self.visible, self.total);
            }
        }
    }
}
