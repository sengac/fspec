//! HitlDialog — Priority::Critical modal for `request_user_input` prompts.
//!
//! Feature: spec/features/pause-and-hitl-dialogs.feature
//!
//! Renders an `HitlRequest`:
//!   - `request.question` as the dialog title.
//!   - `request.header` as the first body row.
//!   - one row per `HitlOption` labelled with a hotkey letter
//!     (`a`, `b`, `c`, …) plus the option label and description.
//!   - when `request.allow_text_input` is true, an additional
//!     free-text input row below the options that can be selected via
//!     Tab/Down and accepts character keystrokes / Backspace.
//!
//! Key handling:
//!   - Up/Down/Tab/BackTab cycles the selected row (looping).
//!   - Hotkey letter (a..z, case-insensitive) selects + submits the
//!     matching option immediately.
//!   - Enter on a highlighted option submits that option.
//!   - Enter on the free-text row submits the typed text.
//!   - Esc pops the dialog without submitting.

use crossterm::event::{Event, KeyCode, KeyModifiers, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;

use codelet_rpc_types::{HitlRequest, HitlResponse, SessionId};

use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog};
use super::{Action, Callback, Component, EventResult, Priority};

/// Canonical id used by `Compositor::remove` when the dialog dismisses.
pub const HITL_DIALOG_ID: &str = "hitl-dialog";

const FOOTER: &str = "↑↓/Tab Cycle │ Enter Submit │ a/b/c Hotkey │ Esc Dismiss";

const HOTKEY_LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";

/// Priority::Critical modal dialog for HITL prompts.
pub struct HitlDialog {
    id: String,
    session_id: SessionId,
    request: HitlRequest,
    /// Selected row index — 0 = first option, … allow_text_input adds
    /// a trailing free-text row.
    selected: usize,
    /// Live free-text buffer; only rendered/used when
    /// `request.allow_text_input` is true.
    text: String,
    action_tx: Option<UnboundedSender<Action>>,
    pending_action: Option<Action>,
}

impl HitlDialog {
    pub fn new(session_id: SessionId, request: HitlRequest) -> Self {
        Self {
            id: HITL_DIALOG_ID.to_string(),
            session_id,
            request,
            selected: 0,
            text: String::new(),
            action_tx: None,
            pending_action: None,
        }
    }

    pub fn with_action_tx(mut self, action_tx: UnboundedSender<Action>) -> Self {
        self.action_tx = Some(action_tx);
        self
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub fn request(&self) -> &HitlRequest {
        &self.request
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn text_buffer(&self) -> &str {
        &self.text
    }

    /// Total number of selectable rows: every option + 1 free-text row
    /// when `allow_text_input`.
    pub fn selectable_row_count(&self) -> usize {
        self.request.options.len() + if self.request.allow_text_input { 1 } else { 0 }
    }

    /// True iff the currently-selected row is the free-text input row.
    pub fn is_free_text_selected(&self) -> bool {
        self.request.allow_text_input && self.selected == self.request.options.len()
    }

    /// Hotkey letter assigned to an option index (`a` = option 0, `b` =
    /// option 1, …). Returns None when the index is out of range or
    /// exceeds the alphabet (>26 options).
    pub fn option_hotkey(&self, option_index: usize) -> Option<char> {
        HOTKEY_LETTERS.chars().nth(option_index)
    }

    /// Test accessor — set the selected row directly. Used by tests
    /// scripted with explicit row-focus preconditions.
    pub fn set_selected_index(&mut self, index: usize) {
        let n = self.selectable_row_count();
        if n == 0 {
            self.selected = 0;
        } else {
            self.selected = index.min(n - 1);
        }
    }

    /// Test accessor — overwrite the free-text buffer.
    pub fn set_text_buffer(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    /// Test-only: drain any pending action stashed by `handle_event`.
    pub fn take_pending_action(&mut self) -> Option<Action> {
        self.pending_action.take()
    }

    fn move_selection(&mut self, delta: i32) {
        let n = self.selectable_row_count() as i32;
        if n == 0 {
            return;
        }
        self.selected = (self.selected as i32 + delta).rem_euclid(n) as usize;
    }

    fn emit_action(&mut self, action: Action) {
        if let Some(tx) = self.action_tx.as_ref() {
            let _ = tx.send(action.clone());
        }
        self.pending_action = Some(action);
    }

    fn submit_option(&mut self, option_index: usize) -> EventResult {
        let Some(opt) = self.request.options.get(option_index) else {
            return EventResult::ignored();
        };
        let response = HitlResponse {
            id: self.request.id.clone(),
            value: opt.label.clone(),
        };
        let action = Action::HitlSubmitted {
            session_id: self.session_id.clone(),
            response,
        };
        self.emit_action(action);
        let id = self.id.clone();
        let callback: Callback = Box::new(move |compositor| {
            let _ = compositor.remove(&id);
        });
        EventResult::Consumed(Some(callback))
    }

    fn submit_free_text(&mut self) -> EventResult {
        let response = HitlResponse {
            id: self.request.id.clone(),
            value: self.text.clone(),
        };
        let action = Action::HitlSubmitted {
            session_id: self.session_id.clone(),
            response,
        };
        self.emit_action(action);
        let id = self.id.clone();
        let callback: Callback = Box::new(move |compositor| {
            let _ = compositor.remove(&id);
        });
        EventResult::Consumed(Some(callback))
    }

    fn esc_and_pop(&mut self) -> EventResult {
        let id = self.id.clone();
        let callback: Callback = Box::new(move |compositor| {
            let _ = compositor.remove(&id);
        });
        EventResult::Consumed(Some(callback))
    }

    fn rows_for_render(&self) -> Vec<DialogRow> {
        let mut rows: Vec<DialogRow> = Vec::new();
        let header_style = Style::default().fg(Color::Gray).bg(Color::Black);
        rows.push(DialogRow {
            spans: vec![Span::styled(self.request.header.clone(), header_style)],
            selectable: false,
            selected: false,
        });
        rows.push(DialogRow {
            spans: vec![Span::raw(String::new())],
            selectable: false,
            selected: false,
        });
        for (i, opt) in self.request.options.iter().enumerate() {
            let hotkey = self
                .option_hotkey(i)
                .map(|c| format!("[{c}] "))
                .unwrap_or_default();
            let label = format!("{hotkey}{}", opt.label);
            let desc_text = if opt.description.is_empty() {
                String::new()
            } else {
                format!("  — {}", opt.description)
            };
            let row = DialogRow {
                spans: vec![
                    Span::raw(label),
                    Span::styled(desc_text, Style::default().add_modifier(Modifier::DIM)),
                ],
                selectable: true,
                selected: self.selected == i,
            };
            rows.push(row);
        }
        if self.request.allow_text_input {
            let is_selected = self.is_free_text_selected();
            let buf = if self.text.is_empty() {
                "(type a custom reply…)".to_string()
            } else {
                self.text.clone()
            };
            rows.push(DialogRow {
                spans: vec![Span::raw("> ".to_string()), Span::raw(buf)],
                selectable: true,
                selected: is_selected,
            });
        }
        rows
    }
}

impl Component for HitlDialog {
    fn priority(&self) -> Priority {
        Priority::Critical
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Esc => return self.esc_and_pop(),
                KeyCode::Enter => {
                    if self.is_free_text_selected() {
                        return self.submit_free_text();
                    }
                    let option_idx = self
                        .selected
                        .min(self.request.options.len().saturating_sub(1));
                    return self.submit_option(option_idx);
                }
                KeyCode::Tab | KeyCode::Down => {
                    self.move_selection(1);
                    return EventResult::consumed();
                }
                KeyCode::BackTab | KeyCode::Up => {
                    self.move_selection(-1);
                    return EventResult::consumed();
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    // If the free-text row is focused, treat as text input.
                    if self.is_free_text_selected() {
                        self.text.push(c);
                        return EventResult::consumed();
                    }
                    // Otherwise: hotkey lookup. Match by alphabet position.
                    let lower = c.to_ascii_lowercase();
                    if let Some(idx) = HOTKEY_LETTERS.chars().position(|h| h == lower) {
                        if idx < self.request.options.len() {
                            return self.submit_option(idx);
                        }
                    }
                    return EventResult::ignored();
                }
                KeyCode::Backspace if self.is_free_text_selected() => {
                    self.text.pop();
                    return EventResult::consumed();
                }
                _ => {}
            }
        }
        if let Event::Mouse(m) = event {
            match m.kind {
                MouseEventKind::ScrollUp => {
                    self.move_selection(-1);
                    return EventResult::consumed();
                }
                MouseEventKind::ScrollDown => {
                    self.move_selection(1);
                    return EventResult::consumed();
                }
                _ => {}
            }
        }
        EventResult::ignored()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let rows = self.rows_for_render();
        let dialog = FspecDialog {
            accent: Accent::Cyan,
            title: &self.request.question,
            rows,
            footer: FOOTER,
            min_width: 60,
        };
        render_dialog(area, buf, &dialog);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use codelet_rpc_types::HitlOption;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn req() -> HitlRequest {
        HitlRequest {
            id: "q-1".to_string(),
            question: "Apply?".to_string(),
            header: "Apply changes?".to_string(),
            options: vec![
                HitlOption {
                    label: "Yes".to_string(),
                    description: "Apply".to_string(),
                },
                HitlOption {
                    label: "No".to_string(),
                    description: "Cancel".to_string(),
                },
            ],
            allow_text_input: false,
        }
    }

    #[test]
    fn hotkey_a_submits_first_option() {
        let mut dialog = HitlDialog::new(SessionId::new("s-1"), req());
        let _ = dialog.handle_event(&key(KeyCode::Char('a')));
        match dialog.take_pending_action() {
            Some(Action::HitlSubmitted {
                session_id,
                response,
            }) => {
                assert_eq!(session_id, SessionId::new("s-1"));
                assert_eq!(response.id, "q-1");
                assert_eq!(response.value, "Yes");
            }
            other => panic!("expected HitlSubmitted with 'Yes', got {other:?}"),
        }
    }

    #[test]
    fn enter_on_second_option_submits_no() {
        let mut dialog = HitlDialog::new(SessionId::new("s-1"), req());
        dialog.set_selected_index(1);
        let _ = dialog.handle_event(&key(KeyCode::Enter));
        match dialog.take_pending_action() {
            Some(Action::HitlSubmitted { response, .. }) => {
                assert_eq!(response.value, "No");
            }
            other => panic!("expected HitlSubmitted with 'No', got {other:?}"),
        }
    }

    #[test]
    fn esc_does_not_emit_submission() {
        let mut dialog = HitlDialog::new(SessionId::new("s-1"), req());
        let _ = dialog.handle_event(&key(KeyCode::Esc));
        assert!(dialog.take_pending_action().is_none());
    }

    #[test]
    fn free_text_row_accepts_typed_chars_and_submits_on_enter() {
        let mut r = req();
        r.allow_text_input = true;
        let mut dialog = HitlDialog::new(SessionId::new("s-1"), r);
        // Tab past both options into the free-text row.
        let _ = dialog.handle_event(&key(KeyCode::Tab));
        let _ = dialog.handle_event(&key(KeyCode::Tab));
        assert!(dialog.is_free_text_selected());
        for c in "maybe later".chars() {
            let _ = dialog.handle_event(&key(KeyCode::Char(c)));
        }
        assert_eq!(dialog.text_buffer(), "maybe later");
        let _ = dialog.handle_event(&key(KeyCode::Enter));
        match dialog.take_pending_action() {
            Some(Action::HitlSubmitted { response, .. }) => {
                assert_eq!(response.value, "maybe later");
            }
            other => panic!("expected HitlSubmitted with text, got {other:?}"),
        }
    }
}
