//! BOARD-022 / BUG-162 — constructor + accessors for
//! [`super::work_unit_search_dialog::WorkUnitSearchDialog`].
//!
//! Feature: spec/features/board-search-dialog-with-tab-toggled-id-title-description-modes.feature
//!
//! Extracted from `work_unit_search_dialog.rs` so that file stays under the
//! 300-LoC budget after the BUG-162 mouse-state fields landed.

use std::cell::Cell;

use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use codelet_rpc_types::WorkUnitInfo;

use super::scroll_viewport::WheelVelocity;
use super::work_unit_search_dialog::{
    filter_work_units, SearchMatch, SearchMode, WorkUnitSearchDialog, WORK_UNIT_SEARCH_DIALOG_ID,
};
use crate::mouse::scrollbar_drag::ScrollbarDrag;

impl WorkUnitSearchDialog {
    /// Construct a fresh dialog over a snapshot of the board's units.
    /// Defaults to Id mode with an empty query (all units listed).
    pub fn new(units: Vec<WorkUnitInfo>) -> Self {
        let mode = SearchMode::default();
        Self {
            id: WORK_UNIT_SEARCH_DIALOG_ID.to_string(),
            query: String::new(),
            matches: filter_work_units(&units, mode, ""),
            units,
            mode,
            selected: 0,
            scroll_offset: 0,
            last_visible_rows: Cell::new(10),
            action_tx: None,
            wheel: WheelVelocity::new(),
            scrollbar_drag: ScrollbarDrag::new(),
            last_dialog_rect: None,
            last_scrollbar_rect: None,
        }
    }

    /// Builder-style action_tx attach for the App's UnboundedSender.
    pub fn with_action_tx(mut self, tx: UnboundedSender<super::Action>) -> Self {
        self.action_tx = Some(tx);
        self
    }

    /// Test accessor — the active mode's short label ("id"/"title"/"desc").
    pub fn mode_label(&self) -> &'static str {
        self.mode.label()
    }

    /// Test accessor — the current match ids in display order.
    pub fn matches(&self) -> Vec<String> {
        self.matches.iter().map(|m| m.id.clone()).collect()
    }

    /// BUG-160: the richer matches (id + snippet) in display order.
    pub fn matches_with_snippets(&self) -> &[SearchMatch] {
        &self.matches
    }

    /// Test accessor — the currently highlighted row index.
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// BUG-162: the first visible match index (scroll window start).
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// BUG-162: the cached fixed dialog rect from the last render
    /// (`None` before the first render).
    pub fn last_dialog_rect(&self) -> Option<Rect> {
        self.last_dialog_rect
    }

    /// BUG-162: the cached scrollbar gutter rect from the last render
    /// (`None` when the matches fit in the visible rows).
    pub fn last_scrollbar_rect(&self) -> Option<Rect> {
        self.last_scrollbar_rect
    }

    /// The visible-rows window (the value `render` last set).
    pub fn visible_rows(&self) -> usize {
        self.last_visible_rows.get().max(1)
    }
}
