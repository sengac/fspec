//! WT-005 — SessionWorktreesDialog: centred listing of session worktrees
//! with a Prune action.
//!
//! Feature: spec/features/the-worktrees-slash-command-lists-session-worktrees-and-prunes-leaked-ones.feature
//!
//! Sibling of [`crate::views::agent::merge_confirm_dialog::MergeConfirmDialog`]:
//! it carries a `Vec<SessionWorktreeInfo>` payload (rendered one row per
//! worktree, dirty rows tagged `[dirty]`) and emits typed
//! [`SessionWorktreesDialogOutcome::Prune`] / [`Cancel`] variants that
//! the dispatch_worktrees layer routes into
//! `Action::PruneLeakedWorktrees` / `Action::CancelWorktreesDialog`.
//!
//! Renders via the shared `dialog_theme` renderer (rounded cyan border —
//! listing/inspection, like the role banner accent).

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;

use codelet_rpc_types::SessionWorktreeInfo;

use crate::components::dialog_theme::{
    render_dialog, Accent, DialogRow, FspecDialog, FOOTER_SEPARATOR,
};
use crate::components::{Action, Callback, Component, EventResult, Priority};

/// Outcome of routing a single key event through the SessionWorktreesDialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionWorktreesDialogOutcome {
    /// User activated the Prune button (default focus).
    Prune,
    /// User activated the Cancel button or pressed Esc.
    Cancel,
    /// Dialog handled the key internally (focus navigation).
    Continued,
    /// Dialog ignored the key — caller may route it elsewhere.
    Ignored,
}

/// Compositor stable id for the session-worktrees overlay.
pub const SESSION_WORKTREES_DIALOG_ID: &str = "session-worktrees-dialog";

/// A listing overlay that paints one row per session worktree (with
/// `[dirty]` markers) above two buttons (Prune, Cancel).
pub struct SessionWorktreesDialog {
    rows: Vec<SessionWorktreeInfo>,
    focused: usize,
    action_tx: Option<UnboundedSender<Action>>,
}

impl SessionWorktreesDialog {
    /// Construct a fresh dialog seeded with the backend's worktree rows.
    /// Focus starts on the Prune button (index 0).
    pub fn new(rows: Vec<SessionWorktreeInfo>) -> Self {
        Self {
            rows,
            focused: 0,
            action_tx: None,
        }
    }

    /// Optional builder hook — wire the App's action channel so the
    /// dialog can emit follow-up actions (Prune / Cancel) from key events.
    pub fn with_action_tx(mut self, action_tx: UnboundedSender<Action>) -> Self {
        self.action_tx = Some(action_tx);
        self
    }

    /// Read-only accessor for the wrapped rows.
    pub fn rows(&self) -> &[SessionWorktreeInfo] {
        &self.rows
    }

    /// Index of the currently focused button (0 = Prune, 1 = Cancel).
    pub fn focused_button(&self) -> usize {
        self.focused
    }

    fn focus_prev(&mut self) {
        if self.focused == 0 {
            self.focused = 1;
        } else {
            self.focused -= 1;
        }
    }

    fn focus_next(&mut self) {
        if self.focused + 1 >= 2 {
            self.focused = 0;
        } else {
            self.focused += 1;
        }
    }

    fn outcome_for_index(&self, idx: usize) -> SessionWorktreesDialogOutcome {
        match idx {
            0 => SessionWorktreesDialogOutcome::Prune,
            _ => SessionWorktreesDialogOutcome::Cancel,
        }
    }

    /// Route a single key event through the dialog.
    ///
    /// Esc emits `Cancel` regardless of focused button; Tab/Right cycle
    /// focus forward; Shift+Tab/Left cycle focus backward; Enter
    /// confirms the focused button.
    pub fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> SessionWorktreesDialogOutcome {
        if mods.contains(KeyModifiers::CONTROL) || mods.contains(KeyModifiers::ALT) {
            return SessionWorktreesDialogOutcome::Ignored;
        }
        match code {
            KeyCode::Esc => SessionWorktreesDialogOutcome::Cancel,
            KeyCode::Left => {
                self.focus_prev();
                SessionWorktreesDialogOutcome::Continued
            }
            KeyCode::Right => {
                self.focus_next();
                SessionWorktreesDialogOutcome::Continued
            }
            KeyCode::BackTab => {
                self.focus_prev();
                SessionWorktreesDialogOutcome::Continued
            }
            KeyCode::Tab => {
                if mods.contains(KeyModifiers::SHIFT) {
                    self.focus_prev();
                } else {
                    self.focus_next();
                }
                SessionWorktreesDialogOutcome::Continued
            }
            KeyCode::Enter => self.outcome_for_index(self.focused),
            _ => SessionWorktreesDialogOutcome::Ignored,
        }
    }

    fn emit_action(&self, action: Action) {
        if let Some(tx) = self.action_tx.as_ref() {
            let _ = tx.send(action);
        }
    }

    fn remove_callback(&self) -> Callback {
        Box::new(|compositor| {
            let _ = compositor.remove(SESSION_WORKTREES_DIALOG_ID);
        })
    }

    fn build_worktree_rows(&self) -> Vec<DialogRow> {
        if self.rows.is_empty() {
            return vec![DialogRow {
                spans: vec![Span::raw("(no session worktrees)".to_string())],
                selectable: false,
                selected: false,
            }];
        }
        self.rows
            .iter()
            .map(|w| {
                let marker = if w.dirty { "[dirty]" } else { "       " };
                DialogRow {
                    spans: vec![
                        Span::raw(format!("{marker} ")),
                        Span::raw(w.session_id.value.clone()),
                        Span::raw("  ".to_string()),
                        Span::styled(
                            w.worktree_path.clone(),
                            Style::default().add_modifier(Modifier::DIM),
                        ),
                    ],
                    selectable: false,
                    selected: false,
                }
            })
            .collect()
    }

    fn build_button_row(&self) -> DialogRow {
        let accent = Accent::Cyan.color();
        let labels = ["Prune", "Cancel"];
        let mut spans: Vec<Span<'static>> = Vec::new();
        for (i, label) in labels.iter().enumerate() {
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

    /// Render the dialog as a centred overlay inside `area`.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let spacer = DialogRow {
            spans: vec![Span::raw(String::new())],
            selectable: false,
            selected: false,
        };
        let mut rows = self.build_worktree_rows();
        rows.push(spacer);
        rows.push(self.build_button_row());
        let dialog = FspecDialog {
            accent: Accent::Cyan,
            title: "Session Worktrees",
            rows,
            footer: "Tab / ←→: focus  Enter: confirm  Esc: cancel",
            min_width: 50,
            query_row: None,
        };
        render_dialog(area, buf, &dialog);
    }
}

// ─────────────────────────────────────────────────────────────────────
// Component impl so the dialog can be pushed onto the App's compositor
// with a stable id and Foreground priority (mirrors MergeConfirmDialog).
// ─────────────────────────────────────────────────────────────────────

impl Component for SessionWorktreesDialog {
    fn id(&self) -> &str {
        SESSION_WORKTREES_DIALOG_ID
    }

    fn priority(&self) -> Priority {
        Priority::Foreground
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        let key = match event {
            Event::Key(k) => *k,
            _ => return EventResult::ignored(),
        };
        if key.kind != crossterm::event::KeyEventKind::Press {
            return EventResult::ignored();
        }
        match self.handle_key(key.code, key.modifiers) {
            SessionWorktreesDialogOutcome::Prune => {
                self.emit_action(Action::PruneLeakedWorktrees);
                EventResult::Consumed(Some(self.remove_callback()))
            }
            SessionWorktreesDialogOutcome::Cancel => {
                self.emit_action(Action::CancelWorktreesDialog);
                EventResult::Consumed(Some(self.remove_callback()))
            }
            SessionWorktreesDialogOutcome::Continued => EventResult::consumed(),
            SessionWorktreesDialogOutcome::Ignored => EventResult::ignored(),
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        SessionWorktreesDialog::render(self, area, buf);
    }
}
