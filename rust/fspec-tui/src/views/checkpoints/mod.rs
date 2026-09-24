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
    /// Boxed — see `BlocklistEvent::Emit` for the `large_enum_variant`
    /// rationale.
    Emit(Box<crate::components::Action>),
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
        // BUG-184: the view now displays this list — record its identity
        // so a no-op git-state refresh can be dropped before it spawns a
        // re-fetch (see `git_state.rs` R1).
        self.load
            .set_list_signature(&list_signature(&self.checkpoints));
        self.sync_loading_label();
    }

    /// Replace the checkpoint list from a git-state refresh, keeping the
    /// existing selection stable by (work_unit_id, name). The previously
    /// selected checkpoint is re-looked-up in the fresh list; when it no
    /// longer exists (checkpoint deleted) the selection falls back to the
    /// first row. Dependent files + diff are cleared so the App re-loads
    /// the cascade for the (re-)selected checkpoint.
    ///
    /// BUG-184: the checkpoint-list scroll position is PRESERVED as an
    /// offset and clamped to the fresh list length (max =
    /// `new_len - visible`; a list shorter than the window falls back
    /// to the top). The selection follows the preserved window: when
    /// the re-looked-up row falls outside the visible window it moves
    /// to the window's anchor row (first visible for a fallback, last
    /// visible when it slid past the bottom) — so the highlighted row
    /// and the scroll window never diverge (same anchor rule
    /// `move_checkpoint_selection`'s `ensure_visible` implements).
    ///
    /// Never touches the load tracker — a refresh of a LOADED view must
    /// not re-open the loading dialog.
    pub fn refresh_checkpoints_preserving_selection(&mut self, checkpoints: Vec<CheckpointInfo>) {
        let previous = self
            .selected_checkpoint_info()
            .map(|c| (c.work_unit_id.clone(), c.name.clone()));
        let mut checkpoints = checkpoints;
        for checkpoint in checkpoints.iter_mut() {
            checkpoint.work_unit_id = sanitize_for_terminal(&checkpoint.work_unit_id);
            checkpoint.name = sanitize_for_terminal(&checkpoint.name);
        }
        self.checkpoints = checkpoints;
        let new_len = self.checkpoints.len();
        self.selected_checkpoint = previous
            .as_ref()
            .and_then(|(w, n)| {
                self.checkpoints
                    .iter()
                    .position(|c| &c.work_unit_id == w && &c.name == n)
            })
            .unwrap_or(0);
        // BUG-184: preserve the scroll offset, clamped to the fresh list.
        // (Before: the window reset to 0 whenever the re-selected row sat
        // below the old window — the 10s refresh jolted the list up.)
        let old_window = self
            .last_checkpoints_rect
            .map(|r| r.height as usize)
            .filter(|h| *h > 0);
        // BUG-184: preserve the scroll offset, clamped to the fresh list.
        // (Before: the window reset to 0 whenever the re-selected row sat
        // below the old window — the 10s refresh jolted the list up.)
        // Selection/scroll consistency: the re-selected row must lie
        // inside the preserved window, otherwise the window's anchor row
        // (first visible when the selection falls above, last visible
        // when it slides past the bottom) takes it.
        match old_window {
            Some(visible) if visible > 0 => {
                if new_len > visible {
                    let max_scroll = new_len - visible;
                    self.checkpoint_scroll = self.checkpoint_scroll.min(max_scroll);
                    if self.selected_checkpoint < self.checkpoint_scroll {
                        self.selected_checkpoint = self.checkpoint_scroll;
                    } else if self.selected_checkpoint >= self.checkpoint_scroll + visible {
                        self.selected_checkpoint = self.checkpoint_scroll + visible - 1;
                        self.checkpoint_scroll = self.selected_checkpoint + 1 - visible;
                    }
                } else {
                    // List shorter than the window: fall back to the top.
                    self.checkpoint_scroll = 0;
                    self.selected_checkpoint = 0;
                }
            }
            _ => {
                self.checkpoint_scroll = 0;
            }
        }
        self.clear_files();
        // BUG-184: the fresh files/diff re-load for the (re-)selected
        // checkpoint replaces the files by the SAME key — the Files pane
        // scroll offset survives it (set_files only resets the offset on
        // a key CHANGE; the diff is clamped, not zeroed, on re-load).
        self.sync_loading_label();
    }

    /// Test/R6 seam: move the checkpoint selection to an explicit index
    /// (clamped).
    pub fn set_selected_checkpoint(&mut self, index: usize) {
        if !self.checkpoints.is_empty() {
            self.selected_checkpoint = index.min(self.checkpoints.len().saturating_sub(1));
        }
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
    ///
    /// BUG-184: a re-load of the SAME key (the git-state refresh cascade
    /// re-fetches the (re-)selected checkpoint's files) preserves the
    /// Files-pane scroll offset; only a key CHANGE (a different
    /// checkpoint was selected) resets it.
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
        let same_key = self.files_key.as_ref() == Some(&(work_unit_id.clone(), name.clone()));
        self.files = files;
        if !same_key {
            self.selected_file = 0;
            self.file_scroll = 0;
            // A different checkpoint was selected: its diff is a different
            // document — drop the cached diff so the cascade re-loads it.
            self.clear_diff();
        } else {
            // BUG-184: same-key re-load — keep the Files-pane point,
            // clamped to the fresh list (the list may have shrunk
            // beneath the preserved selection/window). The diff is NOT
            // cleared: the App re-fetches the (re-)selected file's diff
            // and it lands via `set_diff`'s same-key path (which
            // preserves the Diff-pane scroll).
            let len = self.files.len();
            self.selected_file = self.selected_file.min(len.saturating_sub(1));
            let visible = self
                .last_files_rect
                .map(|r| r.height as usize)
                .filter(|h| *h > 0);
            match visible {
                Some(v) if v > 0 && len > v => {
                    self.file_scroll = self.file_scroll.min(len - v);
                }
                _ => {
                    self.file_scroll = 0;
                }
            }
            if let Some(v) = visible {
                if v > 0 {
                    if self.selected_file < self.file_scroll {
                        self.selected_file = self.file_scroll;
                    } else if self.selected_file >= self.file_scroll + v {
                        self.selected_file = self.file_scroll + v - 1;
                        self.file_scroll = self.selected_file + 1 - v;
                    }
                }
            }
        }
        self.files_key = Some((work_unit_id, name));
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
        // BUG-184: a re-load of the SAME key (the git-state refresh
        // cascade re-fetches the selected file's diff) preserves the
        // Diff-pane scroll offset (clamped to the fresh diff length);
        // only a key CHANGE (a different checkpoint/file) resets it.
        let same_key =
            self.diff_key.as_ref() == Some(&(work_unit_id.clone(), name.clone(), path.clone()));
        self.diff_key = Some((work_unit_id, name, path));
        self.diff_lines = match diff {
            Some(text) if !text.is_empty() => text.split('\n').map(sanitize_for_terminal).collect(),
            _ => vec!["No changes to display".to_string()],
        };
        if same_key {
            let viewport = self
                .last_diff_rect
                .map(|r| r.height as usize)
                .filter(|h| *h > 0);
            let max_scroll = match viewport {
                Some(v) if self.diff_lines.len() > v => self.diff_lines.len() - v,
                _ => 0,
            };
            self.diff_scroll = self.diff_scroll.min(max_scroll);
        } else {
            self.diff_scroll = 0;
        }
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

    /// The checkpoint-list scroll offset (test seam + the BUG-184
    /// refresh clamp).
    pub fn checkpoint_scroll(&self) -> usize {
        self.checkpoint_scroll
    }

    /// The Files-pane scroll offset (BUG-184 test seam).
    pub fn file_scroll(&self) -> usize {
        self.file_scroll
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

    /// BUG-184: the displayed-list identity the refresh dedup compares
    /// against — one `work_unit_id/name:auto` entry per checkpoint (in
    /// list order; the timestamp leg is excluded — rows never render
    /// it).
    pub(crate) fn list_signature(&self) -> String {
        list_signature(&self.checkpoints)
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

/// BUG-184: identity of the checkpoint list as DISPLAYED — one
/// `work_unit_id/name:auto` entry per checkpoint (in list order). The
/// row rendering (`checkpoint_label`) depends only on work_unit_id +
/// name (+ the auto flag), NOT the timestamp — so the frame's timestamp
/// leg (a fallback "now" stamp when no index sidecar exists, which
/// churns on every watcher capture) must NOT participate: two frames
/// whose lists render byte-identically are the same displayed list, and
/// the refresh path skips re-fetching them. Order participates because
/// a re-ordered list IS a visible change.
fn list_signature(checkpoints: &[CheckpointInfo]) -> String {
    checkpoints
        .iter()
        .map(|c| {
            format!(
                "{}/{}:{}",
                c.work_unit_id,
                c.name,
                if c.is_automatic { "auto" } else { "manual" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
