//! RPC-016 — BoardStore viewport math tests.
//!
//! Feature: spec/features/rpc016-board-store-viewport.feature
//!
//! Pins the per-column `scroll_offsets` field plus the five new
//! mutation methods used by App::dispatch to drive viewport-aware
//! selection (PageUp/PageDown/Home/End + auto-scrolling arrow keys).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::{BoardStore, COLUMN_ORDER};
use codelet_rpc_types::WorkUnitInfo;

fn wu(id: &str, status: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

fn seed_backlog(n: usize) -> BoardStore {
    let mut store = BoardStore::default();
    let units: Vec<WorkUnitInfo> = (0..n)
        .map(|i| wu(&format!("AUTH-{i:03}"), "backlog"))
        .collect();
    store.replace_work_units(units);
    store
}

/// Scenario: Default BoardStore reports zero scroll_offset for every column
#[test]
fn default_boardstore_reports_zero_scroll_offset_for_every_column() {
    // @step Given a freshly constructed BoardStore via BoardStore::default()
    let store = BoardStore::default();
    // @step When the developer reads scroll_offset_for for each of the seven canonical columns
    // @step Then every column returns 0
    for column in COLUMN_ORDER {
        assert_eq!(
            store.scroll_offset_for(column),
            0,
            "fresh BoardStore.scroll_offset_for({column}) must be 0"
        );
    }
}

/// Scenario: set_scroll_offset_for stores per-column offsets independently
#[test]
fn set_scroll_offset_for_stores_per_column_offsets_independently() {
    // @step Given a freshly constructed BoardStore via BoardStore::default()
    let mut store = BoardStore::default();
    // @step When the developer calls set_scroll_offset_for("backlog", 4)
    store.set_scroll_offset_for("backlog", 4);
    // @step And the developer calls set_scroll_offset_for("done", 12)
    store.set_scroll_offset_for("done", 12);
    // @step Then scroll_offset_for("backlog") returns 4
    assert_eq!(store.scroll_offset_for("backlog"), 4);
    // @step And scroll_offset_for("done") returns 12
    assert_eq!(store.scroll_offset_for("done"), 12);
    // @step And scroll_offset_for("implementing") returns 0
    assert_eq!(store.scroll_offset_for("implementing"), 0);
}

/// Scenario: move_selection within visible viewport leaves scroll_offset unchanged
#[test]
fn move_selection_within_visible_viewport_leaves_scroll_offset_unchanged() {
    // @step Given a BoardStore seeded with twenty story work units all in the backlog column
    let mut store = seed_backlog(20);
    // @step And the focused column is "backlog" with selected index 3 and scroll_offset 0
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 3);
    store.set_scroll_offset_for("backlog", 0);
    // @step When the developer calls move_selection(1, 10) (move down by 1 with viewport_height 10)
    store.move_selection(1, 10);
    // @step Then selected_index_for("backlog") returns 4
    assert_eq!(store.selected_index_for("backlog"), 4);
    // @step And scroll_offset_for("backlog") returns 0
    assert_eq!(store.scroll_offset_for("backlog"), 0);
}

/// Scenario: move_selection beyond bottom of viewport scrolls the focused column down
#[test]
fn move_selection_beyond_bottom_of_viewport_scrolls_focused_column_down() {
    // @step Given a BoardStore seeded with twenty story work units all in the backlog column
    let mut store = seed_backlog(20);
    // @step And the focused column is "backlog" with selected index 9 and scroll_offset 0
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 9);
    store.set_scroll_offset_for("backlog", 0);
    // @step When the developer calls move_selection(1, 10) (move down by 1 with viewport_height 10)
    store.move_selection(1, 10);
    // @step Then selected_index_for("backlog") returns 10
    assert_eq!(store.selected_index_for("backlog"), 10);
    // @step And scroll_offset_for("backlog") is strictly greater than 0
    let offset = store.scroll_offset_for("backlog");
    assert!(
        offset > 0,
        "scroll_offset must advance past 0 after moving selection past the visible viewport (was {offset})"
    );
    // @step And the selected index remains inside the visible viewport window
    // Selected index 10 with viewport_height 10 must satisfy:
    //   offset <= 10 < offset + viewport_height
    // accounting for ↑/↓ arrow rows consuming one viewport row each.
    let viewport_height: usize = 10;
    let selected = store.selected_index_for("backlog");
    assert!(
        selected >= offset && selected < offset + viewport_height,
        "selected_index {selected} must be inside [{offset}, {})",
        offset + viewport_height
    );
}

/// Scenario: move_selection above top of viewport scrolls the focused column up
#[test]
fn move_selection_above_top_of_viewport_scrolls_focused_column_up() {
    // @step Given a BoardStore seeded with twenty story work units all in the backlog column
    let mut store = seed_backlog(20);
    // @step And the focused column is "backlog" with selected index 5 and scroll_offset 5
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 5);
    store.set_scroll_offset_for("backlog", 5);
    // @step When the developer calls move_selection(-1, 10) (move up by 1 with viewport_height 10)
    store.move_selection(-1, 10);
    // @step Then selected_index_for("backlog") returns 4
    assert_eq!(store.selected_index_for("backlog"), 4);
    // @step And scroll_offset_for("backlog") is strictly less than 5
    let offset = store.scroll_offset_for("backlog");
    assert!(
        offset < 5,
        "scroll_offset must decrement below 5 after moving selection above the visible viewport (was {offset})"
    );
}

/// Scenario: move_selection wraps to first index when moving past the last unit
#[test]
fn move_selection_wraps_to_first_index_when_moving_past_the_last_unit() {
    // @step Given a BoardStore seeded with three story work units all in the backlog column
    let mut store = seed_backlog(3);
    // @step And the focused column is "backlog" with selected index 2 and scroll_offset 0
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 2);
    store.set_scroll_offset_for("backlog", 0);
    // @step When the developer calls move_selection(1, 10) (move down by 1 with viewport_height 10)
    store.move_selection(1, 10);
    // @step Then selected_index_for("backlog") returns 0
    assert_eq!(store.selected_index_for("backlog"), 0);
    // @step And scroll_offset_for("backlog") returns 0
    assert_eq!(store.scroll_offset_for("backlog"), 0);
}

/// Scenario: move_selection wraps to last index when moving above the first unit
#[test]
fn move_selection_wraps_to_last_index_when_moving_above_the_first_unit() {
    // @step Given a BoardStore seeded with five story work units all in the backlog column
    let mut store = seed_backlog(5);
    // @step And the focused column is "backlog" with selected index 0 and scroll_offset 0
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 0);
    store.set_scroll_offset_for("backlog", 0);
    // @step When the developer calls move_selection(-1, 10) (move up by 1 with viewport_height 10)
    store.move_selection(-1, 10);
    // @step Then selected_index_for("backlog") returns 4
    assert_eq!(store.selected_index_for("backlog"), 4);
    // @step And the visible viewport contains the selected index
    let offset = store.scroll_offset_for("backlog");
    let viewport_height: usize = 10;
    let selected = store.selected_index_for("backlog");
    assert!(
        selected >= offset && selected < offset + viewport_height,
        "selected_index {selected} must be inside [{offset}, {})",
        offset + viewport_height
    );
}

/// Scenario: scroll_focused_column advances the selection by viewport_height
#[test]
fn scroll_focused_column_advances_the_selection_by_viewport_height() {
    // @step Given a BoardStore seeded with thirty story work units all in the backlog column
    let mut store = seed_backlog(30);
    // @step And the focused column is "backlog" with selected index 0 and scroll_offset 0
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 0);
    store.set_scroll_offset_for("backlog", 0);
    // @step When the developer calls scroll_focused_column(1, 10)
    store.scroll_focused_column(1, 10);
    // @step Then selected_index_for("backlog") returns 10
    assert_eq!(store.selected_index_for("backlog"), 10);
    // @step And scroll_offset_for("backlog") is strictly greater than 0
    assert!(
        store.scroll_offset_for("backlog") > 0,
        "scroll_focused_column(1, 10) must also advance the scroll_offset above 0"
    );
}

/// Scenario: select_first_in_focused resets the focused column to index 0 with offset 0
#[test]
fn select_first_in_focused_resets_focused_column_to_index_0_offset_0() {
    // @step Given a BoardStore seeded with thirty story work units all in the backlog column
    let mut store = seed_backlog(30);
    // @step And the focused column is "backlog" with selected index 17 and scroll_offset 8
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 17);
    store.set_scroll_offset_for("backlog", 8);
    // @step When the developer calls select_first_in_focused()
    store.select_first_in_focused();
    // @step Then selected_index_for("backlog") returns 0
    assert_eq!(store.selected_index_for("backlog"), 0);
    // @step And scroll_offset_for("backlog") returns 0
    assert_eq!(store.scroll_offset_for("backlog"), 0);
}

/// Scenario: select_last_in_focused jumps to the last unit
#[test]
fn select_last_in_focused_jumps_to_the_last_unit() {
    // @step Given a BoardStore seeded with thirty story work units all in the backlog column
    let mut store = seed_backlog(30);
    // @step And the focused column is "backlog" with selected index 0 and scroll_offset 0
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 0);
    store.set_scroll_offset_for("backlog", 0);
    // @step When the developer calls select_last_in_focused()
    store.select_last_in_focused();
    // @step Then selected_index_for("backlog") returns 29
    assert_eq!(store.selected_index_for("backlog"), 29);
}
