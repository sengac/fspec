//! TUI-121 — Priority::Foreground form for adding or editing a saved daemon.
//!
//! Feature: spec/features/manage-saved-daemons-from-the-tui.feature
//! Feature: spec/features/daemon-management-on-par-with-workspaces.feature
//!
//! Three single-line fields — name, URL and token. Tab / Shift+Tab move
//! between them, Enter emits `Action::SaveDaemon`, Esc cancels. The token is
//! never displayed (each character paints as `•`); when editing it starts
//! empty and an empty token keeps the saved one. TUI-123: a refusal is shown
//! under the field it concerns, and a value longer than its field scrolls
//! (the part around the cursor is shown, cut with `…`), so the form stays
//! usable at the board's 64-column minimum.

use crossterm::event::{Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;
use tui_textarea::{Input, TextArea};

use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog, FOOTER_SEPARATOR};
use super::{Action, Callback, Component, EventResult, Priority};
use crate::daemons::{DaemonField, DaemonForm, DaemonFormError, TokenChange};

/// Compositor id of the daemon form.
pub const DAEMON_FORM_DIALOG_ID: &str = "daemon-form-dialog";

const NAME: usize = 0;
const URL: usize = 1;
const TOKEN: usize = 2;
/// Content width of the form when the terminal allows it.
const FORM_WIDTH: u16 = 70;
/// Cells before a field's value: the focus marker and the padded label.
const LABEL_CELLS: usize = 9;

fn field(text: &str) -> TextArea<'static> {
    let mut area = TextArea::new(vec![text.to_owned()]);
    area.set_cursor_line_style(Style::default());
    area.move_cursor(tui_textarea::CursorMove::End);
    area
}

/// TUI-123: the part of `chars` around `cursor` that fits `width` cells,
/// with `…` where it is cut.
pub(crate) fn scroll_window(chars: &[char], cursor: usize, width: usize) -> String {
    if chars.len() <= width {
        return chars.iter().collect();
    }
    let inner = width.saturating_sub(2).max(1);
    let start = cursor
        .saturating_sub(inner.saturating_sub(1))
        .min(chars.len().saturating_sub(inner));
    let end = (start + inner).min(chars.len());
    let mut shown = String::new();
    if start > 0 {
        shown.push('…');
    }
    shown.extend(&chars[start..end]);
    if end < chars.len() {
        shown.push('…');
    }
    shown
}

/// TUI-123: `text` word-wrapped to `width` characters (long words split).
pub(crate) fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let mut word: Vec<char> = word.chars().collect();
        while word.len() > width {
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            lines.push(word.drain(..width).collect());
        }
        let used = line.chars().count();
        if used > 0 && used + 1 + word.len() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.extend(word);
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

/// The add / edit daemon form.
pub struct DaemonFormDialog {
    /// The saved daemon being edited (`None` when adding).
    previous: Option<String>,
    fields: [TextArea<'static>; 3],
    focus: usize,
    error: Option<DaemonFormError>,
    action_tx: UnboundedSender<Action>,
}

impl DaemonFormDialog {
    /// A form for `previous` (an edit) or a new daemon, seeded with `form`
    /// and, after a refusal, its `error`.
    pub fn new(
        previous: Option<String>,
        form: &DaemonForm,
        error: Option<DaemonFormError>,
        action_tx: UnboundedSender<Action>,
    ) -> Self {
        let token = match &form.token {
            TokenChange::Set(token) => token.as_str(),
            TokenChange::Keep => "",
        };
        let focus = match error.as_ref().and_then(|error| error.field) {
            Some(DaemonField::Url) => URL,
            Some(DaemonField::Token) => TOKEN,
            Some(DaemonField::Name) | None => NAME,
        };
        Self {
            previous,
            fields: [field(&form.name), field(&form.url), field(token)],
            focus,
            error,
            action_tx,
        }
    }

    fn text(&self, index: usize) -> String {
        self.fields[index].lines().join("")
    }

    fn close() -> Callback {
        Box::new(|compositor| {
            let _ = compositor.remove(DAEMON_FORM_DIALOG_ID);
        })
    }

    fn submit(&self) -> EventResult {
        let token = self.text(TOKEN);
        let form = DaemonForm {
            name: self.text(NAME).trim().to_owned(),
            url: self.text(URL).trim().to_owned(),
            token: if token.is_empty() {
                TokenChange::Keep
            } else {
                TokenChange::Set(token)
            },
        };
        let action = Action::SaveDaemon {
            previous: self.previous.clone(),
            form,
        };
        if self.action_tx.send(action).is_err() {
            tracing::error!("daemon form: action bus closed; the daemon was not saved");
        }
        EventResult::Consumed(Some(Self::close()))
    }

    /// The field's value as shown in `width` cells: scrolled to the
    /// cursor, the token masked.
    fn shown_value(&self, index: usize, width: usize) -> String {
        let text = self.text(index);
        if index == TOKEN && text.is_empty() && self.previous.is_some() {
            return scroll_window(
                &"(empty keeps the saved token)".chars().collect::<Vec<_>>(),
                0,
                width,
            );
        }
        let chars: Vec<char> = if index == TOKEN {
            text.chars().map(|_| '•').collect()
        } else {
            text.chars().collect()
        };
        let cursor = self.fields[index].cursor().1;
        scroll_window(&chars, cursor, width)
    }

    fn row(&self, index: usize, label: &str, width: usize) -> DialogRow {
        let marker = if index == self.focus { "› " } else { "  " };
        let style = if index == self.focus {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let value = self.shown_value(index, width.saturating_sub(LABEL_CELLS));
        DialogRow {
            spans: vec![Span::styled(format!("{marker}{label:<7}{value}"), style)],
            selectable: false,
            selected: false,
        }
    }

    fn error_rows(&self, field: Option<DaemonField>, width: usize) -> Vec<DialogRow> {
        let Some(error) = self.error.as_ref().filter(|error| error.field == field) else {
            return Vec::new();
        };
        let indent = if field.is_some() { LABEL_CELLS } else { 0 };
        wrap_text(&error.message, width.saturating_sub(indent))
            .into_iter()
            .map(|line| DialogRow {
                spans: vec![Span::styled(
                    format!("{:indent$}{line}", ""),
                    Style::default().fg(Color::Red),
                )],
                selectable: false,
                selected: false,
            })
            .collect()
    }
}

impl Component for DaemonFormDialog {
    fn priority(&self) -> Priority {
        Priority::Foreground
    }

    fn id(&self) -> &str {
        DAEMON_FORM_DIALOG_ID
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Esc => EventResult::Consumed(Some(Self::close())),
                KeyCode::Enter => self.submit(),
                KeyCode::Tab => {
                    self.focus = (self.focus + 1) % 3;
                    EventResult::consumed()
                }
                KeyCode::BackTab => {
                    self.focus = (self.focus + 2) % 3;
                    EventResult::consumed()
                }
                _ => {
                    let input =
                        Input::from(crossterm::event::KeyEvent::new(key.code, key.modifiers));
                    let _ = self.fields[self.focus].input(input);
                    EventResult::consumed()
                }
            },
            Event::Paste(text) => {
                let _ = self.fields[self.focus].insert_str(text.replace(['\r', '\n'], ""));
                EventResult::consumed()
            }
            _ => EventResult::ignored(),
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // 2 border + 2 padding cells around the content.
        let width = usize::from(area.width.min(FORM_WIDTH + 4).saturating_sub(4));
        let mut rows = Vec::new();
        for (index, label, field) in [
            (NAME, "Name", DaemonField::Name),
            (URL, "URL", DaemonField::Url),
            (TOKEN, "Token", DaemonField::Token),
        ] {
            rows.push(self.row(index, label, width));
            rows.extend(self.error_rows(Some(field), width));
        }
        rows.extend(self.error_rows(None, width));
        let sep = FOOTER_SEPARATOR;
        let footer = format!("Tab Next field{sep}Enter Save{sep}Esc Cancel");
        let title = if self.previous.is_some() {
            "Edit saved daemon"
        } else {
            "Add saved daemon"
        };
        render_dialog(
            area,
            buf,
            &FspecDialog {
                accent: Accent::Cyan,
                title,
                rows,
                footer: footer.as_str(),
                min_width: FORM_WIDTH,
                query_row: None,
            },
        );
    }
}
