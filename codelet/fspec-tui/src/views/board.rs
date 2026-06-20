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
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use tokio::sync::mpsc::UnboundedSender;

use crate::components::{Action, EventResult};
use crate::store::{BoardStore, COLUMN_ORDER};
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
pub mod viewport;

use self::columns::paint_column_headers;
use self::grid::{
    build_border_row, calculate_column_widths, column_width_at, slice_column_rects, SeparatorType,
};
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
            _ => {}
        }

        EventResult::ignored()
    }

    fn selected_session(&self, store: &BoardStore) -> Option<SessionId> {
        let unit = store.selected_work_unit()?;
        store.session_for(&unit.id).cloned()
    }

    /// Render the rich BoardView against the supplied store. Composes
    /// the box-drawing topology row-by-row: top border, 4-row header
    /// strip (RPC-015 logo + checkpoints + keybindings), ├──┤ plain
    /// separator, 5-row details strip, ├┬┤ separator, column header
    /// row, ├┼┤ separator, content rows, ├┴┤ separator, RPC-013 footer
    /// string, bottom border.
    pub fn render_with_store(&self, area: Rect, buf: &mut Buffer, store: &BoardStore) {
        if area.width < 4 || area.height < 17 {
            return;
        }
        let widths = calculate_column_widths(area.width);
        let inner_width: u16 = (0..COLUMN_ORDER.len() as u16)
            .map(|i| column_width_at(i as usize, widths))
            .sum::<u16>()
            + (COLUMN_ORDER.len() as u16 - 1);
        if inner_width + 2 > area.width {
            return;
        }
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // top border
                Constraint::Length(4), // RPC-015 header strip
                Constraint::Length(1), // ├──┤ plain separator (RPC-015)
                Constraint::Length(5), // details strip (RPC-014)
                Constraint::Length(1), // ├┬┤ separator
                Constraint::Length(1), // column header
                Constraint::Length(1), // ├┼┤ separator
                Constraint::Min(0),    // content
                Constraint::Length(1), // ├┴┤ separator
                Constraint::Length(1), // footer
                Constraint::Length(1), // bottom border
            ])
            .split(area);
        let border_style = Style::default().fg(self.theme.border);

        borders::paint_border_string(
            split[0],
            buf,
            &build_border_row(widths, "┌", "┐", SeparatorType::Plain),
            border_style,
        );
        borders::paint_side_borders(split[1], buf, border_style);
        header::paint(borders::inner_rect(split[1]), buf, store, &self.theme);
        borders::paint_border_string(
            split[2],
            buf,
            &build_border_row(widths, "├", "┤", SeparatorType::Plain),
            border_style,
        );
        borders::paint_side_borders(split[3], buf, border_style);
        details_strip::render(
            borders::inner_rect(split[3]),
            buf,
            store.selected_work_unit(),
        );
        borders::paint_border_string(
            split[4],
            buf,
            &build_border_row(widths, "├", "┤", SeparatorType::Top),
            border_style,
        );
        borders::paint_side_borders(split[5], buf, border_style);
        paint_column_headers(split[5], buf, widths, store, &self.theme);
        // RPC-023: cache per-column header rects for click-to-focus.
        self.last_column_header_areas
            .set(Some(slice_column_rects(split[5], widths)));
        borders::paint_border_string(
            split[6],
            buf,
            &build_border_row(widths, "├", "┤", SeparatorType::Cross),
            border_style,
        );
        // RPC-016: record the viewport height the painter is about to
        // observe so handle_event can emit ScrollFocusedColumnUp/Down
        // with the right scroll step.
        self.last_viewport_height.set(split[7].height);
        // RPC-023: cache the content rect + per-column content rects
        // for wheel + click hit-testing.
        self.last_content_area.set(Some(split[7]));
        self.last_column_content_areas
            .set(Some(slice_column_rects(split[7], widths)));
        paint_content_rows(split[7], buf, widths, store, &self.theme);
        borders::paint_border_string(
            split[8],
            buf,
            &build_border_row(widths, "├", "┤", SeparatorType::Bottom),
            border_style,
        );
        borders::paint_side_borders(split[9], buf, border_style);
        footer::render(borders::inner_rect(split[9]), buf, &self.theme);
        borders::paint_border_string(
            split[10],
            buf,
            &build_border_row(widths, "└", "┘", SeparatorType::Plain),
            border_style,
        );
    }
}
