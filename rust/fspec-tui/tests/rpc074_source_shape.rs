//! RPC-074 — Source-shape regression: the TS-divergent
//! `[notice] /clear: history cleared` scrollback line and the
//! `UserNotification("history cleared")` broadcast that RPC-046 /
//! RPC-037 originally introduced MUST stay deleted.
//!
//! Feature: spec/features/rpc-074-source-shape-regression-divergent-clear-strings.feature
//!
//! TS reference: `src/tui/components/AgentView.tsx:1554-1564`
//! (handleClearCommand) only blanks the input + calls
//! `sessionClearHistory(currentSessionId)`. No conversation entry is
//! ever pushed. Errors route to `logger.error`, not to the user-visible
//! scrollback. The Rust port mirrors this exactly. The `Cleared` state
//! reset is driven by the `StreamChunk::SessionStateChange { state:
//! Cleared }` chunk (TUI-066 contract).
//!
//! Pattern borrowed from
//! `rust/fspec-tui/tests/source_shape_rpc049.rs` — a literal-string
//! grep is the cheapest, most precise way to keep an invented behaviour
//! from creeping back into the Rust port.

use codelet_test_helpers::{OrFail, TestResult};
use std::path::{Path, PathBuf};

/// Workspace root (one level above this crate's manifest dir).
fn workspace_root() -> Result<PathBuf, codelet_test_helpers::TestError> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    Ok(manifest_dir
        .parent()
        .or_fail("codelet-fspec-tui manifest dir must have a parent")?
        .to_path_buf())
}

fn read(path: &Path) -> Result<String, codelet_test_helpers::TestError> {
    Ok(
        codelet_test_helpers::source_composition::read_tree_or_file(path)
            .map_err(|e| format!("failed to read {}: {e}", path.display()))?,
    )
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Rust source files do not contain TS-divergent /clear strings
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn rust_source_files_do_not_contain_ts_divergent_clear_strings() -> TestResult {
    // @step Given the file rust/fspec-tui/src/app/dispatch_slash_clear.rs is read into memory
    let dispatch_path = workspace_root()?.join("fspec-tui/src/app/dispatch_slash_clear.rs");
    let dispatch_body = read(&dispatch_path)?;

    // @step And the file rust/core/src/session_manager_handle.rs is read into memory
    let handle_path = workspace_root()?
        .parent()
        .or_fail("rust/ has a parent")?
        .join("rust/core/src/session_manager_handle.rs");
    let handle_body = read_tree(&handle_path)?;

    // @step When the test searches both files for the literal strings "history cleared" and "[notice] /clear"
    let dispatch_has_history_cleared = dispatch_body.contains("history cleared");
    let dispatch_has_notice_clear = dispatch_body.contains("[notice] /clear");
    let dispatch_has_error_clear_failed = dispatch_body.contains("[error] /clear failed:");
    let handle_has_quoted_history_cleared = handle_body.contains("\"history cleared\"");

    // @step Then dispatch_slash_clear.rs contains neither literal
    assert!(
        !dispatch_has_history_cleared,
        "TS parity (RPC-074): dispatch_slash_clear.rs must not contain `history cleared`"
    );
    assert!(
        !dispatch_has_notice_clear,
        "TS parity (RPC-074): dispatch_slash_clear.rs must not contain `[notice] /clear`"
    );
    assert!(
        !dispatch_has_error_clear_failed,
        "TS parity (RPC-074): dispatch_slash_clear.rs must not push `[error] /clear failed: ...` lines (errors go to tracing::error! only)"
    );

    // @step And session_manager_handle.rs does not contain the literal string "\"history cleared\""
    assert!(
        !handle_has_quoted_history_cleared,
        "TS parity (RPC-074): session_manager_handle.rs must not broadcast a \
         StreamChunk::UserNotification carrying the literal string \
         `\"history cleared\"`"
    );
    Ok(())
}

/// REFAC-021/022: read a source file together with its include!d fragments
/// and file-backed child modules (split modules read as one text).
fn read_tree(path: &std::path::Path) -> Result<String, codelet_test_helpers::TestError> {
    Ok(
        codelet_test_helpers::source_composition::read_module_tree(path)
            .map_err(|error| format!("read {}: {error}", path.display()))?,
    )
}
