//! RPC-057 — MergeConfirmDialog: centred merge/discard confirmation overlay.
//!
//! Feature: spec/features/rpc057-merge-worktree-dispatch.feature
//!
//! Sibling of [`crate::views::agent::confirm_dialog::ConfirmDialog`] but
//! purpose-built for the /merge-worktree flow: it carries a
//! [`SessionChangesSummary`] payload (rendered inline) and emits typed
//! [`MergeConfirmDialogOutcome::Merge`] / [`Discard`] / [`Cancel`]
//! variants that the dispatch_merge_worktree layer routes into
//! `Action::MergeConfirmed` / `DiscardConfirmed` / `CancelMergeDialog`.
//!
//! Renders via the shared `dialog_theme` renderer (rounded yellow border,
//! bold yellow inner title, opaque black background, inverse highlight
//! on the focused button) for consistency with the other RPC-026 /
//! RPC-027 confirmation overlays.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use codelet_rpc_types::{SessionChangesSummary, SessionId};

use crate::components::dialog_theme::{
    render_dialog, Accent, DialogRow, FspecDialog, FOOTER_SEPARATOR,
};

/// Outcome of routing a single key event through the MergeConfirmDialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeConfirmDialogOutcome {
    /// User activated the Merge button (default focus).
    Merge { session_id: SessionId },
    /// User activated the Discard button.
    Discard { session_id: SessionId },
    /// User activated the Cancel button or pressed Esc.
    Cancel,
    /// Dialog handled the key internally (focus navigation).
    Continued,
    /// Dialog ignored the key — caller may route it elsewhere.
    Ignored,
}

/// Compositor stable id for the merge-confirm overlay.
pub const MERGE_CONFIRM_DIALOG_ID: &str = "merge-confirm-dialog";

/// A merge/discard confirmation overlay that paints a
/// [`SessionChangesSummary`] inline above three buttons
/// (Merge, Discard, Cancel).
pub struct MergeConfirmDialog {
    session_id: SessionId,
    summary: SessionChangesSummary,
    focused: usize,
}

impl MergeConfirmDialog {
    /// Construct a fresh dialog. Focus starts on the Merge button (index 0).
    pub fn new(session_id: SessionId, summary: SessionChangesSummary) -> Self {
        Self {
            session_id,
            summary,
            focused: 0,
        }
    }

    /// Read-only accessor for the wrapped session id.
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// Read-only accessor for the wrapped summary.
    pub fn summary(&self) -> &SessionChangesSummary {
        &self.summary
    }

    /// Index of the currently focused button (0 = Merge, 1 = Discard,
    /// 2 = Cancel).
    pub fn focused_button(&self) -> usize {
        self.focused
    }

    fn focus_prev(&mut self) {
        if self.focused == 0 {
            self.focused = 2;
        } else {
            self.focused -= 1;
        }
    }

    fn focus_next(&mut self) {
        if self.focused + 1 >= 3 {
            self.focused = 0;
        } else {
            self.focused += 1;
        }
    }

    fn outcome_for_index(&self, idx: usize) -> MergeConfirmDialogOutcome {
        match idx {
            0 => MergeConfirmDialogOutcome::Merge {
                session_id: self.session_id.clone(),
            },
            1 => MergeConfirmDialogOutcome::Discard {
                session_id: self.session_id.clone(),
            },
            _ => MergeConfirmDialogOutcome::Cancel,
        }
    }

    /// Route a single key event through the dialog.
    ///
    /// Esc emits `Cancel` regardless of focused button; Tab/Right cycle
    /// focus forward; Shift+Tab/Left cycle focus backward; Enter
    /// confirms the focused button.
    pub fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> MergeConfirmDialogOutcome {
        if mods.contains(KeyModifiers::CONTROL) || mods.contains(KeyModifiers::ALT) {
            return MergeConfirmDialogOutcome::Ignored;
        }
        match code {
            KeyCode::Esc => MergeConfirmDialogOutcome::Cancel,
            KeyCode::Left => {
                self.focus_prev();
                MergeConfirmDialogOutcome::Continued
            }
            KeyCode::Right => {
                self.focus_next();
                MergeConfirmDialogOutcome::Continued
            }
            KeyCode::BackTab => {
                self.focus_prev();
                MergeConfirmDialogOutcome::Continued
            }
            KeyCode::Tab => {
                if mods.contains(KeyModifiers::SHIFT) {
                    self.focus_prev();
                } else {
                    self.focus_next();
                }
                MergeConfirmDialogOutcome::Continued
            }
            KeyCode::Enter => self.outcome_for_index(self.focused),
            _ => MergeConfirmDialogOutcome::Ignored,
        }
    }

    fn build_summary_row(&self) -> DialogRow {
        let files_word = if self.summary.files_changed == 1 {
            "file"
        } else {
            "files"
        };
        let text = format!(
            "{} {files_word} changed, +{} / -{}, {} commit{}",
            self.summary.files_changed,
            self.summary.insertions,
            self.summary.deletions,
            self.summary.commits.len(),
            if self.summary.commits.len() == 1 {
                ""
            } else {
                "s"
            }
        );
        DialogRow {
            spans: vec![Span::raw(text)],
            selectable: false,
            selected: false,
        }
    }

    fn build_button_row(&self) -> DialogRow {
        let accent = Accent::Yellow.color();
        let labels = ["Merge", "Discard", "Cancel"];
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

    /// Render the dialog as a centred overlay inside `area`. Uses the
    /// shared dialog_theme renderer for the rounded yellow border +
    /// black background + bold inner title.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let spacer = DialogRow {
            spans: vec![Span::raw(String::new())],
            selectable: false,
            selected: false,
        };
        let dialog = FspecDialog {
            accent: Accent::Yellow,
            title: "Merge Worktree",
            rows: vec![self.build_summary_row(), spacer, self.build_button_row()],
            footer: "Tab / ←→: focus  Enter: confirm  Esc: cancel",
            min_width: 50,
            query_row: None,
        };
        render_dialog(area, buf, &dialog);
    }
}

// ─────────────────────────────────────────────────────────────────────
// Component impl so the dialog can be pushed onto the App's compositor
// with a stable id and Foreground priority. The dialog's `handle_key`
// is invoked from the App's keyboard-routing layer (dispatch_merge_worktree);
// here we expose only the id/priority/render shape that the
// `Compositor` cares about.
// ─────────────────────────────────────────────────────────────────────

use crate::components::{Component, Priority};

impl Component for MergeConfirmDialog {
    fn id(&self) -> &str {
        MERGE_CONFIRM_DIALOG_ID
    }

    fn priority(&self) -> Priority {
        Priority::Foreground
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        MergeConfirmDialog::render(self, area, buf);
    }
}
