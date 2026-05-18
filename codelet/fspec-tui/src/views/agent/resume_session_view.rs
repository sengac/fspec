//! RPC-026 — ResumeSessionView: full-screen session picker.
//!
//! Feature: spec/features/rpc026-resume-and-search-mode-views.feature
//!
//! Mode view rendered when the user types `/resume` (slash command) —
//! replaces the legacy popup-style resume picker. Paints into the
//! ENTIRE area Rect, hiding AgentView's normal header/scrollback/
//! input/footer layout. Mirrors TS AgentView.tsx resume mode
//! (lines 1336-1398, 5002-5191).
//!
//! Behaviour:
//!   * ↑/↓ navigate with wrap-around AND scroll-window updates so the
//!     selected row stays visible inside the terminal height.
//!   * Enter emits `Selected(SessionId)`.
//!   * D opens a `ConfirmDialog` overlay scoped to this view.
//!   * Esc emits `Dismiss`.
//!   * Inside the delete-confirm dialog: Enter on Primary emits
//!     `ConfirmedDelete`; Esc / Cancel emits `CancelledDelete`.
//!
//! Forbidden imports (per source-shape regression): this widget does
//! NOT depend on the legacy floating-popup machinery.

use codelet_rpc_types::{SessionId, SessionInfo};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph, Widget};

use super::confirm_dialog::{ConfirmDialog, ConfirmDialogOutcome};

const CHROME_ROWS: u16 = 3;

/// Outcome of routing a single key event through the resume view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResumeSessionViewOutcome {
    Selected(SessionId),
    Dismiss,
    RequestDelete(SessionId),
    ConfirmedDelete(SessionId),
    CancelledDelete,
    Continued,
    Ignored,
}

/// Full-screen resume session picker.
pub struct ResumeSessionView {
    sessions: Vec<SessionInfo>,
    selected_index: usize,
    scroll_offset: usize,
    delete_confirm: Option<ConfirmDialog>,
}

impl Default for ResumeSessionView {
    fn default() -> Self {
        Self::new()
    }
}

impl ResumeSessionView {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            delete_confirm: None,
        }
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn selected(&self) -> Option<&SessionInfo> {
        self.sessions.get(self.selected_index)
    }

    pub fn sessions(&self) -> &[SessionInfo] {
        &self.sessions
    }

    pub fn delete_confirm(&self) -> Option<&ConfirmDialog> {
        self.delete_confirm.as_ref()
    }

    /// Replace the session list. Selection + scroll are reset.
    pub fn set_sessions(&mut self, sessions: Vec<SessionInfo>) {
        self.sessions = sessions;
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    fn adjust_scroll(&mut self, visible_rows: usize) {
        if visible_rows == 0 || self.sessions.is_empty() {
            self.scroll_offset = 0;
            return;
        }
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + visible_rows {
            self.scroll_offset = self.selected_index + 1 - visible_rows;
        }
    }

    fn move_up(&mut self, visible_rows: usize) {
        if self.sessions.is_empty() {
            return;
        }
        if self.selected_index == 0 {
            self.selected_index = self.sessions.len() - 1;
            self.scroll_offset = self
                .sessions
                .len()
                .saturating_sub(visible_rows.max(1));
        } else {
            self.selected_index -= 1;
        }
        self.adjust_scroll(visible_rows);
    }

    fn move_down(&mut self, visible_rows: usize) {
        if self.sessions.is_empty() {
            return;
        }
        if self.selected_index + 1 >= self.sessions.len() {
            self.selected_index = 0;
            self.scroll_offset = 0;
        } else {
            self.selected_index += 1;
        }
        self.adjust_scroll(visible_rows);
    }

    /// Route a single key event through the view.
    ///
    /// `visible_rows` is the body height the view will receive on the
    /// next render — used to keep `scroll_offset` aligned. Passing 0
    /// effectively disables the scroll-window math (selection still
    /// advances).
    pub fn handle_key(
        &mut self,
        code: KeyCode,
        mods: KeyModifiers,
        visible_rows: usize,
    ) -> ResumeSessionViewOutcome {
        if let Some(dialog) = self.delete_confirm.as_mut() {
            match dialog.handle_key(code, mods) {
                ConfirmDialogOutcome::Primary => {
                    let outcome = match self.selected() {
                        Some(info) => ResumeSessionViewOutcome::ConfirmedDelete(
                            SessionId::new(info.id.clone()),
                        ),
                        None => ResumeSessionViewOutcome::CancelledDelete,
                    };
                    self.delete_confirm = None;
                    return outcome;
                }
                ConfirmDialogOutcome::Secondary | ConfirmDialogOutcome::Cancel => {
                    self.delete_confirm = None;
                    return ResumeSessionViewOutcome::CancelledDelete;
                }
                ConfirmDialogOutcome::Continued => return ResumeSessionViewOutcome::Continued,
                ConfirmDialogOutcome::Ignored => return ResumeSessionViewOutcome::Ignored,
            }
        }
        if mods.contains(KeyModifiers::CONTROL) || mods.contains(KeyModifiers::ALT) {
            return ResumeSessionViewOutcome::Ignored;
        }
        match code {
            KeyCode::Esc => ResumeSessionViewOutcome::Dismiss,
            KeyCode::Up => {
                self.move_up(visible_rows);
                ResumeSessionViewOutcome::Continued
            }
            KeyCode::Down => {
                self.move_down(visible_rows);
                ResumeSessionViewOutcome::Continued
            }
            KeyCode::Enter => match self.selected() {
                Some(info) => ResumeSessionViewOutcome::Selected(SessionId::new(info.id.clone())),
                None => ResumeSessionViewOutcome::Ignored,
            },
            KeyCode::Char('d') | KeyCode::Char('D') => match self.selected() {
                Some(info) => {
                    let id = info.id.clone();
                    let body = format!("Delete session {id}?");
                    self.delete_confirm = Some(ConfirmDialog::new(
                        "Delete session?",
                        body,
                        "Delete",
                        None,
                        "Cancel",
                    ));
                    ResumeSessionViewOutcome::RequestDelete(SessionId::new(id))
                }
                None => ResumeSessionViewOutcome::Ignored,
            },
            _ => ResumeSessionViewOutcome::Ignored,
        }
    }

    fn render_title(&self, area: Rect, buf: &mut Buffer) {
        let text = format!("Resume Session ({} available)", self.sessions.len());
        let style = Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD);
        Paragraph::new(Line::from(Span::styled(text, style))).render(area, buf);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Enter Select | ↑↓ Navigate | D Delete | Esc Cancel")
            .render(area, buf);
    }

    fn render_body(&self, area: Rect, buf: &mut Buffer) {
        if self.sessions.is_empty() {
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
        let end = (self.scroll_offset + visible_rows).min(self.sessions.len());
        for (row_idx, info) in self.sessions[self.scroll_offset..end].iter().enumerate() {
            let global_idx = self.scroll_offset + row_idx;
            let marker = if global_idx == self.selected_index { "▸" } else { " " };
            let label = format!(" {marker} {} ({})", info.id, info.status);
            let style = if global_idx == self.selected_index {
                Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
            } else {
                Style::default()
            };
            let y = area.y + row_idx as u16;
            let row_area = Rect { x: area.x, y, width: area.width, height: 1 };
            Paragraph::new(Line::from(Span::styled(label, style))).render(row_area, buf);
        }
    }

    /// Paint the view into the FULL area Rect. The first statement is
    /// `Clear.render(area, buf)` so the underlying AgentView pixels are
    /// fully overwritten. When `delete_confirm` is `Some`, the dialog
    /// overlay is painted on top after the base paint.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);
        let title_area = split[0];
        let body_area = split[2];
        let footer_area = split[3];
        self.render_title(title_area, buf);
        self.render_body(body_area, buf);
        self.render_footer(footer_area, buf);
        if let Some(dialog) = self.delete_confirm.as_ref() {
            dialog.render(area, buf);
        }
    }

    /// Heuristic visible-row hint used by AgentView when computing
    /// `handle_key`'s `visible_rows` argument. Subtracts the title +
    /// separator + footer chrome from `area.height`.
    pub fn visible_rows_for(area: Rect) -> usize {
        area.height.saturating_sub(CHROME_ROWS) as usize
    }
}
