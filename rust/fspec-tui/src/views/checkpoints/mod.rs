//! RPC-364 — `CheckpointsView` three-pane state + event handling.
//!
//! Feature: spec/features/rust-checkpoints-view.feature
//!
//! A full-screen mode-view (entered via the board `C` key →
//! `Action::OpenCheckpointsView`) with THREE panes: a Checkpoints list, a
//! Files list, and a unified Diff pane, plus a focus state machine
//! (Checkpoints→Files→Diff). Modeled on `ChangedFilesView` (RPC-356);
//! reuses the shared `diff_common` helpers (RPC-363) and
//! `scroll_viewport` scroll math. Owned by `Navigator` via
//! `ViewMode::Checkpoints`. Browse + diff only — restore/delete land in
//! RPC-365/366.

use codelet_rpc_types::{ChangedFile, CheckpointInfo};
use crossterm::event::Event;
use ratatui::layout::Rect;

use crate::components::load_state::LoadTracker;
use crate::components::loading_dialog::LoadingDialog;
use crate::components::scroll_viewport::{WheelDirection, WheelVelocity};
use crate::terminal::sanitize::sanitize_for_terminal;

mod checkpoint_row;
mod delete;
mod delete_dialog;
mod dialog;
mod keys;
mod navigation;
mod render;
mod restore;

pub use checkpoint_row::checkpoint_label;
pub use delete_dialog::{DeleteDialog, DeletePhase, DeleteTarget};
pub use dialog::{DialogPhase, RestoreDialog, RestoreTarget};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "restore_tests.rs"]
mod restore_tests;

#[cfg(test)]
#[path = "delete_tests.rs"]
mod delete_tests;

/// Which pane currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pane {
    #[default]
    Checkpoints,
    Files,
    Diff,
}

/// Outcome of routing a single event through the view. Mirrors
/// `ChangedFilesEvent`: `Emit(Action)` is how the view asks the App to
/// fold state (e.g. a lazy file/diff load) via the dispatcher.
#[derive(Debug, Clone)]
pub enum CheckpointsEvent {
    Consumed,
    Ignored,
    Close,
    Emit(crate::components::Action),
}

/// Three-pane checkpoints browser state.
pub struct CheckpointsView {
    checkpoints: Vec<CheckpointInfo>,
    selected_checkpoint: usize,
    checkpoint_scroll: usize,
    files: Vec<ChangedFile>,
    selected_file: usize,
    file_scroll: usize,
    /// Diff lines for the currently-selected file (split on `\n`).
    diff_lines: Vec<String>,
    diff_scroll: usize,
    /// `(work_unit_id, name)` key the current `files` belong to (stale
    /// `CheckpointFilesLoaded` for a different checkpoint is dropped).
    files_key: Option<(String, String)>,
    /// `(work_unit_id, name, path)` key the current `diff_lines` belong
    /// to (stale `CheckpointFileDiffLoaded` is dropped).
    diff_key: Option<(String, String, String)>,
    focused_pane: Pane,
    wheel: WheelVelocity,
    last_checkpoints_rect: Option<Rect>,
    last_files_rect: Option<Rect>,
    last_diff_rect: Option<Rect>,
    /// TUI-101: scrollbar click-and-drag state machines (one per pane).
    cp_scrollbar_drag: crate::mouse::scrollbar_drag::ScrollbarDrag,
    files_scrollbar_drag: crate::mouse::scrollbar_drag::ScrollbarDrag,
    diff_scrollbar_drag: crate::mouse::scrollbar_drag::ScrollbarDrag,
    /// TUI-101: cached scrollbar rects from last render for hit-testing.
    last_cp_sb_rect: Option<Rect>,
    last_files_sb_rect: Option<Rect>,
    last_diff_sb_rect: Option<Rect>,
    /// RPC-365: the active restore confirmation/status modal, if any.
    /// While `Some`, key events are captured by the dialog.
    restore_dialog: Option<dialog::RestoreDialog>,
    /// RPC-366: the active delete confirmation/status modal, if any.
    /// While `Some`, key events are captured by the dialog.
    delete_dialog: Option<delete_dialog::DeleteDialog>,
    /// TUI-106: shared animated loading dialog (Pattern B, view-owned).
    /// Mount = Some on open; dismiss (TUI-107 wiring) once `load` idles.
    pub loading: LoadingDialog,
    /// TUI-106: staged in-flight cascade tracker (list → files → diff).
    /// Fed by the App dispatchers; drives `is_loading()` + the dialog.
    pub load: LoadTracker,
    /// TUI-107: when the view was opened — the spinner's elapsed-ms
    /// origin (the run loop owns the clock; the view only reports state).
    loading_started: std::time::Instant,
}

impl Default for CheckpointsView {
    fn default() -> Self {
        Self::new()
    }
}

impl CheckpointsView {
    pub fn new() -> Self {
        Self {
            checkpoints: Vec::new(),
            selected_checkpoint: 0,
            checkpoint_scroll: 0,
            files: Vec::new(),
            selected_file: 0,
            file_scroll: 0,
            diff_lines: Vec::new(),
            diff_scroll: 0,
            files_key: None,
            diff_key: None,
            focused_pane: Pane::Checkpoints,
            wheel: WheelVelocity::new(),
            last_checkpoints_rect: None,
            last_files_rect: None,
            last_diff_rect: None,
            cp_scrollbar_drag: crate::mouse::scrollbar_drag::ScrollbarDrag::new(),
            files_scrollbar_drag: crate::mouse::scrollbar_drag::ScrollbarDrag::new(),
            diff_scrollbar_drag: crate::mouse::scrollbar_drag::ScrollbarDrag::new(),
            last_cp_sb_rect: None,
            last_files_sb_rect: None,
            last_diff_sb_rect: None,
            restore_dialog: None,
            delete_dialog: None,
            loading: LoadingDialog::new("Loading checkpoints", "Loading checkpoint list…"),
            load: LoadTracker::new("Loading checkpoint list…"),
            loading_started: std::time::Instant::now(),
        }
    }

    /// TUI-107: elapsed milliseconds since the view opened — the
    /// spinner-frame origin for the loading dialog (the 16 ms tick
    /// repaints the view; the glyph advances at 80 ms cadence).
    pub fn loading_elapsed_ms(&self) -> u64 {
        self.loading_started.elapsed().as_millis() as u64
    }

    /// TUI-106: true while any cascade stage (list → files → diff) is
    /// in flight. The loading dialog paints while this is true; the
    /// real empty state ("No checkpoints available") only surfaces
    /// AFTER the list flushes AND the cascade has settled.
    pub fn is_loading(&self) -> bool {
        self.load.is_loading()
    }

    /// TUI-106: copy the tracker's active stage label onto the mounted
    /// loading dialog so the dialog names what is loading. Called by
    /// the App dispatchers after every tracker transition.
    pub fn sync_loading_label(&mut self) {
        if let Some(label) = self.load.active_label() {
            self.loading.label = label;
        }
    }

    /// Replace the checkpoint list from `Action::CheckpointsLoaded`.
    /// Resets selection/scroll and clears dependent files + diff.
    ///
    /// TUI-107: also flushes the list stage on the tracker (the load may
    /// be empty — a failed load degrades to the real empty state). The
    /// App dispatcher keeps its own `mark_list_flushed` call for the
    /// files-stage hand-off; the tracker's flag is idempotent.
    ///
    /// **TUI-111**: `work_unit_id` + `name` are sanitized on ingress so
    /// the checkpoint label rows and the restore dialog always paint
    /// clean text.
    pub fn set_checkpoints(&mut self, checkpoints: Vec<CheckpointInfo>) {
        let mut checkpoints = checkpoints;
        for checkpoint in checkpoints.iter_mut() {
            checkpoint.work_unit_id = sanitize_for_terminal(&checkpoint.work_unit_id);
            checkpoint.name = sanitize_for_terminal(&checkpoint.name);
        }
        self.checkpoints = checkpoints;
        self.selected_checkpoint = 0;
        self.checkpoint_scroll = 0;
        self.clear_files();
        self.load.mark_list_flushed();
        self.sync_loading_label();
    }

    fn clear_files(&mut self) {
        self.files.clear();
        self.selected_file = 0;
        self.file_scroll = 0;
        self.files_key = None;
        self.clear_diff();
    }

    fn clear_diff(&mut self) {
        self.diff_lines.clear();
        self.diff_scroll = 0;
        self.diff_key = None;
    }

    /// Fold a `CheckpointFilesLoaded` response. Ignored when the loaded
    /// key no longer matches the selected checkpoint (stale async result).
    ///
    /// **TUI-111**: file paths + change types are sanitized on ingress
    /// (the key parameters are sanitized too so the stale-drop match
    /// against the stored sanitized list lines up).
    pub fn set_files(&mut self, work_unit_id: &str, name: &str, files: Vec<ChangedFile>) {
        let work_unit_id = sanitize_for_terminal(work_unit_id);
        let name = sanitize_for_terminal(name);
        if !self.selection_matches(&work_unit_id, &name) {
            return;
        }
        let mut files = files;
        for file in files.iter_mut() {
            file.path = sanitize_for_terminal(&file.path);
            file.change_type = sanitize_for_terminal(&file.change_type);
        }
        self.files = files;
        self.selected_file = 0;
        self.file_scroll = 0;
        self.files_key = Some((work_unit_id, name));
        self.clear_diff();
    }

    /// Fold a `CheckpointFileDiffLoaded` response. Ignored when the key
    /// no longer matches the selected checkpoint + file.
    ///
    /// **TUI-111**: diff lines are sanitized on ingress.
    pub fn set_diff(&mut self, work_unit_id: &str, name: &str, path: &str, diff: Option<String>) {
        let work_unit_id = sanitize_for_terminal(work_unit_id);
        let name = sanitize_for_terminal(name);
        let path = sanitize_for_terminal(path);
        if !self.selection_matches(&work_unit_id, &name) {
            return;
        }
        if self.selected_file_path().as_deref() != Some(path.as_str()) {
            return;
        }
        self.diff_key = Some((work_unit_id, name, path));
        self.diff_scroll = 0;
        self.diff_lines = match diff {
            Some(text) if !text.is_empty() => text.split('\n').map(sanitize_for_terminal).collect(),
            _ => vec!["No changes to display".to_string()],
        };
    }

    fn selection_matches(&self, work_unit_id: &str, name: &str) -> bool {
        self.selected_checkpoint_info()
            .map(|c| c.work_unit_id == work_unit_id && c.name == name)
            .unwrap_or(false)
    }

    pub fn selected_checkpoint_info(&self) -> Option<&CheckpointInfo> {
        self.checkpoints.get(self.selected_checkpoint)
    }

    pub(super) fn selected_file_path(&self) -> Option<String> {
        self.files.get(self.selected_file).map(|f| f.path.clone())
    }

    /// RPC-365: number of files in the currently-loaded checkpoint (used
    /// by the restore-all confirmation copy).
    pub(super) fn file_count(&self) -> usize {
        self.files.len()
    }

    /// The repo-relative path of the first file (the freshly-selected one
    /// after a `set_files`), used by the dispatcher to kick off the
    /// initial diff load.
    pub fn first_file_path(&self) -> Option<String> {
        self.files.first().map(|f| f.path.clone())
    }

    pub fn focused_pane(&self) -> Pane {
        self.focused_pane
    }

    pub fn selected_checkpoint(&self) -> usize {
        self.selected_checkpoint
    }

    pub fn selected_file(&self) -> usize {
        self.selected_file
    }

    pub fn diff_scroll(&self) -> usize {
        self.diff_scroll
    }

    pub fn is_empty(&self) -> bool {
        self.checkpoints.is_empty()
    }

    /// TUI-109: number of checkpoints folded into the view (test seam).
    pub fn checkpoints_len(&self) -> usize {
        self.checkpoints.len()
    }

    /// RPC-365: borrow the active restore dialog, if any. Used by the
    /// renderer (to paint the modal over the panes) and by tests.
    pub fn dialog(&self) -> Option<&dialog::RestoreDialog> {
        self.restore_dialog.as_ref()
    }

    /// Route a key or mouse event. Returns a `CheckpointsEvent` the
    /// Navigator translates onto the action bus.
    pub fn handle_event(&mut self, event: &Event) -> CheckpointsEvent {
        match event {
            Event::Key(key) => self.handle_key(*key),
            Event::Mouse(mouse) => self.handle_mouse(*mouse),
            _ => CheckpointsEvent::Ignored,
        }
    }

    fn pane_at(&self, col: u16, row: u16) -> Option<Pane> {
        let inside = |r: &Rect| {
            col >= r.x
                && col < r.x.saturating_add(r.width)
                && row >= r.y
                && row < r.y.saturating_add(r.height)
        };
        if self.last_diff_rect.as_ref().map(inside).unwrap_or(false) {
            return Some(Pane::Diff);
        }
        if self.last_files_rect.as_ref().map(inside).unwrap_or(false) {
            return Some(Pane::Files);
        }
        if self
            .last_checkpoints_rect
            .as_ref()
            .map(inside)
            .unwrap_or(false)
        {
            return Some(Pane::Checkpoints);
        }
        None
    }

    /// RPC-369: focus the pane under a mouse click.
    pub(super) fn set_focused_pane(&mut self, pane: Pane) {
        self.focused_pane = pane;
    }

    /// RPC-369: map a clicked screen `row` to a list index for the
    /// Checkpoints (`checkpoints = true`) or Files pane, using that pane's
    /// cached CONTENT rect and scroll offset. Returns `None` when the rect
    /// is unknown or the click lands past the last populated row.
    pub(super) fn row_target(&self, row: u16, checkpoints: bool) -> Option<usize> {
        let (rect, scroll, len) = if checkpoints {
            (
                self.last_checkpoints_rect,
                self.checkpoint_scroll,
                self.checkpoints.len(),
            )
        } else {
            (self.last_files_rect, self.file_scroll, self.files.len())
        };
        let rect = rect?;
        let offset = row.saturating_sub(rect.y) as usize;
        if offset >= len.saturating_sub(scroll) {
            return None;
        }
        Some(scroll + offset)
    }

    /// Advance the wheel velocity model and return the resulting scroll
    /// step. Wrapper so `keys.rs` need not name `WheelVelocity`/dir.
    fn wheel_step(&mut self, dir: WheelDirection) -> i32 {
        self.wheel.step(dir)
    }
}
