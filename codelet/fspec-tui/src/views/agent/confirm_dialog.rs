//! RPC-026 — ConfirmDialog: centred confirmation overlay.
//!
//! Feature: spec/features/rpc026-resume-and-search-mode-views.feature
//!
//! Rust equivalent of the TS ThreeButtonDialog (AgentView.tsx).
//! Hosts up to three labelled buttons (primary / secondary / cancel)
//! laid out in the dialog's footer row. Left/Right cycle focus; Enter
//! activates the focused button; Esc returns `Cancel`.
//!
//! Unlike the resume / search mode views, this widget IS conceptually
//! a popup — it's rendered as a small centred floating block over the
//! parent view. Used by `ResumeSessionView` to confirm deletion.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

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

    fn dialog_rect(&self, area: Rect) -> Rect {
        let body_width = self.body.chars().count().max(self.title.chars().count()) as u16;
        let width = body_width.saturating_add(4).max(40).min(area.width);
        let height: u16 = 6;
        let height = height.min(area.height);
        let x = area.x.saturating_add(area.width.saturating_sub(width) / 2);
        let y = area.y.saturating_add(area.height.saturating_sub(height) / 2);
        Rect { x, y, width, height }
    }

    fn render_buttons(&self, area: Rect, buf: &mut Buffer) {
        let mut spans: Vec<Span<'static>> = Vec::new();
        for (i, label) in self.buttons.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" │ "));
            }
            let style = if i == self.focused {
                Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
            } else {
                Style::default()
            };
            spans.push(Span::styled(format!(" {label} "), style));
        }
        Paragraph::new(Line::from(spans)).render(area, buf);
    }

    /// Render the dialog as a centred overlay inside `area`. Paints
    /// `Clear` over the dialog rect first so the parent view is
    /// hidden behind the dialog body.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let dialog = self.dialog_rect(area);
        Clear.render(dialog, buf);
        let block = Block::default()
            .borders(Borders::ALL)
            .title(self.title.clone());
        let inner = block.inner(dialog);
        block.render(dialog, buf);
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(inner);
        Paragraph::new(self.body.clone()).render(layout[0], buf);
        self.render_buttons(layout[1], buf);
    }
}
