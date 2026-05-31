//! RoleDialog — Priority::Foreground modal for editing the per-session
//! role overlay.
//!
//! Feature: spec/features/role-dialog-component.feature
//!
//! Card: RPC-063 (parent RPC-030).
//!
//! Mounted by the `/role` slash command via
//! `App::handle_open_role_dialog` (in `app/dispatch_rpc063.rs`). The
//! dialog seeds its draft from `AgentViewStore::role_for(current_session)`
//! and emits `Action::SetSessionRole(session_id, Some(text))` on Enter
//! or `Action::SetSessionRole(session_id, None)` on Ctrl+D / Enter on
//! an empty draft. Esc is a no-op cancel.
//!
//! Renders via the shared `dialog_theme::render_dialog` with
//! `Accent::Cyan` — mirroring the cyan role banner accent used by
//! `RoleBanner` (`views/agent/role_banner.rs`).

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;
use tui_textarea::{Input, TextArea};

use codelet_rpc_types::SessionId;

use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog, FOOTER_SEPARATOR};
use super::{Action, Callback, Component, EventResult, Priority};

/// Canonical id used by `Compositor::remove` AND
/// `Compositor::contains` to address the role-dialog idempotently.
pub const ROLE_DIALOG_ID: &str = "role-dialog";

/// Priority::Foreground modal dialog for editing the session role.
///
/// Wraps a single-line `tui_textarea::TextArea` so the user can type,
/// edit, and paste the role text. The current draft is exposed via
/// [`Self::draft`] for tests + the dispatcher's introspection.
pub struct RoleDialog {
    id: String,
    session_id: SessionId,
    textarea: TextArea<'static>,
    action_tx: Option<UnboundedSender<Action>>,
    pending_action: Option<Action>,
}

impl RoleDialog {
    /// Construct a fresh dialog bound to `session_id`. When
    /// `seed_role` is `Some(text)`, the editor's initial content is
    /// `text` with the cursor at the end. When `None`, the editor is
    /// empty.
    pub fn new(session_id: SessionId, seed_role: Option<String>) -> Self {
        let lines: Vec<String> = match seed_role {
            Some(text) if !text.is_empty() => text.split('\n').map(str::to_string).collect(),
            _ => vec![String::new()],
        };
        let mut textarea = TextArea::new(lines);
        // Match MultiLineInput: hide the textarea's own cursor-line
        // highlight; the dialog theme paints the row backgrounds.
        textarea.set_cursor_line_style(Style::default());
        textarea.move_cursor(tui_textarea::CursorMove::End);
        Self {
            id: ROLE_DIALOG_ID.to_string(),
            session_id,
            textarea,
            action_tx: None,
            pending_action: None,
        }
    }

    /// Optional builder hook — wire the App's action channel so the
    /// dialog can emit follow-up actions in addition to stashing them
    /// in `pending_action`.
    pub fn with_action_tx(mut self, action_tx: UnboundedSender<Action>) -> Self {
        self.action_tx = Some(action_tx);
        self
    }

    /// Current draft text — lines joined with '\n'. Trimmed of nothing
    /// (whitespace is preserved verbatim so the user can type
    /// significant leading/trailing spaces if they want to).
    pub fn draft(&self) -> String {
        self.textarea.lines().join("\n")
    }

    /// Test-only accessor — drain the stashed pending action.
    pub fn take_pending_action(&mut self) -> Option<Action> {
        self.pending_action.take()
    }

    fn emit_action(&mut self, action: Action) {
        if let Some(tx) = self.action_tx.as_ref() {
            let _ = tx.send(action.clone());
        }
        self.pending_action = Some(action);
    }

    fn remove_callback(&self) -> Callback {
        let id = self.id.clone();
        Box::new(move |compositor| {
            let _ = compositor.remove(&id);
        })
    }

    fn save(&mut self) -> EventResult {
        let draft = self.draft();
        let role = if draft.is_empty() { None } else { Some(draft) };
        self.emit_action(Action::SetSessionRole(self.session_id.clone(), role));
        EventResult::Consumed(Some(self.remove_callback()))
    }

    fn clear(&mut self) -> EventResult {
        self.emit_action(Action::SetSessionRole(self.session_id.clone(), None));
        EventResult::Consumed(Some(self.remove_callback()))
    }

    fn cancel(&self) -> EventResult {
        EventResult::Consumed(Some(self.remove_callback()))
    }
}

impl Component for RoleDialog {
    fn priority(&self) -> Priority {
        Priority::Foreground
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        if let Event::Key(key) = event {
            // Ctrl+D — clear the role and dismiss.
            if key.code == KeyCode::Char('d') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return self.clear();
            }
            match key.code {
                KeyCode::Esc => return self.cancel(),
                KeyCode::Enter if key.modifiers.is_empty() => return self.save(),
                _ => {}
            }
            // Everything else is routed to the textarea so the user can
            // type, edit, backspace, paste, etc. Shift+Enter is
            // currently allowed to insert a literal newline so multi-
            // line roles still survive the round-trip even though the
            // single-line UX is the documented mode (per the attached
            // spec the TS textarea rarely exceeds one line — this just
            // matches MultiLineInput's behaviour without diverging).
            let input = Input::from(crossterm::event::KeyEvent::new(key.code, key.modifiers));
            let _ = self.textarea.input(input);
            return EventResult::consumed();
        }
        if let Event::Paste(s) = event {
            let _ = self.textarea.insert_str(s);
            return EventResult::consumed();
        }
        EventResult::ignored()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let sep = FOOTER_SEPARATOR;
        let footer = format!("Enter Save{sep}Ctrl+D Clear{sep}Esc Cancel");
        // Render the current draft as a single dialog row so the user
        // sees what they've typed. The selection-highlight inversion is
        // disabled by `selectable: false` — the row paints plain so
        // the textarea's cursor (`set_cursor_position`) can still be
        // placed over it by the parent frame if the App wants to.
        let draft = self.draft();
        let body_row = DialogRow {
            spans: vec![Span::raw(if draft.is_empty() {
                String::from(" ")
            } else {
                draft
            })],
            selectable: false,
            selected: false,
        };
        let dialog = FspecDialog {
            accent: Accent::Cyan,
            title: "Role",
            rows: vec![body_row],
            footer: footer.as_str(),
            min_width: 60,
        };
        render_dialog(area, buf, &dialog);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn new_empty_dialog_has_empty_draft() {
        let d = RoleDialog::new(SessionId::new("s-1"), None);
        assert_eq!(d.draft(), "");
        assert_eq!(d.id(), ROLE_DIALOG_ID);
        assert_eq!(d.priority(), Priority::Foreground);
    }

    #[test]
    fn new_seeded_dialog_pre_fills_draft() {
        let d = RoleDialog::new(SessionId::new("s-1"), Some("Hello".to_string()));
        assert_eq!(d.draft(), "Hello");
    }
}
