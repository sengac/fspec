//! RPC-013 — Source-shape regression for the view-aware footer refactor.
//!
//! Feature: spec/features/rpc013-source-shape.feature
//!
//! Pins:
//!   - codelet/fspec-tui/src/views/footer.rs is deleted.
//!   - `FooterView` identifier is removed from views/mod.rs, lib.rs,
//!     app/state.rs (after comment stripping).
//!   - Navigator::render_with_stores no longer constrains a Length(1)
//!     footer row.
//!   - AgentView::render_with_store splits into Min(0) + Length(3) +
//!     Length(1) and paints the placeholder footer literal.
//!   - File-size invariant (< 300 LoC) preserved for every modified
//!     view file.

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
    let path = src_dir().join(rel);
    let body = common::read_to_string_or_panic(&path);
    common::strip_rust_comments(&body)
}

fn count_lines(rel: &str) -> usize {
    let path = src_dir().join(rel);
    common::read_to_string_or_panic(&path).lines().count()
}

/// Scenario: FooterView module and its re-exports are removed
#[test]
fn footer_view_module_and_re_exports_are_removed() {
    // @step Given the codelet/fspec-tui crate after RPC-013 lands
    // @step When a developer scans the crate source tree
    // @step Then the file codelet/fspec-tui/src/views/footer.rs does NOT exist
    let footer = src_dir().join("views").join("footer.rs");
    assert!(
        !footer.exists(),
        "codelet/fspec-tui/src/views/footer.rs must not exist after RPC-013"
    );
    // @step And codelet/fspec-tui/src/views/mod.rs does NOT contain the identifier "FooterView"
    let mod_rs = read_stripped("views/mod.rs");
    assert!(
        !mod_rs.contains("FooterView"),
        "views/mod.rs must not reference FooterView after RPC-013"
    );
    // @step And codelet/fspec-tui/src/lib.rs does NOT contain the identifier "FooterView"
    let lib_rs = read_stripped("lib.rs");
    assert!(
        !lib_rs.contains("FooterView"),
        "lib.rs must not reference FooterView after RPC-013"
    );
    // @step And codelet/fspec-tui/src/app/state.rs does NOT contain the identifier "FooterView"
    let state_rs = read_stripped("app/state.rs");
    assert!(
        !state_rs.contains("FooterView"),
        "app/state.rs must not reference FooterView after RPC-013"
    );
}

/// Scenario: Navigator no longer reserves a Length(1) footer row
#[test]
fn navigator_no_longer_reserves_a_length_1_footer_row() {
    // @step Given the Navigator render path in codelet/fspec-tui/src/views/navigator.rs
    // @step When a developer scans the render_with_stores method body
    let nav = read_stripped("views/navigator.rs");
    // @step Then the method does NOT contain "Constraint::Length(1)" anywhere
    assert!(
        !nav.contains("Constraint::Length(1)"),
        "navigator.rs must not contain Constraint::Length(1) after RPC-013"
    );
    // @step And the method does NOT reference `self.footer`
    assert!(
        !nav.contains("self.footer"),
        "navigator.rs must not reference self.footer after RPC-013"
    );
}

/// Scenario: AgentView splits its area into scrollback + input + footer rows
#[test]
fn agent_view_splits_into_scrollback_input_and_footer_rows() {
    // @step Given an AgentView module at codelet/fspec-tui/src/views/agent.rs
    // @step When a developer scans the render_with_store method body
    let agent = read_stripped("views/agent.rs");
    // @step Then the method contains a Layout split with a Min(0) flex row and a trailing Length(1) footer row
    //
    // RPC-019 update: the input row's exact constraint moved from a
    // hardcoded `Constraint::Length(3)` to a variable that tracks the
    // textarea's `visible_rows()` (default cap 6). The flex Min(0)
    // scrollback row and the trailing Length(1) footer row are the
    // structural invariants this scenario now pins.
    assert!(
        agent.contains("Constraint::Min(0)"),
        "agent.rs must contain a Constraint::Min(0) flex row after RPC-013/RPC-019"
    );
    assert!(
        agent.contains("Constraint::Length(1)"),
        "agent.rs must contain a Constraint::Length(1) footer row after RPC-013"
    );
    assert!(
        agent.contains("Constraint::Length(input_height)"),
        "agent.rs must size the input row from MultiLineInput::visible_rows() after RPC-019"
    );
    // @step And the bottom 1-row chunk is painted with the placeholder footer string
    //       "Enter=send  Ctrl+C=interrupt  ESC=back"
    //
    // Note: after RPC-018 the literal lives in
    // views/agent::PLACEHOLDER_FOOTER_HINTS and is re-exported by the
    // SessionFooter widget. Checking the components individually
    // keeps the pin robust against whitespace shifts inside the
    // literal.
    assert!(
        agent.contains("Enter=send"),
        "agent.rs (or its re-export chain) must render 'Enter=send'"
    );
    assert!(
        agent.contains("Ctrl+C=interrupt"),
        "agent.rs (or its re-export chain) must render 'Ctrl+C=interrupt'"
    );
    assert!(
        agent.contains("ESC=back"),
        "agent.rs (or its re-export chain) must render 'ESC=back'"
    );
}

/// Scenario: File-size invariant preserved for every modified view file
#[test]
fn every_modified_view_file_stays_under_300_loc() {
    // @step Given the directory codelet/fspec-tui/src/views/
    // @step When a test counts the line-count of every .rs file under that directory
    let targets = [
        // @step Then views/board.rs has fewer than 300 lines
        "views/board.rs",
        // @step And views/agent.rs has fewer than 300 lines
        "views/agent.rs",
        // @step And views/navigator.rs has fewer than 300 lines
        "views/navigator.rs",
        // @step And views/mod.rs has fewer than 300 lines
        "views/mod.rs",
    ];
    let mut violations = Vec::new();
    for rel in targets {
        let lines = count_lines(rel);
        if lines >= 300 {
            violations.push(format!("{rel}: {lines} lines >= 300 ceiling"));
        }
    }
    assert!(
        violations.is_empty(),
        "RPC-013 modified files MUST stay < 300 LoC. Violations: {violations:?}"
    );
}

/// Scenario: BoardView source contains the literal UnifiedBoardLayout footer string
///
/// NOTE: We read the source RAW (no comment stripping) because the
/// `strip_rust_comments` helper operates byte-by-byte and corrupts the
/// multi-byte UTF-8 arrows used in the footer literal. The doc comments
/// in views/board.rs do NOT contain the legacy '? help' / 'switch pane'
/// strings (verified independently below), so raw reading is safe.
///
/// RPC-016 widened this scenario: the footer literal moved into the
/// sibling `views/board/footer.rs` module so `views/board.rs` could stay
/// under the 300 LoC ceiling. The scenario now scans both files.
#[test]
fn board_view_source_contains_literal_footer_string() {
    // @step Given the BoardView module at codelet/fspec-tui/src/views/board.rs
    // @step When a developer scans the source after comment stripping
    let board = read_raw("views/board.rs");
    let footer = read_raw("views/board/footer.rs");
    let combined = format!("{board}\n{footer}");
    // @step Then the file contains the substring "← → Columns"
    assert!(combined.contains("← →"), "board.rs|footer.rs must contain the '← →' key span");
    assert!(combined.contains("Columns"), "board.rs|footer.rs must contain the 'Columns' label span");
    // @step And the file contains the substring "↑↓ Work Units"
    assert!(combined.contains("↑↓"), "board.rs|footer.rs must contain the '↑↓' key span");
    assert!(combined.contains("Work Units"), "board.rs|footer.rs must contain the 'Work Units' label span");
    // @step And the file contains the substring "[ Priority Up"
    assert!(combined.contains("Priority Up"), "board.rs|footer.rs must contain the 'Priority Up' label span");
    // @step And the file contains the substring "] Priority Down"
    assert!(combined.contains("Priority Down"), "board.rs|footer.rs must contain the 'Priority Down' label span");
    // @step And the file contains the substring "↵ Work Agent"
    assert!(combined.contains("↵"), "board.rs|footer.rs must contain the '↵' key span");
    assert!(combined.contains("Work Agent"), "board.rs|footer.rs must contain the 'Work Agent' label span");
    // @step And the file contains the substring "ESC Back"
    assert!(combined.contains("ESC"), "board.rs|footer.rs must contain the 'ESC' key span");
    assert!(combined.contains("Back"), "board.rs|footer.rs must contain the 'Back' label span");
    // @step And the file does NOT contain the substring "? help"
    let stripped = read_stripped("views/board.rs");
    assert!(!stripped.contains("? help"), "board.rs (code) must not contain '? help' after RPC-013");
    // @step And the file does NOT contain the substring "switch pane"
    assert!(!stripped.contains("switch pane"), "board.rs (code) must not contain 'switch pane' after RPC-013");
}

/// Internal sanity (no scenario): identical to the BoardView-source
/// scenario above, retained to keep failure messages clear for the
/// legacy hint check independently of the literal-string check.
#[test]
fn board_view_source_does_not_contain_legacy_generic_hint() {
    let board = read_stripped("views/board.rs");
    assert!(!board.contains("? help"), "board.rs must not contain '? help' after RPC-013");
    assert!(!board.contains("switch pane"), "board.rs must not contain 'switch pane' after RPC-013");
}
