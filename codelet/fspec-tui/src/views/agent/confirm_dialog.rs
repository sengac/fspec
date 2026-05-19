//! RPC-026 — ConfirmDialog: centred confirmation overlay.
//!
//! Feature: spec/features/rpc026-resume-and-search-mode-views.feature
//! Feature: spec/features/rpc027-model-confirm-dialogs.feature
//!
//! RPC-027 update: renders via the shared dialog_theme renderer
//! (rounded yellow border, bold yellow inner title, opaque black
//! background, inverse highlight on the focused button).

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use crate::components::dialog_theme::{
    render_dialog, Accent, DialogRow, FspecDialog, FOOTER_SEPARATOR,
};

/// Outcome of routing a single key event through the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmDialogOutcome {
    /// User activated the Primary button (index 0).
    Primary,
    /// User activated the Secondary button (index 1) — only emitted
    /// when a non-empty secondary label was supplied at construction.
    Secondary,
    /// User activated the Cancel button or pressed Esc.
    Cancel,
    /// Dialog handled the key internally (navigation).
    Continued,
    /// Dialog ignored the key — caller may route it elsewhere.
    Ignored,
}

/// A simple confirmation overlay with up to three buttons.
pub struct ConfirmDialog {
    title: String,
    body: String,
    buttons: Vec<String>,
    focused: usize,
}

impl ConfirmDialog {
    /// Construct a fresh dialog. `secondary_label` may be `None` to
    /// produce a two-button dialog (Primary + Cancel); when `Some` the
    /// dialog has three buttons in left-to-right order.
    ///
    /// Focus starts on the Primary button (index 0).
    pub fn new(
        title: impl Into<String>,
        body: impl Into<String>,
        primary_label: impl Into<String>,
        secondary_label: Option<String>,
        cancel_label: impl Into<String>,
    ) -> Self {
        let mut buttons = vec![primary_label.into()];
        if let Some(sec) = secondary_label {
            buttons.push(sec);
        }
        buttons.push(cancel_label.into());
        Self {
            title: title.into(),
            body: body.into(),
            buttons,
            focused: 0,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn buttons(&self) -> &[String] {
        &self.buttons
    }

    pub fn focused(&self) -> usize {
        self.focused
    }

    pub fn primary_label(&self) -> &str {
        self.buttons.first().map(String::as_str).unwrap_or("")
    }

    pub fn secondary_label(&self) -> Option<&str> {
        if self.buttons.len() >= 3 {
            self.buttons.get(1).map(String::as_str)
        } else {
            None
        }
    }

    pub fn cancel_label(&self) -> &str {
        self.buttons.last().map(String::as_str).unwrap_or("Cancel")
    }

    fn cancel_index(&self) -> usize {
        self.buttons.len().saturating_sub(1)
    }

    fn outcome_for_index(&self, idx: usize) -> ConfirmDialogOutcome {
        if idx == 0 {
            ConfirmDialogOutcome::Primary
        } else if idx == self.cancel_index() {
            ConfirmDialogOutcome::Cancel
        } else {
            ConfirmDialogOutcome::Secondary
        }
    }

    fn focus_prev(&mut self) {
        if self.buttons.is_empty() {
            return;
        }
        if self.focused == 0 {
            self.focused = self.buttons.len() - 1;
        } else {
            self.focused -= 1;
        }
    }

    fn focus_next(&mut self) {
        if self.buttons.is_empty() {
            return;
        }
        if self.focused + 1 >= self.buttons.len() {
            self.focused = 0;
        } else {
            self.focused += 1;
        }
    }

    /// Route a single key event through the dialog.
    pub fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> ConfirmDialogOutcome {
        if mods.contains(KeyModifiers::CONTROL) || mods.contains(KeyModifiers::ALT) {
            return ConfirmDialogOutcome::Ignored;
        }
        match code {
            KeyCode::Esc => ConfirmDialogOutcome::Cancel,
            KeyCode::Left => {
                self.focus_prev();
                ConfirmDialogOutcome::Continued
            }
            KeyCode::Right => {
                self.focus_next();
                ConfirmDialogOutcome::Continued
            }
            KeyCode::Tab => {
                self.focus_next();
                ConfirmDialogOutcome::Continued
            }
            KeyCode::Enter => self.outcome_for_index(self.focused),
            _ => ConfirmDialogOutcome::Ignored,
        }
    }

    fn build_button_row(&self) -> DialogRow {
        let accent = Accent::Yellow.color();
        let mut spans: Vec<Span<'static>> = Vec::new();
        for (i, label) in self.buttons.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(FOOTER_SEPARATOR.to_string()));
            }
            let style = if i == self.focused {
                Style::default()
                    .bg(accent)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            spans.push(Span::styled(format!(" {label} "), style));
        }
        DialogRow {
            spans,
            selectable: false,
            selected: false,
        }
    }

    /// Render the dialog as a centred overlay inside `area`. Uses the
    /// shared dialog_theme renderer for the rounded yellow border +
    /// black background + bold inner title; appends a button row.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let body_row = DialogRow {
            spans: vec![Span::raw(self.body.clone())],
            selectable: false,
            selected: false,
        };
        let spacer = DialogRow {
            spans: vec![Span::raw(String::new())],
            selectable: false,
            selected: false,
        };
        let dialog = FspecDialog {
            accent: Accent::Yellow,
            title: &self.title,
            rows: vec![body_row, spacer, self.build_button_row()],
            footer: "",
            min_width: 40,
        };
        render_dialog(area, buf, &dialog);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn confirm_dialog_rendering_is_byte_equal_across_runs_insta_snapshot() {
        let dialog = ConfirmDialog::new(
            "Delete Session",
            "This action cannot be undone.",
            "Delete",
            None,
            "Cancel",
        );
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
        terminal
            .draw(|frame| {
                dialog.render(frame.area(), frame.buffer_mut());
            })
            .expect("draw");
        let buf = terminal.backend().buffer().clone();
        let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
        for y in 0..buf.area.height {
            let mut row = String::with_capacity(buf.area.width as usize);
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            rows.push(row);
        }
        insta::assert_yaml_snapshot!("confirm_dialog__centered_popup_80x24", rows);
    }
}
