//! RPC-026 — Resume picker popup widget.
//!
//! Feature: spec/features/rpc026-resume-picker.feature
//!
//! Centred floating overlay rendered above AgentView's MultiLineInput
//! when the user picks the `/resume` slash command. Lists every
//! SessionInfo returned by the backend's `list_sessions()` call, in
//! the order the backend delivered them (no client-side sort —
//! preserves the RPC contract).
//!
//! Ownership: AgentView holds an `Option<ResumePicker>`. When `Some`,
//! AgentView routes its keystrokes through `handle_key` BEFORE
//! forwarding to MultiLineInput. App::dispatch fires the result
//! via `Action::AttachToSession` once Enter resolves a selection.
//!
//! ↑/↓ navigate with wrap-around; Enter selects; Tab is ignored
//! (no partial fill makes sense); Esc dismisses.

use codelet_rpc_types::{SessionId, SessionInfo};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use tui_popup::Popup;

use super::popup_body::{widest_line, PopupBody};

/// Outcome of routing a single key event through the resume picker.
#[derive(Debug, Clone)]
pub enum ResumePickerOutcome {
    /// User picked a session with Enter — App::dispatch should fire
    /// `Action::AttachToSession(id)`.
    Selected(SessionId),
    /// User pressed Esc — drop the popup, leave AgentViewStore alone.
    Dismiss,
    /// Popup handled the key internally (navigation).
    Continued,
    /// Popup ignored the key — caller may route it elsewhere.
    Ignored,
}

/// Resume picker palette state.
pub struct ResumePicker {
    sessions: Vec<SessionInfo>,
    selected_index: usize,
}

impl Default for ResumePicker {
    fn default() -> Self {
        Self::new()
    }
}

impl ResumePicker {
    /// Construct a fresh popup with no sessions and selected_index == 0.
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            selected_index: 0,
        }
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn selected(&self) -> Option<&SessionInfo> {
        self.sessions.get(self.selected_index)
    }

    pub fn sessions(&self) -> &[SessionInfo] {
        &self.sessions
    }

    /// Replace the session list. Selection is reset to the first row.
    pub fn set_sessions(&mut self, sessions: Vec<SessionInfo>) {
        self.sessions = sessions;
        self.selected_index = 0;
    }

    fn move_up(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        if self.selected_index == 0 {
            self.selected_index = self.sessions.len() - 1;
        } else {
            self.selected_index -= 1;
        }
    }

    fn move_down(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        if self.selected_index + 1 >= self.sessions.len() {
            self.selected_index = 0;
        } else {
            self.selected_index += 1;
        }
    }

    /// Route a single key event through the popup. Returns
    /// `Ignored` for any modifier-prefixed chord so AgentView can
    /// route Shift+arrow back through its own handlers.
    pub fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> ResumePickerOutcome {
        if mods.contains(KeyModifiers::SHIFT) || mods.contains(KeyModifiers::CONTROL) {
            return ResumePickerOutcome::Ignored;
        }
        match code {
            KeyCode::Esc => ResumePickerOutcome::Dismiss,
            KeyCode::Up => {
                self.move_up();
                ResumePickerOutcome::Continued
            }
            KeyCode::Down => {
                self.move_down();
                ResumePickerOutcome::Continued
            }
            KeyCode::Enter => match self.selected() {
                Some(info) => ResumePickerOutcome::Selected(SessionId::new(info.id.clone())),
                None => ResumePickerOutcome::Ignored,
            },
            _ => ResumePickerOutcome::Ignored,
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let body = self.build_body();
        let width = widest_line(&body) + 2;
        let height = body.lines().count() as u16;
        let sized = PopupBody {
            text: body,
            selected_index: self.selected_index,
            width,
            height,
        };
        Popup::new(sized).title("Resume Session").render(area, buf);
    }

    fn build_body(&self) -> String {
        if self.sessions.is_empty() {
            return "(no sessions to resume)".to_string();
        }
        let mut out = String::new();
        for (i, info) in self.sessions.iter().take(10).enumerate() {
            let marker = if i == self.selected_index { "▸" } else { " " };
            let label = format!(" {} ({})", info.name, info.status);
            out.push_str(&format!("{marker} {id}{label}\n", id = info.id));
        }
        out.push_str("\n↑↓ Navigate │ Enter Attach │ Esc Close");
        out
    }
}
