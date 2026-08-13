//! RPC-023 — Source-shape regressions for the mouse-handling port.
//!
//! Feature: spec/features/rpc023-source-shape.feature
//!
//! Pins the file layout, identifier-locality invariants, and 300 LoC
//! ceiling for the new mouse subsystem. Also enforces that the Action
//! enum gained the three new variants and that dialog-priority
//! components remain Event::Key-only.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

fn src_dir() -> std::path::PathBuf {
    common::workspace_root().join("fspec-tui").join("src")
}

fn read_raw(rel: &str) -> String {
    let path = src_dir().join(rel);
    common::read_to_string_or_panic(&path)
}

fn read_stripped(rel: &str) -> String {
    let body = read_raw(rel);
    common::strip_rust_comments(&body)
}

fn count_lines(path: &std::path::Path) -> usize {
    common::read_to_string_or_panic(path).lines().count()
}

/// Scenario: rust/fspec-tui/src/mouse module exists with the expected files
#[test]
fn mouse_module_exists_with_the_expected_files() {
    // @step Given the fspec-tui crate after RPC-023 lands
    // @step When a developer scans the src/ directory
    let mouse = src_dir().join("mouse");
    // @step Then the file rust/fspec-tui/src/mouse/mod.rs exists
    assert!(
        mouse.join("mod.rs").exists(),
        "rust/fspec-tui/src/mouse/mod.rs must exist after RPC-023"
    );
    // @step And the file rust/fspec-tui/src/mouse/hit_test.rs exists
    assert!(
        mouse.join("hit_test.rs").exists(),
        "rust/fspec-tui/src/mouse/hit_test.rs must exist after RPC-023"
    );
    // @step And the file rust/fspec-tui/src/mouse/toggle.rs exists
    assert!(
        mouse.join("toggle.rs").exists(),
        "rust/fspec-tui/src/mouse/toggle.rs must exist after RPC-023"
    );
}

/// Scenario: No raw SGR mouse escape strings appear outside terminal.rs
#[test]
fn no_raw_sgr_mouse_escape_strings_appear_anywhere_in_src() {
    // @step Given the directory rust/fspec-tui/src
    let rs_files = common::collect_rs_files(&src_dir());
    assert!(!rs_files.is_empty(), "expected at least one .rs file");
    // @step When a test scans every .rs file with comments stripped
    let needles = [
        "\\x1b[?1000h",
        "\\x1b[?1006h",
        "\\x1b[?1006l",
        "\\x1b[?1000l",
    ];
    let mut violations: Vec<String> = Vec::new();
    for path in &rs_files {
        let body = common::read_to_string_or_panic(path);
        let code = common::strip_rust_comments(&body);
        for needle in needles {
            if code.contains(needle) {
                violations.push(format!("{}: {needle}", path.display()));
            }
        }
    }
    // @step Then no file contains the literal byte sequence "\x1b[?1000h"
    // @step And no file contains the literal byte sequence "\x1b[?1006h"
    // @step And no file contains the literal byte sequence "\x1b[?1006l"
    // @step And no file contains the literal byte sequence "\x1b[?1000l"
    assert!(
        violations.is_empty(),
        "raw SGR mouse escape strings must not appear in rust/fspec-tui/src — crossterm owns the protocol. Violations: {violations:?}"
    );
}

/// Scenario: EnableMouseCapture and DisableMouseCapture appear only in terminal.rs and mouse/toggle.rs
#[test]
fn enable_disable_mouse_capture_appear_only_in_terminal_and_mouse_toggle() {
    // @step Given the directory rust/fspec-tui/src
    let rs_files = common::collect_rs_files(&src_dir());
    let terminal = src_dir().join("terminal.rs");
    let toggle = src_dir().join("mouse").join("toggle.rs");
    // @step When a test scans every .rs file with comments stripped for EnableMouseCapture / DisableMouseCapture identifiers
    let mut violations: Vec<String> = Vec::new();
    for path in &rs_files {
        if path == &terminal || path == &toggle {
            continue;
        }
        let body = common::read_to_string_or_panic(path);
        let code = common::strip_rust_comments(&body);
        for needle in ["EnableMouseCapture", "DisableMouseCapture"] {
            if code.contains(needle) {
                violations.push(format!("{}: {needle}", path.display()));
            }
        }
    }
    // @step Then the only files containing these identifiers are src/terminal.rs and src/mouse/toggle.rs
    assert!(
        violations.is_empty(),
        "EnableMouseCapture / DisableMouseCapture must only appear in terminal.rs + mouse/toggle.rs. Violations: {violations:?}"
    );
}

/// Scenario: Dialog-priority components match Event::Key exclusively
#[test]
fn dialog_priority_components_match_event_key_exclusively() {
    // @step Given the source of components/disconnect_dialog.rs and components/help_dialog.rs
    let disconnect = read_stripped("components/disconnect_dialog.rs");
    let help = read_stripped("components/help_dialog.rs");
    // @step When a test scans the stripped source for Event::Mouse pattern arms
    // @step Then neither file contains an Event::Mouse match arm
    assert!(
        !disconnect.contains("Event::Mouse"),
        "components/disconnect_dialog.rs must NOT contain Event::Mouse match arms — dialogs are Event::Key only until RPC-022"
    );
    assert!(
        !help.contains("Event::Mouse"),
        "components/help_dialog.rs must NOT contain Event::Mouse match arms — dialogs are Event::Key only until RPC-022"
    );
}

/// Scenario: Mouse module files and views/board.rs stay under 300 lines
#[test]
fn mouse_module_files_and_views_board_rs_stay_under_300_lines() {
    // @step Given rust/fspec-tui/src/mouse/*.rs and rust/fspec-tui/src/views/board.rs after RPC-023 lands
    let mouse_dir = src_dir().join("mouse");
    let mut targets: Vec<std::path::PathBuf> = vec![src_dir().join("views").join("board.rs")];
    let entries = std::fs::read_dir(&mouse_dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", mouse_dir.display()));
    for entry in entries {
        let entry = entry.expect("read_dir entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            targets.push(path);
        }
    }
    targets.sort();
    // @step When a test counts the lines in each file
    let mut violations = Vec::new();
    for path in &targets {
        let lines = count_lines(path);
        if lines >= 300 {
            violations.push(format!("{}: {lines} lines >= 300", path.display()));
        }
    }
    // @step Then each file has fewer than 300 lines
    assert!(
        violations.is_empty(),
        "RPC-023 source files MUST stay < 300 LoC. Violations: {violations:?}"
    );
}

/// Scenario: Action enum gains the three new variants
#[test]
fn action_enum_gains_the_three_new_variants() {
    // @step Given rust/fspec-tui/src/components/mod.rs after RPC-023 lands
    let body = read_raw("components/mod.rs");
    // @step When a developer reads the file source raw
    // @step Then the file contains the substring "SetFocusedColumn"
    assert!(
        body.contains("SetFocusedColumn"),
        "Action enum must include `SetFocusedColumn(usize)` after RPC-023"
    );
    // @step And the file contains the substring "SelectIndexInFocused"
    assert!(
        body.contains("SelectIndexInFocused"),
        "Action enum must include `SelectIndexInFocused(usize)` after RPC-023"
    );
    // @step And the file contains the substring "ReEnableMouseTracking"
    assert!(
        body.contains("ReEnableMouseTracking"),
        "Action enum must include `ReEnableMouseTracking(String)` after RPC-023"
    );
}

/// Scenario: rect_contains is half-open on the right and bottom edges
///
/// This scenario also lives as an inline unit test in
/// `src/mouse/hit_test.rs`; the integration test here lets `cargo test
/// --test source_shape_rpc023` cover it without rebuilding the inline
/// unit module.
#[test]
fn rect_contains_is_half_open_on_the_right_and_bottom_edges() {
    use codelet_fspec_tui::mouse::rect_contains;
    use ratatui::layout::Rect;

    // @step Given a Rect with x=5, y=5, width=10, height=10
    let r = Rect {
        x: 5,
        y: 5,
        width: 10,
        height: 10,
    };
    // @step When rect_contains is evaluated for several points
    // @step Then rect_contains returns true for (5, 5)
    assert!(rect_contains(r, 5, 5), "(5,5) should be inside");
    // @step And rect_contains returns true for (14, 14)
    assert!(rect_contains(r, 14, 14), "(14,14) should be inside");
    // @step And rect_contains returns false for (15, 14)
    assert!(!rect_contains(r, 15, 14), "(15,14) is past the right edge");
    // @step And rect_contains returns false for (14, 15)
    assert!(!rect_contains(r, 14, 15), "(14,15) is past the bottom edge");
    // @step And rect_contains returns false for (4, 5)
    assert!(!rect_contains(r, 4, 5), "(4,5) is before the left edge");
}
