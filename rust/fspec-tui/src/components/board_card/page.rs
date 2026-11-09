//! BOARD-024 — one canonical details page: wrapped rows, scroll, keyboard
//! focus and the hitboxes of the latest render.
//!
//! Scrolling (arrows, pages, endpoints, wheel) clamps against the latest
//! rendered geometry. Tab/Shift-Tab cycle the selectable links and
//! attachments and reveal the focused row; a terminal resize also reveals
//! it, while the focus index (the activation target) never changes with
//! geometry. Mouse clicks only hit rows painted by the latest render.

use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use serde_json::Value;
use unicode_width::UnicodeWidthStr;

pub(super) use super::document::Target;
use super::document::{project, Document, LineKind};
use super::wrap::wrap;

struct Row {
    indent: u16,
    text: String,
    kind: LineKind,
}

pub(super) struct Page {
    pub(super) id: String,
    pub(super) title: String,
    doc: Document,
    scroll: usize,
    focus: Option<usize>,
    reveal: bool,
    size: Option<(u16, u16)>,
    viewport: usize,
    total: usize,
    hitboxes: Vec<(Rect, usize)>,
    rows: Option<(u16, Vec<Row>)>,
}

impl Page {
    /// Parse a `{id, workspace, workUnit}` envelope into a page.
    pub(super) fn from_envelope(id: &str, payload: &str) -> Result<Self, String> {
        let envelope: Value = serde_json::from_str(payload)
            .map_err(|error| format!("invalid details JSON: {error}"))?;
        let record = envelope
            .get("workUnit")
            .and_then(Value::as_object)
            .ok_or_else(|| "invalid details envelope: workUnit is not an object".to_owned())?;
        let title = record
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        Ok(Self {
            id: id.to_owned(),
            title,
            doc: project(record),
            scroll: 0,
            focus: None,
            reveal: false,
            size: None,
            viewport: 1,
            total: 0,
            hitboxes: Vec::new(),
            rows: None,
        })
    }

    /// TUI-130: the page's whole text for a copy: `id: title`, then every
    /// logical line (unwrapped, rows below the fold included).
    pub(super) fn plain_text(&self) -> String {
        let mut text = format!("{}: {}", self.id, self.title);
        for line in &self.doc.lines {
            text.push('\n');
            text.push_str(&" ".repeat(usize::from(line.indent)));
            text.push_str(&line.text);
        }
        text
    }

    fn max_scroll(&self) -> usize {
        self.total.saturating_sub(self.viewport)
    }

    pub(super) fn scroll_by(&mut self, delta: isize) {
        let next = self.scroll as isize + delta;
        self.scroll = next.clamp(0, self.max_scroll() as isize) as usize;
    }

    fn cycle(&mut self, forward: bool) {
        let count = self.doc.targets.len();
        if count == 0 {
            return;
        }
        self.focus = Some(match (self.focus, forward) {
            (None, true) => 0,
            (None, false) => count - 1,
            (Some(index), true) => (index + 1) % count,
            (Some(index), false) => (index + count - 1) % count,
        });
        self.reveal = true;
    }

    /// Handle a details key; returns a target to activate on Enter.
    pub(super) fn key(&mut self, code: KeyCode) -> Option<Target> {
        let page = self.viewport.max(1) as isize;
        match code {
            KeyCode::Up => self.scroll_by(-1),
            KeyCode::Down => self.scroll_by(1),
            KeyCode::PageUp => self.scroll_by(-page),
            KeyCode::PageDown => self.scroll_by(page),
            KeyCode::Home => self.scroll = 0,
            KeyCode::End => self.scroll = self.max_scroll(),
            KeyCode::Tab => self.cycle(true),
            KeyCode::BackTab => self.cycle(false),
            KeyCode::Enter => {
                return self
                    .focus
                    .and_then(|index| self.doc.targets.get(index).cloned());
            }
            _ => {}
        }
        None
    }

    /// Activate the target painted at `(column, row)` by the latest render.
    pub(super) fn click(&mut self, column: u16, row: u16) -> Option<Target> {
        let index = self
            .hitboxes
            .iter()
            .find(|(rect, _)| rect.contains((column, row).into()))
            .map(|(_, index)| *index)?;
        self.focus = Some(index);
        self.doc.targets.get(index).cloned()
    }

    fn wrapped(&self, width: u16) -> Vec<Row> {
        let mut rows = Vec::new();
        for line in &self.doc.lines {
            let available = usize::from(width.saturating_sub(line.indent)).max(1);
            let item = matches!(line.kind, LineKind::Item(_));
            let wrap_width = if item {
                available.saturating_sub(2).max(1)
            } else {
                available
            };
            for (index, text) in wrap(&line.text, wrap_width).into_iter().enumerate() {
                let continuation = item && index > 0;
                rows.push(Row {
                    indent: line.indent + if continuation { 2 } else { 0 },
                    text,
                    kind: line.kind,
                });
            }
        }
        rows
    }

    fn reveal_focus(&mut self, rows: &[Row]) {
        let Some(focus) = self.focus else {
            return;
        };
        let matching = |row: &Row| row.kind == LineKind::Item(focus);
        let (Some(first), Some(last)) = (
            rows.iter().position(matching),
            rows.iter().rposition(matching),
        ) else {
            return;
        };
        if first < self.scroll || last - first + 1 > self.viewport {
            self.scroll = first;
        } else if last >= self.scroll + self.viewport {
            self.scroll = last + 1 - self.viewport;
        }
    }

    /// Paint the scrolled content into `area`, recording geometry/hitboxes.
    pub(super) fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let rows = match self.rows.take() {
            Some((width, rows)) if width == area.width => rows,
            _ => self.wrapped(area.width),
        };
        if self.size != Some((area.width, area.height)) {
            self.reveal |= self.size.is_some();
            self.size = Some((area.width, area.height));
        }
        self.viewport = usize::from(area.height).max(1);
        self.total = rows.len();
        if self.reveal {
            self.reveal_focus(&rows);
            self.reveal = false;
        }
        self.scroll = self.scroll.min(self.max_scroll());
        self.hitboxes.clear();
        let visible = rows.iter().skip(self.scroll).take(usize::from(area.height));
        for (offset, row) in visible.enumerate() {
            let y = area.y + offset as u16;
            let x = area.x + row.indent.min(area.width);
            let room = usize::from(area.width.saturating_sub(row.indent));
            let style = match row.kind {
                LineKind::Heading => Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                LineKind::Item(index) if self.focus == Some(index) => {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                }
                LineKind::Item(_) => Style::default().fg(Color::Cyan),
                LineKind::Text => Style::default(),
            };
            buf.set_stringn(x, y, &row.text, room, style);
            if let LineKind::Item(index) = row.kind {
                let width = (row.text.width() as u16).min(area.width.saturating_sub(row.indent));
                self.hitboxes.push((Rect::new(x, y, width, 1), index));
            }
        }
        self.rows = Some((area.width, rows));
    }
}
