//! RPC-014 — Source-shape regression for the rich BoardView grid +
//! work-unit details strip refactor.
//!
//! Feature: spec/features/rpc014-source-shape.feature

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

/// Scenario: Grid helpers and details_strip modules exist as separate files
#[test]
fn grid_and_details_strip_modules_exist_as_separate_files() {
    // @step Given the codelet/fspec-tui crate after RPC-014 lands
    // @step When a developer scans the views directory
    // @step Then the file codelet/fspec-tui/src/views/board/grid.rs exists
    let grid = src_dir().join("views").join("board").join("grid.rs");
    assert!(grid.exists(), "codelet/fspec-tui/src/views/board/grid.rs must exist after RPC-014");
    // @step And the file codelet/fspec-tui/src/views/board/details_strip.rs exists
    let strip = src_dir().join("views").join("board").join("details_strip.rs");
    assert!(
        strip.exists(),
        "codelet/fspec-tui/src/views/board/details_strip.rs must exist after RPC-014"
    );
    // @step And the file codelet/fspec-tui/src/views/board.rs exists
    let board = src_dir().join("views").join("board.rs");
    assert!(board.exists(), "codelet/fspec-tui/src/views/board.rs must exist after RPC-014");
}

/// Scenario: New and modified board modules stay under 300 lines
#[test]
fn new_and_modified_board_modules_stay_under_300_lines() {
    // @step Given the directory codelet/fspec-tui/src/views/board/
    let board_dir = src_dir().join("views").join("board");
    let board_orchestrator = src_dir().join("views").join("board.rs");
    // @step When a test counts the line-count of every .rs file in views/board/ plus views/board.rs
    let mut targets: Vec<std::path::PathBuf> = vec![board_orchestrator];
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
    // Sanity: the directory must include the two RPC-014 modules called
    // out in rule [7] of the example map (grid.rs and details_strip.rs).
    // Any additional helper modules that landed in views/board/ are also
    // included in the < 300 LoC check by virtue of the directory scan.
    let names: Vec<String> = targets
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
        .collect();
    for required in ["grid.rs", "details_strip.rs"] {
        assert!(
            names.iter().any(|n| n == required),
            "views/board/ must contain {required}; found: {names:?}"
        );
    }
    let mut violations = Vec::new();
    for path in &targets {
        let lines = count_lines_path(path);
        if lines >= 300 {
            violations.push(format!("{}: {} lines >= 300 ceiling", path.display(), lines));
        }
    }
    // @step Then views/board.rs has fewer than 300 lines
    // @step And views/board/grid.rs has fewer than 300 lines
    // @step And views/board/details_strip.rs has fewer than 300 lines
    assert!(
        violations.is_empty(),
        "RPC-014 board modules MUST stay < 300 LoC. Violations: {violations:?}"
    );
}

/// Scenario: WorkUnitInfo gains the attachments field
#[test]
fn work_unit_info_gains_the_attachments_field() {
    // @step Given codelet/rpc-types/src/lib.rs after RPC-014 lands
    let body = read_raw(&rpc_types_lib());
    // @step When a developer reads the WorkUnitInfo struct body
    // @step Then the body contains the substring "pub attachments: Vec<String>"
    assert!(
        body.contains("pub attachments: Vec<String>"),
        "codelet/rpc-types/src/lib.rs WorkUnitInfo must add `pub attachments: Vec<String>`"
    );
}

/// Scenario: Core work_units parser reads attachments with serde default
#[test]
fn core_work_units_parser_reads_attachments_with_serde_default() {
    // @step Given codelet/core/src/work_units.rs after RPC-014 lands
    let raw = read_raw(&core_work_units());
    // @step When a developer reads the WorkUnitRecord struct body
    // @step Then the body contains the field name "attachments"
    assert!(
        raw.contains("attachments"),
        "codelet/core/src/work_units.rs must reference an `attachments` field"
    );
    // @step And the field carries a `#[serde(default)]` attribute so missing fields parse as Vec::new()
    // We assert the textual co-occurrence of `#[serde(default)]` immediately before
    // `attachments` somewhere in the file.
    let normalized = raw.replace('\n', " ").replace("    ", " ");
    let needle_a = "#[serde(default)] attachments";
    let needle_b = "#[serde(default)]  attachments";
    let needle_c = "#[serde(default)]   attachments";
    let has_default_before_attachments = normalized.contains(needle_a)
        || normalized.contains(needle_b)
        || normalized.contains(needle_c)
        || normalized.contains("#[serde(default)] pub attachments")
        || normalized.contains("#[serde(default)]  pub attachments");
    assert!(
        has_default_before_attachments,
        "codelet/core/src/work_units.rs `attachments` field must carry `#[serde(default)]`"
    );
}

/// Scenario: RPC-013 invariants preserved
#[test]
fn rpc013_invariants_preserved() {
    // @step Given codelet/fspec-tui/src/views/navigator.rs after RPC-014 lands
    let nav = read_stripped("views/navigator.rs");
    // @step Then the file does NOT contain "Constraint::Length(1)"
    assert!(
        !nav.contains("Constraint::Length(1)"),
        "navigator.rs must not re-introduce Constraint::Length(1) after RPC-013/RPC-014"
    );
    // @step And codelet/fspec-tui/src/views/mod.rs does NOT contain the identifier "FooterView"
    let mod_rs = read_stripped("views/mod.rs");
    assert!(!mod_rs.contains("FooterView"), "views/mod.rs must not reference FooterView");
    // @step And codelet/fspec-tui/src/lib.rs does NOT contain the identifier "FooterView"
    let lib_rs = read_stripped("lib.rs");
    assert!(!lib_rs.contains("FooterView"), "lib.rs must not reference FooterView");
    // @step And the file codelet/fspec-tui/src/views/footer.rs does NOT exist
    let footer = src_dir().join("views").join("footer.rs");
    assert!(!footer.exists(), "codelet/fspec-tui/src/views/footer.rs must not exist");
}

/// Scenario: BoardView still emits Action variants and renders the RPC-013 footer
#[test]
fn board_view_still_emits_action_variants_and_renders_rpc013_footer() {
    // @step Given codelet/fspec-tui/src/views/board.rs after RPC-014 lands
    let board = read_raw(&src_dir().join("views").join("board.rs"));
    // RPC-016 moved the footer literal into views/board/footer.rs so the
    // orchestrator can stay under the 300 LoC ceiling — combine both for
    // the literal-string assertions below.
    let footer = read_raw(&src_dir().join("views").join("board").join("footer.rs"));
    let combined = format!("{board}\n{footer}");
    // @step When a developer scans the file source raw
    // @step Then the file contains the substring "Action::EnterWorkUnit"
    assert!(board.contains("Action::EnterWorkUnit"), "missing Action::EnterWorkUnit in board.rs");
    // @step And the file contains the substring "Action::FocusNextColumn"
    assert!(board.contains("Action::FocusNextColumn"), "missing Action::FocusNextColumn in board.rs");
    // @step And the file contains the substring "Action::ReorderUp"
    assert!(board.contains("Action::ReorderUp"), "missing Action::ReorderUp in board.rs");
    // @step And the file contains the substring "← → Columns"
    assert!(combined.contains("← →"), "board.rs|footer.rs must contain the '← →' span");
    assert!(combined.contains("Columns"), "board.rs|footer.rs must contain 'Columns' span");
    // @step And the file contains the substring "↵ Work Agent"
    assert!(combined.contains("↵"), "board.rs|footer.rs must contain '↵' span");
    assert!(combined.contains("Work Agent"), "board.rs|footer.rs must contain 'Work Agent' span");
    // @step And the file contains the substring "ESC Back"
    assert!(combined.contains("ESC"), "board.rs|footer.rs must contain 'ESC' span");
    assert!(combined.contains("Back"), "board.rs|footer.rs must contain 'Back' span");
}

/// Scenario: Views still avoid encapsulated transport crates and host runtime construction
#[test]
fn views_still_avoid_encapsulated_transport_crates_and_runtime_construction() {
    // @step Given the directory codelet/fspec-tui/src/views/ (including the new board/ subdir)
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
    assert!(
        violations.is_empty(),
        "RPC-014 must preserve transport-encapsulation + host-runtime invariants. Violations: {violations:?}"
    );
}
