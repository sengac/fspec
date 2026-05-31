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
//! `codelet/fspec-tui/tests/source_shape_rpc049.rs` — a literal-string
//! grep is the cheapest, most precise way to keep an invented behaviour
//! from creeping back into the Rust port.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

/// Workspace root (one level above this crate's manifest dir).
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("codelet-fspec-tui manifest dir must have a parent")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Rust source files do not contain TS-divergent /clear strings
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn rust_source_files_do_not_contain_ts_divergent_clear_strings() {
    // @step Given the file codelet/fspec-tui/src/app/dispatch_rpc046.rs is read into memory
    let dispatch_path = workspace_root().join("fspec-tui/src/app/dispatch_rpc046.rs");
    let dispatch_body = read(&dispatch_path);

    // @step And the file codelet/core/src/session_manager_handle.rs is read into memory
    let handle_path = workspace_root()
        .parent()
        .expect("codelet/ has a parent")
        .join("codelet/core/src/session_manager_handle.rs");
    let handle_body = read(&handle_path);

    // @step When the test searches both files for the literal strings "history cleared" and "[notice] /clear"
    let dispatch_has_history_cleared = dispatch_body.contains("history cleared");
    let dispatch_has_notice_clear = dispatch_body.contains("[notice] /clear");
    let dispatch_has_error_clear_failed = dispatch_body.contains("[error] /clear failed:");
    let handle_has_quoted_history_cleared = handle_body.contains("\"history cleared\"");

    // @step Then dispatch_rpc046.rs contains neither literal
    assert!(
        !dispatch_has_history_cleared,
        "TS parity (RPC-074): dispatch_rpc046.rs must not contain `history cleared`"
    );
    assert!(
        !dispatch_has_notice_clear,
        "TS parity (RPC-074): dispatch_rpc046.rs must not contain `[notice] /clear`"
    );
    assert!(
        !dispatch_has_error_clear_failed,
        "TS parity (RPC-074): dispatch_rpc046.rs must not push `[error] /clear failed: ...` lines (errors go to tracing::error! only)"
    );

    // @step And session_manager_handle.rs does not contain the literal string "\"history cleared\""
    assert!(
        !handle_has_quoted_history_cleared,
        "TS parity (RPC-074): session_manager_handle.rs must not broadcast a \
         StreamChunk::UserNotification carrying the literal string \
         `\"history cleared\"`"
    );
}
