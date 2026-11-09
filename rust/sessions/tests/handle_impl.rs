//! Functional and source-shape tests for the RPC-042
//! `SessionManagerHandle` impl on the extracted `SessionManager`.
//!
//! Feature: spec/features/implement-sessionmanagerhandle-for-codelet-sessions-sessionmanager.feature
//!
//! The functional tests construct a real `codelet_sessions::SessionManager`,
//! cast it to `Arc<dyn codelet_core::SessionManagerHandle>`, and drive
//! every trait method against an unknown session id to verify the
//! safe-default semantics required by RPC-042. They use a real tokio
//! runtime because `create_session` / `create_isolated_session` bridge
//! sync→async via `tokio::runtime::Handle::current().block_on(...)`.
//!
//! The shape tests (`scenario_impl_block_exists_with_every_override`,
//! `scenario_conversions_module_exists`,
//! `scenario_build_and_dependency_rule_invariants`) enforce the static
//! structural contract by inspecting `rust/sessions/src/*.rs` files
//! and asserting via grep-style substring matches that the impl block,
//! the `uuid_from` helper, and the conversions module are all present.
//!
//! Note on file layout: Rule [8] of RPC-042 names a separate
//! `handle_impl_shape.rs` sibling file (mirroring RPC-039's
//! `background_session_shape.rs` and RPC-040's
//! `session_manager_shape.rs`). The current fspec coverage tooling
//! enforces a 1 feature ↔ 1 test file mapping, so both functional and
//! shape tests are co-located here to satisfy that 1:1 constraint
//! while still preserving every shape assertion the spec requires.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{
    ApprovalChoice, FspecResult, HitlResponse, LogRecord, PauseKind as RpcPauseKind, SessionId,
    SessionStatus, StreamChunk, ThinkingLevel, TokenRestoreState, WorkUnitContext,
};
use codelet_sessions::conversions::{
    approval_choice_to_pause_response, confirm_accept_to_pause_response, pause_state_to_rpc,
};
use codelet_sessions::SessionManager;
use codelet_tools::tool_pause::{
    PauseKind as ToolPauseKind, PauseResponse as ToolPauseResponse, PauseState as ToolPauseState,
};

fn make_handle() -> Arc<dyn SessionManagerHandle> {
    SessionManager::new() as Arc<dyn SessionManagerHandle>
}

// =============================================================================
// Shape-test helpers (file inspection for the structural scenarios)
// =============================================================================

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("codelet-sessions manifest dir must have a parent")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    codelet_test_helpers::source_composition::read_tree_or_file(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn sessions_src_dir() -> PathBuf {
    workspace_root().join("sessions").join("src")
}

/// Concatenate the source bytes of every Rust file directly under
/// `rust/sessions/src/` so the shape test accepts either placement
/// (single-file `session_manager.rs` or split into `handle_impl.rs`).
fn sessions_src_concat() -> String {
    let dir = sessions_src_dir();
    // Sort entries for a deterministic concatenation order. `read_dir` yields
    // entries in filesystem order, which made `after_impl` (everything after
    // the impl marker) depend on where each file landed — adding a new src
    // file could flip whether an unrelated module's block_on bridge fell
    // before or after the marker. Sorting keeps the block_on count stable.
    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .map(|entry| entry.expect("dir entry").path())
        .collect();
    paths.sort();
    // Each top-level file is read with its own module tree, so lib.rs (the
    // whole crate) and files another top-level file pulls in through
    // `#[path = "..."]` (e.g. profile_discovery.rs) would be counted twice.
    let nested: Vec<String> = paths
        .iter()
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .flat_map(|text| {
            text.lines()
                .filter_map(|line| line.trim().strip_prefix("#[path = \""))
                .filter_map(|rest| rest.split('"').next().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .collect();
    let mut out = String::new();
    for path in paths {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let skip = name == "lib.rs" || nested.iter().any(|nested| nested == name);
        if path.extension().and_then(|s| s.to_str()) == Some("rs") && !skip {
            out.push_str(&read(&path));
            out.push('\n');
        }
    }
    out
}

const TRAIT_METHODS: &[&str] = &[
    "list_sessions",
    "create_session_checked",
    "send_input",
    "send_input_with_thinking",
    "interrupt",
    "get_session_status",
    "chunks_rx",
    "logs_rx",
    "chunks_tx",
    "logs_tx",
    "status_changes_rx",
    "status_changes_tx",
    "get_session_tokens",
    "get_session_model",
    "get_compaction_progress",
    "get_buffered_output",
    "clear_history",
    "compact_session",
    "restore_session_messages",
    "restore_session_token_state",
    "get_work_unit_context",
    "set_work_unit_context",
    "get_pending_input",
    "set_pending_input",
    "set_active_session",
    "clear_active_session",
    "get_active_session",
    "get_effective_cwd",
    "get_supervisors",
    "get_debug_enabled",
    "set_debug_enabled",
    "toggle_debug",
    "pause_human_action",
    "pause_confirm",
    "pause_triple",
    "send_hitl_response",
    "get_pause_state",
    "get_hitl_request",
    "send_fspec_result",
    "create_isolated_session",
    "destroy_session",
    "set_thinking_level_default",
];

#[path = "handle_impl/broadcasts_round_trip.rs"]
mod broadcasts_round_trip;
#[path = "handle_impl/build_and_dependency_rule.rs"]
mod build_and_dependency_rule;
#[path = "handle_impl/impl_block_exists_with.rs"]
mod impl_block_exists_with;
#[path = "handle_impl/missing_session_input.rs"]
mod missing_session_input;
#[path = "handle_impl/session_manager_satisfies_trait.rs"]
mod session_manager_satisfies_trait;
#[path = "cmpct060/support.rs"]
pub mod support;
