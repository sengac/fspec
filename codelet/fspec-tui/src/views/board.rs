//! BoardView — rich box-drawing Kanban grid + work-unit details strip.
//!
//! Feature files:
//!   - spec/features/rpc012-board-agent-navigation.feature
//!   - spec/features/rpc013-board-footer.feature
//!   - spec/features/rpc014-board-grid.feature
//!   - spec/features/rpc014-source-shape.feature
//!   - spec/features/boardview-mouse-handling.feature (RPC-023)
//!
//! Cards: RPC-012 / RPC-013 / RPC-014 / RPC-016 / RPC-023.
//!
//! Renders the seven canonical kanban columns with box-drawing
//! separators, a 5-row details strip, focused-column highlighting and
//! per-column viewport scroll (RPC-016 `↑`/`↓` arrows). Wheel + click
//! mouse handling lives in the sibling `mouse` module (RPC-023).
//! BoardView holds NO work-units state — `render_with_store` borrows a
//! `&BoardStore`. Keyboard + mouse handlers emit Actions onto the bus
//! that `App::dispatch` consumes.

use std::cell::Cell;
use std::sync::Arc;

use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use crate::components::{Action, EventResult};
use crate::store::BoardStore;
use crate::theme::Theme;

pub mod borders;
pub mod checkpoint_status;
pub mod columns;
pub mod details_strip;
pub mod footer;
pub mod grid;
pub mod header;
pub mod keybinding_shortcuts;
pub mod logo;
pub mod mouse;
pub mod render;
pub mod viewport;

use self::columns::paint_column_headers;
use self::grid::calculate_column_widths;
use self::viewport::paint_content_rows;

/// BoardView holds NO work-units state — only the action bus + theme +
/// the most-recent viewport_height observed at render time (so
/// handle_event can emit Action::ScrollFocusedColumnUp/Down with the
/// right scroll step for the CURRENT terminal height).
pub struct BoardView {
    pub theme: Arc<Theme>,
    pub action_tx: Option<UnboundedSender<Action>>,
    /// RPC-016: the column-content viewport_height observed by the most
    /// recent `render_with_store` call. Read by `handle_event` to
    /// produce ScrollFocusedColumnUp/Down payloads, and by App::dispatch
    /// when routing SelectNext/SelectPrev through BoardStore::move_selection.
    last_viewport_height: Cell<u16>,
    /// RPC-023: the column-content Rect (split[7]) observed by the
    /// most recent `render_with_store`. Read by the mouse branch in
    /// [`self::mouse::handle_mouse`] for wheel hit-testing.
    pub(super) last_content_area: Cell<Option<Rect>>,
    /// RPC-023: per-column header Rects observed by the most recent
    /// `render_with_store`. Indexed by `COLUMN_ORDER` position.
    pub(super) last_column_header_areas: Cell<Option<[Rect; 7]>>,
    /// RPC-023: per-column content Rects observed by the most recent
    /// `render_with_store`. Indexed by `COLUMN_ORDER` position.
    pub(super) last_column_content_areas: Cell<Option<[Rect; 7]>>,
}

impl BoardView {
    pub fn new(theme: Arc<Theme>, action_tx: UnboundedSender<Action>) -> Self {
        Self {
            theme,
            action_tx: Some(action_tx),
            last_viewport_height: Cell::new(1),
            last_content_area: Cell::new(None),
            last_column_header_areas: Cell::new(None),
            last_column_content_areas: Cell::new(None),
        }
    }

    /// RPC-016: read the most-recent column-content viewport_height
    /// observed by `render_with_store`. App::dispatch uses this when
    /// routing arrow keys through `BoardStore::move_selection`.
    pub fn last_viewport_height(&self) -> usize {
        self.last_viewport_height.get() as usize
    }

    pub(super) fn emit(&self, action: Action) {
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(action);
        }
    }

    /// Handle a keyboard or mouse event against the supplied store
    /// snapshot. The store is &-borrow only; mutation flows through
    /// App::dispatch in response to the emitted action.
    pub fn handle_event(&self, event: &Event, store: &BoardStore) -> EventResult {
        // RPC-023: mouse branch lives in `mouse.rs` so this file stays
        // under the 300 LoC ceiling.
        if matches!(event, Event::Mouse(_)) {
            return mouse::handle_mouse(self, event, store);
        }

        let Event::Key(key) = event else {
            return EventResult::ignored();
        };

        // Shift+Right → open AgentView (with or without an attached session).
        if key.code == KeyCode::Right && key.modifiers.contains(KeyModifiers::SHIFT) {
            let target = self.selected_session(store);
            self.emit(Action::OpenAgentView(target));
            return EventResult::consumed();
        }

        // Enter → hand off to AgentView for the focused work unit.
        if key.code == KeyCode::Enter {
            if let Some(unit) = store.selected_work_unit() {
                self.emit(Action::EnterWorkUnit(unit.id.clone()));
                return EventResult::consumed();
            }
            return EventResult::ignored();
        }

        match key.code {
            KeyCode::PageUp => {
                let vh = self.last_viewport_height();
                self.emit(Action::ScrollFocusedColumnUp(vh));
                return EventResult::consumed();
            }
            KeyCode::PageDown => {
                let vh = self.last_viewport_height();
                self.emit(Action::ScrollFocusedColumnDown(vh));
                return EventResult::consumed();
            }
            KeyCode::Home => {
                self.emit(Action::SelectFirstInFocused);
                return EventResult::consumed();
            }
            KeyCode::End => {
                self.emit(Action::SelectLastInFocused);
                return EventResult::consumed();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.emit(Action::FocusPrevColumn);
                return EventResult::consumed();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.emit(Action::FocusNextColumn);
                return EventResult::consumed();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.emit(Action::SelectNext);
                return EventResult::consumed();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.emit(Action::SelectPrev);
                return EventResult::consumed();
            }
            KeyCode::Char('[') => {
                self.emit(Action::ReorderUp);
                return EventResult::consumed();
            }
            KeyCode::Char(']') => {
                self.emit(Action::ReorderDown);
                return EventResult::consumed();
            }
            // RPC-356: open the dual-pane Changed Files view.
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.emit(Action::OpenChangedFilesView);
                return EventResult::consumed();
            }
            // RPC-364: open the three-pane Checkpoints view.
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.emit(Action::OpenCheckpointsView);
                return EventResult::consumed();
            }
            // RPC-373: open FOUNDATION.md in the browser via the viewer server.
            // Modifier-free only: `Ctrl+D` is reserved as the App-level
            // hard-quit shortcut (RPC-102) and must fall through to Stage 4.
            KeyCode::Char('d') | KeyCode::Char('D')
                if !key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.emit(Action::OpenFoundation);
                return EventResult::consumed();
            }
            // RPC-374: open the attachment picker for the selected work unit.
            // Always consume the key; emit the picker action only when the
            // selected unit has at least one attachment (silent no-op otherwise).
            // Modifier-free only, so Ctrl-chorded keys fall through.
            KeyCode::Char('a') | KeyCode::Char('A')
                if !key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                if store
                    .selected_work_unit()
                    .is_some_and(|u| !u.attachments.is_empty())
                {
                    self.emit(Action::OpenAttachmentPicker);
                }
                return EventResult::consumed();
            }
            _ => {}
        }

        EventResult::ignored()
    }

    fn selected_session(&self, store: &BoardStore) -> Option<SessionId> {
        let unit = store.selected_work_unit()?;
        store.session_for(&unit.id).cloned()
    }

    /// Render the rich BoardView against the supplied store. The
    /// box-drawing composition lives in [`self::render`] so this file
    /// stays under the 300 LoC ceiling.
    pub fn render_with_store(&self, area: Rect, buf: &mut Buffer, store: &BoardStore) {
        render::render_with_store(self, area, buf, store);
    }
}
