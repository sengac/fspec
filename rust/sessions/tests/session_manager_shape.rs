//! SessionManager shape tests for the `codelet-sessions` crate (RPC-040).
//!
//! Feature: spec/features/move-sessionmanager-from-codelet-napi-into-codelet-sessions.feature
//!
//! These tests codify the static shape of the moved `SessionManager`,
//! `ChainOfCommand`, the lifted `navigation` and `credentials` modules,
//! and the new `SessionManagerHooks` trait by inspecting source files
//! and manifests, building the involved crates, and running
//! `cargo metadata`. Each `#[test]` corresponds to a single Gherkin
//! scenario in the feature file; the `// @step` comments below map each
//! Gherkin step verbatim to the assertion that enforces it.
//!
//! Pattern borrowed from `rust/sessions/tests/background_session_shape.rs`
//! (RPC-039).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

// =============================================================================
// Path & file helpers (shared by every scenario in this file).
// =============================================================================

/// Workspace root (one level above this crate's manifest dir).
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

/// Path to the moved SessionManager file
/// (`rust/sessions/src/session_manager.rs`).
fn moved_sm_path() -> PathBuf {
    workspace_root()
        .join("sessions")
        .join("src")
        .join("session_manager.rs")
}

/// Path to the moved ChainOfCommand file
/// (`rust/sessions/src/chain_of_command.rs`).
fn moved_coc_path() -> PathBuf {
    workspace_root()
        .join("sessions")
        .join("src")
        .join("chain_of_command.rs")
}

/// Path to the lifted navigation module.
fn moved_nav_path() -> PathBuf {
    workspace_root()
        .join("sessions")
        .join("src")
        .join("navigation.rs")
}

/// Path to the lifted credentials mod.rs.
fn moved_creds_mod_path() -> PathBuf {
    workspace_root()
        .join("sessions")
        .join("src")
        .join("credentials")
        .join("mod.rs")
}

/// Strip `//`-style line comments so substring scans only see code.
fn strip_line_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for line in src.lines() {
        match line.find("//") {
            Some(pos) => {
                out.push_str(&line[..pos]);
                out.push('\n');
            }
            None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    out
}

/// Whitespace-insensitive substring check: removes ALL whitespace from both
/// haystack and needle before matching. This keeps structural assertions
/// robust to rustfmt line-wrapping (a `.method()` call moved onto its own
/// line, or a multi-parameter signature split across several lines) WITHOUT
/// weakening the guarantee — the required tokens must still appear in the
/// required order, just irrespective of intervening whitespace.
fn contains_ws_insensitive(haystack: &str, needle: &str) -> bool {
    fn strip_ws(s: &str) -> String {
        s.chars().filter(|c| !c.is_whitespace()).collect()
    }
    strip_ws(haystack).contains(&strip_ws(needle))
}

// Compile-time proof that the moved type paths resolve from the new home.
#[allow(unused_imports)]
use codelet_sessions::chain_of_command::ChainOfCommand;
#[allow(unused_imports)]
use codelet_sessions::credentials::resolve_and_set_env_var;
#[allow(unused_imports)]
use codelet_sessions::navigation::{
    build_navigation_list, get_next_target, get_prev_target, NavigationTarget,
};
#[allow(unused_imports)]
use codelet_sessions::session_manager::{
    NoopSessionManagerHooks, SessionManager, SessionManagerHooks,
};

// =============================================================================
// Scenario: codelet-sessions builds standalone with SessionManager,
// ChainOfCommand, navigation, credentials and the hooks trait at their new
// home.
// =============================================================================

// =============================================================================
// Scenario: The moved session_manager.rs and chain_of_command.rs have no
// napi:: references.
// =============================================================================

// =============================================================================
// Scenario: The moved session_manager.rs has no crate references to
// napi-private modules or free functions.
// =============================================================================

// =============================================================================
// Scenario: create_session_with_id is rewritten to a non-NAPI Result type
// and routes side-effects through the hooks.
// =============================================================================

// =============================================================================
// Scenario: create_isolated_session_with_id returns the wire-type
// IsolatedSessionInfo and the NAPI wrapper converts to IsolatedSessionResult.
// =============================================================================

// =============================================================================
// Scenario: SessionManager gains four new fields beyond the original five.
// =============================================================================

// =============================================================================
// Scenario: codelet-sessions has no transitive dependency on codelet-napi.
// =============================================================================

// =============================================================================
// Scenario: codelet-sessions tests assert SessionManager is reachable from
// its new home and the hooks design is sound.
// =============================================================================

// =============================================================================
// RPC-041 Scenario: SessionManager wires its manager-owned senders into every
// BackgroundSession at creation time.
// =============================================================================

// =============================================================================
// RPC-041 Scenario: The SessionManagerHooks trait no longer carries
// emit_isolation_state_change.
// =============================================================================

// =============================================================================
// RPC-041 Scenario: emit_block_notification_to_tui and spawn_footer_poller
// route through SessionManager::instance().chunks_tx().send.
// =============================================================================

// =============================================================================
// RPC-041 Scenario: The FspecHandler and bridge command_emitter gates consult
// a new is_global_chunk_callback_registered helper.
// =============================================================================

// =============================================================================
// RPC-041 Scenario: Multiple subscribers each observe every chunk emitted by a
// BackgroundSession in arrival order.
// =============================================================================

// =============================================================================
// RPC-041 Scenario: All codelet-sessions shape tests continue to pass with
// inverted GLOBAL_CHUNK_CALLBACK assertions.
// =============================================================================

#[path = "session_manager_shape/broadcast_senders_and_hooks.rs"]
mod broadcast_senders_and_hooks;
#[path = "session_manager_shape/codelet_sessions_builds_standalone.rs"]
mod codelet_sessions_builds_standalone;
#[path = "session_manager_shape/codelet_sessions_tests_assert.rs"]
mod codelet_sessions_tests_assert;
#[path = "session_manager_shape/create_session_with_id.rs"]
mod create_session_with_id;
#[path = "session_manager_shape/napi_emit_helpers_route.rs"]
mod napi_emit_helpers_route;
#[path = "session_manager_shape/session_manager_gains_four.rs"]
mod session_manager_gains_four;
