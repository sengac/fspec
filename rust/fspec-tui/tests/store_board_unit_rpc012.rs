//! RPC-012 — Inline-equivalent unit tests for BoardStore extracted to
//! a separate integration file so `store/board.rs` stays < 300 LoC
//! per the file-size invariant.
//!
//! Feature: spec/features/rpc012-board-store.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::{BoardStore, COLUMN_ORDER};
use codelet_rpc_types::{SessionId, WorkUnitInfo};

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

/// Scenario: BoardStore seeds work units grouped into 7 columns with focus at backlog
#[test]
fn boardstore_seeds_work_units_grouped_into_7_columns_with_focus_at_backlog() {
    // @step Given a freshly constructed BoardStore via BoardStore::default()
    let mut store = BoardStore::default();
    // @step When the developer calls store.replace_work_units with [AUTH-001 backlog, AUTH-002 implementing, AUTH-003 done]
    store.replace_work_units(vec![
        wu("AUTH-001", "backlog"),
        wu("AUTH-002", "implementing"),
        wu("AUTH-003", "done"),
    ]);
    // @step Then store.column_units("backlog") returns exactly [AUTH-001]
    let backlog = store.column_units("backlog");
    assert_eq!(backlog.len(), 1);
    assert_eq!(backlog[0].id, "AUTH-001");
    // @step And store.column_units("implementing") returns exactly [AUTH-002]
    let implementing = store.column_units("implementing");
    assert_eq!(implementing.len(), 1);
    assert_eq!(implementing[0].id, "AUTH-002");
    // @step And store.column_units("done") returns exactly [AUTH-003]
    let done = store.column_units("done");
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].id, "AUTH-003");
    // @step And store.focused_column() returns "backlog"
    assert_eq!(store.focused_column(), "backlog");
    // @step And store.selected_index_for("backlog") returns 0
    assert_eq!(store.selected_index_for("backlog"), 0);
}

/// Scenario: BoardStore re-grouping preserves focus and clamps selection when columns shrink
#[test]
fn boardstore_regrouping_preserves_focus_and_clamps_selection_when_columns_shrink() {
    // @step Given a BoardStore seeded with [AUTH-001 backlog, AUTH-002 implementing, AUTH-003 done]
    let mut store = BoardStore::default();
    store.replace_work_units(vec![
        wu("AUTH-001", "backlog"),
        wu("AUTH-002", "implementing"),
        wu("AUTH-003", "done"),
    ]);
    store.set_focused_column("implementing");
    store.set_selected_index_for("implementing", 0);
    // @step And store.focused_column() returns "implementing"
    assert_eq!(store.focused_column(), "implementing");
    // @step And store.selected_index_for("implementing") returns 0
    assert_eq!(store.selected_index_for("implementing"), 0);
    // @step When store.replace_work_units is called with [AUTH-001 backlog, AUTH-002 validating, AUTH-003 done]
    store.replace_work_units(vec![
        wu("AUTH-001", "backlog"),
        wu("AUTH-002", "validating"),
        wu("AUTH-003", "done"),
    ]);
    // @step Then store.column_units("validating") returns exactly [AUTH-002]
    let validating = store.column_units("validating");
    assert_eq!(validating.len(), 1);
    assert_eq!(validating[0].id, "AUTH-002");
    // @step And store.column_units("implementing") returns an empty slice
    assert!(store.column_units("implementing").is_empty());
    // @step And store.focused_column() still returns "implementing"
    assert_eq!(store.focused_column(), "implementing");
    // @step And store.selected_index_for("implementing") returns 0
    assert_eq!(store.selected_index_for("implementing"), 0);
}

#[test]
fn attach_session_round_trip_and_session_migration() {
    let mut store = BoardStore::default();
    store.replace_work_units(vec![wu("AUTH-001", "backlog"), wu("AUTH-002", "backlog")]);
    let s1 = SessionId::new("s-1");
    store.attach_session("AUTH-001", s1.clone());
    assert_eq!(store.session_for("AUTH-001"), Some(&s1));
    store.attach_session("AUTH-002", s1.clone());
    assert_eq!(store.session_for("AUTH-002"), Some(&s1));
    assert_eq!(store.session_for("AUTH-001"), None);
}

#[test]
fn focus_next_and_prev_wrap_at_boundaries() {
    let mut store = BoardStore::default();
    assert_eq!(store.focused_column_index(), 0);
    store.focus_prev_column();
    assert_eq!(store.focused_column(), "blocked");
    for _ in 0..COLUMN_ORDER.len() {
        store.focus_next_column();
    }
    assert_eq!(store.focused_column(), "blocked");
}
