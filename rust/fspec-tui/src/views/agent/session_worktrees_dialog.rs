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
//!
//! TUI-112: `Component::handle_event` (keys → `handle_key`, left-clicks
//! → R2 button commit) lives in
//! `super::session_worktrees_dialog_dispatch` so this file stays under
//! the 300-LoC ceiling.
//!
//! TUI-131: each worktree's full path wraps on the rows under its session
//! row (`rows`); when the list is taller than the screen it scrolls
//! (Up/Down, PgUp/PgDn, Home/End, the wheel move the chosen worktree and
//! the window follows it) while the buttons and the footer stay visible.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;

use codelet_rpc_types::SessionWorktreeInfo;

use crate::components::dialog_button_hits::{separated_button_layout, LastLayout};
use crate::components::dialog_theme::{
    render_dialog, Accent, DialogRow, FspecDialog, FOOTER_SEPARATOR,
};
use crate::components::dialog_theme_rows::dialog_body_width;
use crate::components::Action;

mod rows;

/// Rows the dialog takes besides its worktree list: border (2), padding
/// (2), title (1) and its gap (1), the spacer and button rows (2), the
/// gap before the footer (1) and the footer (1).
const CHROME_ROWS: u16 = 10;

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
    /// TUI-130: the worktree row Up/Down choose (c copies its path).
    chosen: usize,
    /// TUI-131: the first list row painted, and the worktrees a page
    /// (PgUp/PgDn) moves by, both from the last render.
    offset: usize,
    page: usize,
    action_tx: Option<UnboundedSender<Action>>,
    /// TUI-112: the most recent emitted action, stashed for tests
    /// (drained via `take_pending_action`) when no `action_tx` is set.
    pending_action: Option<Action>,
    /// TUI-112: last-rendered button-row geometry for left-click
    /// hit-testing (R2: a click on a button returns that outcome).
    last_layout: LastLayout,
}

impl SessionWorktreesDialog {
    /// Construct a fresh dialog seeded with the backend's worktree rows.
    /// Focus starts on the Prune button (index 0).
    pub fn new(rows: Vec<SessionWorktreeInfo>) -> Self {
        Self {
            rows,
            focused: 0,
            chosen: 0,
            offset: 0,
            page: 1,
            action_tx: None,
            pending_action: None,
            last_layout: LastLayout::new(),
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

    /// TUI-112: test accessor — the last-rendered button-row geometry
    /// (frame rect is `None` before the first render, R4).
    pub fn last_layout(&self) -> &LastLayout {
        &self.last_layout
    }

    /// Test-only: drain the most recent emitted action stashed by
    /// `handle_event` when no `action_tx` was attached.
    /// TUI-130: the full path of the chosen worktree row.
    pub fn chosen_path(&self) -> Option<String> {
        self.rows.get(self.chosen).map(|w| w.worktree_path.clone())
    }

    pub fn take_pending_action(&mut self) -> Option<Action> {
        self.pending_action.take()
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

    /// TUI-112: the outcome a button index maps to (shared by the Enter
    /// key path and the left-click path).
    pub(crate) fn outcome_for_index(&self, idx: usize) -> SessionWorktreesDialogOutcome {
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
    pub fn handle_key(
        &mut self,
        code: KeyCode,
        mods: KeyModifiers,
    ) -> SessionWorktreesDialogOutcome {
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
            // TUI-130: Up/Down choose the worktree row c copies.
            KeyCode::Up => self.choose(self.chosen.saturating_sub(1)),
            KeyCode::Down => self.choose(self.chosen + 1),
            // TUI-131: the list scrolls with the chosen worktree.
            KeyCode::PageUp => self.choose(self.chosen.saturating_sub(self.page)),
            KeyCode::PageDown => self.choose(self.chosen + self.page),
            KeyCode::Home => self.choose(0),
            KeyCode::End => self.choose(self.rows.len()),
            KeyCode::Enter => self.outcome_for_index(self.focused),
            _ => SessionWorktreesDialogOutcome::Ignored,
        }
    }

    /// TUI-131: choose worktree `index` (clamped to the list).
    pub(crate) fn choose(&mut self, index: usize) -> SessionWorktreesDialogOutcome {
        self.chosen = index.min(self.rows.len().saturating_sub(1));
        SessionWorktreesDialogOutcome::Continued
    }

    /// TUI-112: emit an action on the App's channel (when attached) and
    /// stash it for tests.
    pub(crate) fn emit_action(&mut self, action: &Action) {
        if let Some(tx) = self.action_tx.as_ref() {
            let _ = tx.send(action.clone());
        }
        self.pending_action = Some(action.clone());
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
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let spacer = DialogRow {
            spans: vec![Span::raw(String::new())],
            selectable: false,
            selected: false,
        };
        // TUI-131: the list gets the rows the area leaves and scrolls
        // when it is taller (the window follows the chosen worktree).
        let visible = usize::from(area.height.saturating_sub(CHROME_ROWS)).max(1);
        let entries = rows::worktree_entries(&self.rows, self.chosen, dialog_body_width(area));
        let tallest = entries.iter().map(Vec::len).max().unwrap_or(1).max(1);
        self.page = (visible / tallest).max(1);
        let (mut rows, offset) = rows::list_window(entries, self.chosen, self.offset, visible);
        self.offset = offset;
        rows.push(spacer);
        rows.push(self.build_button_row());
        let dialog = FspecDialog {
            accent: Accent::Cyan,
            title: "Session Worktrees",
            rows,
            footer: "↑↓: worktree  c: copy path  Tab / ←→: focus  Enter: confirm  Esc: cancel",
            min_width: 50,
            query_row: None,
        };
        // TUI-112: cache the button-row geometry (the LAST content row)
        // for left-click hit-testing, derived from the SAME descriptor
        // that is painted so the rects line up with the pixels.
        let labels = ["Prune", "Cancel"];
        let button_row_index = dialog.rows.len() - 1;
        self.last_layout = separated_button_layout(area, &dialog, button_row_index, &labels);
        render_dialog(area, buf, &dialog);
    }
}
