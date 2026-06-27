//! RPC-356 — `ChangedFilesView` dual-pane state + event handling.
//!
//! Feature: spec/features/rust-changed-files-view.feature
//!
//! A full-screen mode-view (entered via the board `F` key →
//! `Action::OpenChangedFilesView`) that shows a scrollable list of
//! changed files on the left and the unified diff of the selected file
//! on the right. Mirrors the TS `ChangedFilesViewer` + `FileDiffViewer`
//! behaviour. Owned by `Navigator` via `ViewMode::ChangedFiles`.
//!
//! Split across sibling modules to stay under the 300-LoC source-shape
//! ceiling: `render` (panes), `row` (file-row formatting), `diff_render`
//! (colored diff lines).

use codelet_rpc_types::ChangedFile;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::components::scroll_viewport::{ensure_visible, WheelVelocity, WheelDirection};

mod diff_render;
mod render;
mod row;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// Which pane currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pane {
    #[default]
    Files,
    Diff,
}

/// Outcome of routing a single event through the view. Mirrors the
/// `BlocklistEvent` / `ProviderSettingsEvent` shape — `Emit(Action)` is
/// how the view asks the App to fold state via the dispatcher.
#[derive(Debug, Clone)]
pub enum ChangedFilesEvent {
    /// View consumed the event; no action to emit.
    Consumed,
    /// View did not consume the event.
    Ignored,
    /// View consumed the event and wants the App to dismiss it.
    Close,
    /// View consumed the event and wants the App to emit this action
    /// (e.g. a diff reload for the newly-selected file).
    Emit(crate::components::Action),
}

/// Dual-pane changed-files view state.
pub struct ChangedFilesView {
    files: Vec<ChangedFile>,
    selected_index: usize,
    focused_pane: Pane,
    /// Diff lines for the currently-selected file (split on `\n`).
    diff_lines: Vec<String>,
    /// Path the current `diff_lines` belong to (so a stale
    /// `FileDiffLoaded` for a different file is dropped).
    diff_path: Option<String>,
    file_scroll: usize,
    diff_scroll: usize,
    wheel: WheelVelocity,
    /// Most recent file-list pane Rect (for wheel hit-testing + visible
    /// row math). Set on render.
    last_files_rect: Option<Rect>,
    /// Most recent diff pane Rect. Set on render.
    last_diff_rect: Option<Rect>,
}

impl Default for ChangedFilesView {
    fn default() -> Self {
        Self::new()
    }
}

impl ChangedFilesView {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            selected_index: 0,
            focused_pane: Pane::Files,
            diff_lines: Vec::new(),
            diff_path: None,
            file_scroll: 0,
            diff_scroll: 0,
            wheel: WheelVelocity::new(),
            last_files_rect: None,
            last_diff_rect: None,
        }
    }

    /// Replace the file list from `Action::ChangedFilesLoaded`. Resets
    /// the selection + scroll and clears any stale diff.
    pub fn set_files(&mut self, files: Vec<ChangedFile>) {
        self.files = files;
        self.selected_index = 0;
        self.file_scroll = 0;
        self.diff_scroll = 0;
        self.diff_lines.clear();
        self.diff_path = None;
    }

    /// Fold a `FileDiffLoaded` response. Ignored when the loaded path no
    /// longer matches the selected file (stale async result).
    pub fn set_diff(&mut self, path: &str, diff: Option<String>) {
        let matches_selection = self
            .selected_file()
            .map(|f| f.path == path)
            .unwrap_or(false);
        if !matches_selection {
            return;
        }
        self.diff_path = Some(path.to_string());
        self.diff_scroll = 0;
        self.diff_lines = match diff {
            Some(text) if !text.is_empty() => {
                text.split('\n').map(ToString::to_string).collect()
            }
            _ => vec!["No changes to display".to_string()],
        };
    }

    /// Borrow the currently-selected file (or `None` when empty).
    pub fn selected_file(&self) -> Option<&ChangedFile> {
        self.files.get(self.selected_index)
    }

    /// The repo-relative path of the selected file, if any.
    pub fn selected_path(&self) -> Option<String> {
        self.selected_file().map(|f| f.path.clone())
    }

    pub fn focused_pane(&self) -> Pane {
        self.focused_pane
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn diff_scroll(&self) -> usize {
        self.diff_scroll
    }

    pub fn file_scroll(&self) -> usize {
        self.file_scroll
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Route a key or mouse event. Returns a `ChangedFilesEvent` the
    /// Navigator translates onto the action bus.
    pub fn handle_event(&mut self, event: &Event) -> ChangedFilesEvent {
        match event {
            Event::Key(key) => self.handle_key(*key),
            Event::Mouse(mouse) => self.handle_mouse(*mouse),
            _ => ChangedFilesEvent::Ignored,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> ChangedFilesEvent {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return ChangedFilesEvent::Ignored;
        }
        match key.code {
            KeyCode::Esc => ChangedFilesEvent::Close,
            KeyCode::Tab | KeyCode::BackTab => {
                self.toggle_pane();
                ChangedFilesEvent::Consumed
            }
            KeyCode::Left | KeyCode::Right => {
                self.toggle_pane();
                ChangedFilesEvent::Consumed
            }
            KeyCode::Down => self.move_selection(1),
            KeyCode::Up => self.move_selection(-1),
            KeyCode::PageDown => self.scroll_focused(self.page_step()),
            KeyCode::PageUp => self.scroll_focused(-self.page_step()),
            _ => ChangedFilesEvent::Consumed,
        }
    }

    fn handle_mouse(&mut self, ev: MouseEvent) -> ChangedFilesEvent {
        let dir = match ev.kind {
            MouseEventKind::ScrollUp => WheelDirection::Up,
            MouseEventKind::ScrollDown => WheelDirection::Down,
            _ => return ChangedFilesEvent::Ignored,
        };
        // Hit-test which pane the wheel landed over; default to the
        // currently-focused pane when the cursor is outside both rects.
        let target = self.pane_at(ev.column, ev.row).unwrap_or(self.focused_pane);
        let step = self.wheel.step(dir);
        match target {
            Pane::Diff => self.apply_diff_scroll(step),
            Pane::Files => {
                let _ = self.move_selection(step);
            }
        }
        ChangedFilesEvent::Consumed
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
        None
    }

    fn toggle_pane(&mut self) {
        self.focused_pane = match self.focused_pane {
            Pane::Files => Pane::Diff,
            Pane::Diff => Pane::Files,
        };
    }

    /// Move the file selection by `delta`, clamped to the list bounds,
    /// and request a diff reload for the new selection. Selection moves
    /// regardless of which pane is focused (matches the TS VirtualList
    /// arrow behaviour where the file list owns Up/Down).
    fn move_selection(&mut self, delta: i32) -> ChangedFilesEvent {
        if self.files.is_empty() {
            return ChangedFilesEvent::Consumed;
        }
        let max = self.files.len().saturating_sub(1);
        let proposed = (self.selected_index as i64).saturating_add(delta as i64);
        let clamped = proposed.clamp(0, max as i64) as usize;
        if clamped == self.selected_index {
            return ChangedFilesEvent::Consumed;
        }
        self.selected_index = clamped;
        self.diff_scroll = 0;
        let visible = self
            .last_files_rect
            .map(|r| r.height as usize)
            .unwrap_or(0);
        ensure_visible(&mut self.file_scroll, self.selected_index, visible, self.files.len());
        match self.selected_path() {
            Some(path) => ChangedFilesEvent::Emit(crate::components::Action::LoadFileDiff(path)),
            None => ChangedFilesEvent::Consumed,
        }
    }

    fn scroll_focused(&mut self, delta: i32) -> ChangedFilesEvent {
        match self.focused_pane {
            Pane::Diff => {
                self.apply_diff_scroll(delta);
                ChangedFilesEvent::Consumed
            }
            Pane::Files => self.move_selection(delta),
        }
    }

    fn apply_diff_scroll(&mut self, delta: i32) {
        let max = self.max_diff_scroll();
        let proposed = (self.diff_scroll as i64).saturating_add(delta as i64);
        self.diff_scroll = proposed.clamp(0, max as i64) as usize;
    }

    /// Maximum diff scroll offset: clamps so the LAST FULL PAGE is the
    /// bottom (the last visible line is the final diff line). Before the
    /// first render the pane height is unknown (0), so we fall back to the
    /// len-1 clamp to keep behaviour sane.
    fn max_diff_scroll(&self) -> usize {
        let viewport = self.last_diff_rect.map(|r| r.height as usize).unwrap_or(0);
        let len = self.diff_lines.len();
        if viewport == 0 {
            return len.saturating_sub(1);
        }
        len.saturating_sub(viewport)
    }

    fn page_step(&self) -> i32 {
        let h = match self.focused_pane {
            Pane::Diff => self.last_diff_rect.map(|r| r.height).unwrap_or(1),
            Pane::Files => self.last_files_rect.map(|r| r.height).unwrap_or(1),
        };
        (h.max(1)) as i32
    }
}
