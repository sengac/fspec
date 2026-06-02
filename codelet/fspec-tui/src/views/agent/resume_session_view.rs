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
use crossterm::event::{KeyCode, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Clear, Widget};

use super::confirm_dialog::{ConfirmDialog, ConfirmDialogOutcome};
use super::mode_view_render::{render_footer_hint, render_session_rows, render_title_with_count};
use crate::components::scroll_viewport::{
    ensure_visible, wrap_index, WheelDirection, WheelVelocity,
};

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
    wheel: WheelVelocity,
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
            wheel: WheelVelocity::new(),
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
        ensure_visible(
            &mut self.scroll_offset,
            self.selected_index,
            visible_rows,
            self.sessions.len(),
        );
    }

    fn move_by(&mut self, delta: i32, visible_rows: usize) {
        if self.sessions.is_empty() {
            return;
        }
        self.selected_index = wrap_index(self.selected_index, delta, self.sessions.len());
        self.adjust_scroll(visible_rows);
    }

    /// Hit-test `ev` against `body_rect` — returns true when the event
    /// falls inside the rect.
    fn rect_contains(ev: MouseEvent, body_rect: Rect) -> bool {
        ev.column >= body_rect.x
            && ev.column < body_rect.x + body_rect.width
            && ev.row >= body_rect.y
            && ev.row < body_rect.y + body_rect.height
    }

    /// Route a mouse event hit-tested against the view's `body_rect`.
    pub fn handle_mouse(
        &mut self,
        ev: MouseEvent,
        body_rect: Rect,
        visible_rows: usize,
    ) -> ResumeSessionViewOutcome {
        if !Self::rect_contains(ev, body_rect) {
            return ResumeSessionViewOutcome::Ignored;
        }
        match ev.kind {
            MouseEventKind::ScrollUp => {
                let step = self.wheel.step(WheelDirection::Up);
                self.move_by(step, visible_rows);
                ResumeSessionViewOutcome::Continued
            }
            MouseEventKind::ScrollDown => {
                let step = self.wheel.step(WheelDirection::Down);
                self.move_by(step, visible_rows);
                ResumeSessionViewOutcome::Continued
            }
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                let candidate = self.scroll_offset + (ev.row - body_rect.y) as usize;
                if candidate < self.sessions.len() {
                    self.selected_index = candidate;
                    self.adjust_scroll(visible_rows);
                    ResumeSessionViewOutcome::Continued
                } else {
                    ResumeSessionViewOutcome::Ignored
                }
            }
            _ => ResumeSessionViewOutcome::Ignored,
        }
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
                self.move_by(-1, visible_rows);
                ResumeSessionViewOutcome::Continued
            }
            KeyCode::Down => {
                self.move_by(1, visible_rows);
                ResumeSessionViewOutcome::Continued
            }
            KeyCode::PageUp => {
                self.move_by(-(visible_rows.max(1) as i32), visible_rows);
                ResumeSessionViewOutcome::Continued
            }
            KeyCode::PageDown => {
                self.move_by(visible_rows.max(1) as i32, visible_rows);
                ResumeSessionViewOutcome::Continued
            }
            KeyCode::Home => {
                self.selected_index = 0;
                self.scroll_offset = 0;
                ResumeSessionViewOutcome::Continued
            }
            KeyCode::End => {
                if !self.sessions.is_empty() {
                    self.selected_index = self.sessions.len() - 1;
                    self.adjust_scroll(visible_rows);
                }
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
        render_title_with_count(title_area, buf, "Resume Session", self.sessions.len(), "available");
        render_session_rows(
            body_area,
            buf,
            &self.sessions,
            self.selected_index,
            self.scroll_offset,
        );
        render_footer_hint(
            footer_area,
            buf,
            "Enter Select | ↑↓ Navigate | D Delete | Esc Cancel",
        );
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
