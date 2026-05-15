//! RPC-016 — Source-shape regressions for the per-column viewport port:
//! viewport module + WorkUnitInfo.last_state_change_at + new Action
//! variants + BoardStore viewport methods.
//!
//! Feature: spec/features/rpc016-source-shape.feature
//!
//! These tests pin the file layout introduced by RPC-016 so future
//! cards cannot silently merge the viewport painter back into board.rs
//! or strip the new additive fields.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

fn src_dir() -> std::path::PathBuf {
    common::workspace_root().join("fspec-tui").join("src")
}

fn rpc_types_lib() -> std::path::PathBuf {
    common::workspace_root().join("rpc-types").join("src").join("lib.rs")
}

fn core_work_units() -> std::path::PathBuf {
    common::workspace_root().join("core").join("src").join("work_units.rs")
}

fn read_raw(path: &std::path::Path) -> String {
    common::read_to_string_or_panic(path)
}

fn read_stripped(rel: &str) -> String {
    let path = src_dir().join(rel);
    let body = common::read_to_string_or_panic(&path);
    common::strip_rust_comments(&body)
}

fn count_lines_path(path: &std::path::Path) -> usize {
    common::read_to_string_or_panic(path).lines().count()
}

/// Scenario: WorkUnitInfo gains the last_state_change_at field
#[test]
fn work_unit_info_gains_the_last_state_change_at_field() {
    // @step Given codelet/rpc-types/src/lib.rs after RPC-016 lands
    let body = read_raw(&rpc_types_lib());
    // @step When a developer reads the file source raw
    // @step Then the file contains the substring "pub last_state_change_at: Option<String>"
    assert!(
        body.contains("pub last_state_change_at: Option<String>"),
        "codelet/rpc-types/src/lib.rs must declare `pub last_state_change_at: Option<String>` on WorkUnitInfo after RPC-016"
    );
}

/// Scenario: codelet_core::work_units reads stateHistory into last_state_change_at
#[test]
fn codelet_core_work_units_reads_state_history_into_last_state_change_at() {
    // @step Given codelet/core/src/work_units.rs after RPC-016 lands
    let body = read_raw(&core_work_units());
    // @step When a developer reads the file source raw
    // @step Then the file contains the substring "stateHistory"
    assert!(
        body.contains("stateHistory") || body.contains("state_history"),
        "codelet/core/src/work_units.rs must reference stateHistory (or state_history serde rename) after RPC-016"
    );
    // @step And the file contains the substring "last_state_change_at"
    assert!(
        body.contains("last_state_change_at"),
        "codelet/core/src/work_units.rs must populate `last_state_change_at` after RPC-016"
    );
}

/// Scenario: Action enum gains the four new viewport variants
#[test]
fn action_enum_gains_the_four_new_viewport_variants() {
    // @step Given codelet/fspec-tui/src/components/mod.rs after RPC-016 lands
    let body = read_raw(&src_dir().join("components").join("mod.rs"));
    // @step When a developer reads the file source raw
    // @step Then the file contains the substring "ScrollFocusedColumnUp"
    assert!(
        body.contains("ScrollFocusedColumnUp"),
        "Action enum must include the `ScrollFocusedColumnUp` variant after RPC-016"
    );
    // @step And the file contains the substring "ScrollFocusedColumnDown"
    assert!(
        body.contains("ScrollFocusedColumnDown"),
        "Action enum must include the `ScrollFocusedColumnDown` variant after RPC-016"
    );
    // @step And the file contains the substring "SelectFirstInFocused"
    assert!(
        body.contains("SelectFirstInFocused"),
        "Action enum must include the `SelectFirstInFocused` variant after RPC-016"
    );
    // @step And the file contains the substring "SelectLastInFocused"
    assert!(
        body.contains("SelectLastInFocused"),
        "Action enum must include the `SelectLastInFocused` variant after RPC-016"
    );
}

/// Scenario: BoardStore declares the scroll_offsets field and viewport methods
#[test]
fn boardstore_declares_scroll_offsets_field_and_viewport_methods() {
    // @step Given codelet/fspec-tui/src/store/board.rs after RPC-016 lands
    let body = read_raw(&src_dir().join("store").join("board.rs"));
    // The RPC-016 viewport methods live in the sibling
    // `board_viewport.rs` module so `board.rs` stays under the 300 LoC
    // ceiling. Both files together form the BoardStore surface.
    let viewport_body = read_raw(&src_dir().join("store").join("board_viewport.rs"));
    let combined = format!("{body}\n{viewport_body}");
    // @step When a developer reads the file source raw
    // @step Then the file contains the substring "scroll_offsets"
    assert!(
        body.contains("scroll_offsets"),
        "BoardStore must declare a `scroll_offsets` field after RPC-016"
    );
    // @step And the file contains the substring "pub fn scroll_offset_for"
    assert!(
        combined.contains("pub fn scroll_offset_for"),
        "BoardStore must expose `pub fn scroll_offset_for` after RPC-016"
    );
    // @step And the file contains the substring "pub fn set_scroll_offset_for"
    assert!(
        combined.contains("pub fn set_scroll_offset_for"),
        "BoardStore must expose `pub fn set_scroll_offset_for` after RPC-016"
    );
    // @step And the file contains the substring "pub fn move_selection"
    assert!(
        combined.contains("pub fn move_selection"),
        "BoardStore must expose `pub fn move_selection` after RPC-016"
    );
    // @step And the file contains the substring "pub fn scroll_focused_column"
    assert!(
        combined.contains("pub fn scroll_focused_column"),
        "BoardStore must expose `pub fn scroll_focused_column` after RPC-016"
    );
    // @step And the file contains the substring "pub fn select_first_in_focused"
    assert!(
        combined.contains("pub fn select_first_in_focused"),
        "BoardStore must expose `pub fn select_first_in_focused` after RPC-016"
    );
    // @step And the file contains the substring "pub fn select_last_in_focused"
    assert!(
        combined.contains("pub fn select_last_in_focused"),
        "BoardStore must expose `pub fn select_last_in_focused` after RPC-016"
    );
}

/// Scenario: Viewport painter module exists as a separate file
#[test]
fn viewport_painter_module_exists_as_separate_file() {
    // @step Given the codelet/fspec-tui crate after RPC-016 lands
    // @step When a developer scans the views/board/ directory
    let viewport = src_dir().join("views").join("board").join("viewport.rs");
    // @step Then the file codelet/fspec-tui/src/views/board/viewport.rs exists
    assert!(
        viewport.exists(),
        "codelet/fspec-tui/src/views/board/viewport.rs must exist after RPC-016"
    );
}

/// Scenario: New and modified board modules stay under 300 lines
#[test]
fn new_and_modified_board_modules_stay_under_300_lines() {
    // @step Given the directory codelet/fspec-tui/src/views/board/ plus the views/board.rs orchestrator and store/board.rs
    let board_dir = src_dir().join("views").join("board");
    let board_orchestrator = src_dir().join("views").join("board.rs");
    let store_board = src_dir().join("store").join("board.rs");
    // @step When a test counts the line-count of every .rs file in views/board/ plus views/board.rs plus store/board.rs
    let mut targets: Vec<std::path::PathBuf> = vec![board_orchestrator.clone(), store_board.clone()];
    let entries = std::fs::read_dir(&board_dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", board_dir.display()));
    for entry in entries {
        let entry = entry.expect("read_dir entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            targets.push(path);
        }
    }
    targets.sort();
    let mut violations = Vec::new();
    for path in &targets {
        let lines = count_lines_path(path);
        if lines >= 300 {
            violations.push(format!("{}: {} lines >= 300 ceiling", path.display(), lines));
        }
    }
    // @step Then views/board.rs has fewer than 300 lines
    // @step And store/board.rs has fewer than 300 lines
    // @step And every .rs file under views/board/ has fewer than 300 lines
    assert!(
        violations.is_empty(),
        "RPC-016 board modules MUST stay < 300 LoC. Violations: {violations:?}"
    );
}

/// Scenario: RPC-013 / RPC-014 / RPC-015 invariants preserved
#[test]
fn rpc013_rpc014_rpc015_invariants_preserved() {
    // @step Given codelet/fspec-tui/src/views/board.rs after RPC-016 lands
    let board = read_raw(&src_dir().join("views").join("board.rs"));
    // @step When a developer reads the file source raw
    // @step Then the file contains the substring "Action::EnterWorkUnit"
    assert!(
        board.contains("Action::EnterWorkUnit"),
        "views/board.rs must still emit Action::EnterWorkUnit after RPC-016"
    );
    // @step And the file contains the substring "Action::FocusNextColumn"
    assert!(
        board.contains("Action::FocusNextColumn"),
        "views/board.rs must still emit Action::FocusNextColumn after RPC-016"
    );
    // @step And the file contains the substring "Action::ReorderUp"
    assert!(
        board.contains("Action::ReorderUp"),
        "views/board.rs must still emit Action::ReorderUp after RPC-016"
    );
    // @step And the file does NOT contain the identifier "FooterView"
    let stripped = common::strip_rust_comments(&board);
    assert!(
        !stripped.contains("FooterView"),
        "views/board.rs must not reference FooterView after RPC-016"
    );
}

/// Scenario: Views still avoid encapsulated transport crates and host runtime construction
#[test]
fn views_still_avoid_encapsulated_transport_crates_and_runtime_construction() {
    // @step Given the directory codelet/fspec-tui/src/views/ (including views/board/)
    let views_dir = src_dir().join("views");
    // @step When a test scans every *.rs file
    let rs_files = common::collect_rs_files(&views_dir);
    assert!(!rs_files.is_empty(), "expected views/*.rs files");
    let mut violations: Vec<String> = Vec::new();
    for path in &rs_files {
        let body = common::read_to_string_or_panic(path);
        let code = common::strip_rust_comments(&body);
        // @step Then NO file imports `codelet_napi::` or `codelet_core::` or `tarpc::` or `tokio_tungstenite::`
        for needle in [
            "codelet_napi::",
            "codelet_core::",
            "tarpc::",
            "tokio_tungstenite::",
        ] {
            if code.contains(needle) {
                violations.push(format!("{}: {}", path.display(), needle));
            }
        }
        // @step And NO file contains `tokio::runtime::Builder` or `Runtime::new()`
        for needle in ["tokio::runtime::Builder", "Runtime::new()"] {
            if code.contains(needle) {
                violations.push(format!("{}: {}", path.display(), needle));
            }
        }
    }
    let _ = read_stripped("lib.rs"); // exercise helper
    assert!(
        violations.is_empty(),
        "RPC-016 must preserve transport-encapsulation + host-runtime invariants. Violations: {violations:?}"
    );
}
