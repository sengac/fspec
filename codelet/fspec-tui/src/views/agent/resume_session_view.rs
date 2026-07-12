//! RPC-026 — ResumeSessionView: full-screen session picker.
//! Mode view for `/resume`. ↑/↓ navigate, Enter/Double-click emits `Selected`.

use codelet_rpc_types::{SessionId, SessionInfo};
use crossterm::event::{KeyCode, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::time::{Duration, Instant};

use super::confirm_dialog::{ConfirmDialog, ConfirmDialogOutcome};
use super::mode_view_render::render_session_rows;
use crate::components::scroll_viewport::{
    ensure_visible, wrap_index, WheelDirection, WheelVelocity,
};

const CHROME_ROWS: u16 = 3;
const DOUBLE_CLICK_TIMEOUT: Duration = Duration::from_millis(300);

/// Detects double-clicks: two left-button-down events on the same row
/// within `DOUBLE_CLICK_TIMEOUT` (300ms).
struct DoubleClickDetector {
    last_click_row: Option<usize>,
    last_click_time: Option<Instant>,
}

impl DoubleClickDetector {
    fn new() -> Self {
        Self {
            last_click_row: None,
            last_click_time: None,
        }
    }

    /// Returns `true` if the click on `row` at `now` is a double-click.
    fn record_click(&mut self, row: usize, now: Instant) -> bool {
        let is_double = match (self.last_click_row, self.last_click_time) {
            (Some(last_row), Some(last_time)) => {
                last_row == row && now.duration_since(last_time) <= DOUBLE_CLICK_TIMEOUT
            }
            _ => false,
        };
        self.last_click_row = Some(row);
        self.last_click_time = Some(now);
        is_double
    }
}

/// Outcome of routing a key or mouse event through the resume view.
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
    double_click: DoubleClickDetector,
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
            double_click: DoubleClickDetector::new(),
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

    /// Replace the session list.
    pub fn set_sessions(&mut self, sessions: Vec<SessionInfo>) {
        self.sessions = sessions;
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    /// TUI-096: Each session occupies 2 visual rows.
    fn adjust_scroll(&mut self, visible_rows: usize) {
        let visible_sessions = visible_rows / 2;
        ensure_visible(
            &mut self.scroll_offset,
            self.selected_index,
            visible_sessions,
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

    /// Hit-test `ev` against `body_rect`.
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
                    let now = Instant::now();
                    if self.double_click.record_click(candidate, now) {
                        // Double-click: resume session immediately
                        let info = &self.sessions[candidate];
                        return ResumeSessionViewOutcome::Selected(SessionId::new(
                            info.id.clone(),
                        ));
                    }
                    // Single-click: move selection
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

    /// Route a key event through the view.
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
                        Some(info) => ResumeSessionViewOutcome::ConfirmedDelete(SessionId::new(
                            info.id.clone(),
                        )),
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

    /// Paint the view into the FULL area Rect.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        crate::views::full_screen_shell::render_full_screen_scaffold(
            area,
            buf,
            "Resume Session",
            self.sessions.len(),
            "available",
            "DblClick Resume | Enter Select | ↑↓ Navigate | D Delete | Esc Cancel",
            |body_area, buf| {
                render_session_rows(
                    body_area,
                    buf,
                    &self.sessions,
                    self.selected_index,
                    self.scroll_offset,
                );
            },
            self.delete_confirm.as_ref(),
        );
    }

    /// Heuristic visible-row hint for `handle_key`'s `visible_rows` argument.
    pub fn visible_rows_for(area: Rect) -> usize {
        area.height.saturating_sub(CHROME_ROWS) as usize
    }
}
