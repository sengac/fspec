// Feature: spec/features/session-persistence-with-fork-and-merge.feature
//
// Session Persistence with Fork and Merge
// Tests for the Rust persistence module
//
// IMPORTANT: These tests use isolated temporary directories to avoid
// polluting ~/.fspec with test data. Each test gets its own temp dir
// via setup_test_env().
//
// RPC-035: relocated from rust/napi/src/persistence/tests.rs into
// codelet-core so the lifted persistence layer owns its own tests.
// `use super::*;` becomes `use crate::persistence::*;` (the canonical
// codelet-core import path) and the setup helper calls
// codelet_common::set_data_directory + reset_stores_for_tests directly
// instead of routing through the deleted NAPI shim.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    // RPC-035: the relocated tests use the pre-RPC-035 `format!("X {}", y)`
    // style throughout. Switching to the inlined-format-args style is purely
    // cosmetic and would obscure the relocation diff. Allow the lint here.
    clippy::uninlined_format_args
)]

use crate::persistence::*;
use std::path::PathBuf;
use std::sync::Mutex;

// Global mutex to ensure tests run sequentially since they share global state
// (MESSAGE_STORE, SESSION_STORE, DATA_DIRECTORY are all global singletons)
lazy_static::lazy_static! {
    pub(super) static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

/// Setup an isolated temp directory for a test.
/// Returns a guard (for sequential execution) and a TempDir that will be
/// cleaned up when dropped.
///
/// MUST be called at the start of every test to ensure:
/// 1. Tests don't pollute ~/.fspec with test data
/// 2. Tests don't interfere with each other
///
/// RPC-035: replaces the previous `crate::persistence::set_data_directory`
/// indirection (which doubled as a credentials + knowledge-graph reset in
/// the NAPI shim) with the codelet-core-only sequence:
/// `codelet_common::set_data_directory` + `reset_stores_for_tests()`.
/// Credentials + knowledge graph are NAPI-only concerns and are reset by
/// codelet-napi's `test_support.rs::setup_test_env` for the bridge tests.
fn setup_test_env() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
    let guard = TEST_MUTEX.lock().unwrap();
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(temp_dir.path().to_path_buf())
        .expect("Failed to set data directory");
    reset_stores_for_tests();
    (guard, temp_dir)
}

// ============================================================================
// NAPI-008: Message Envelope Blob Storage Integration Tests
// ============================================================================

// ============================================================================
// NAPI-008: Token Usage and Compaction State Persistence Tests
// ============================================================================

// ============================================================================
// RPC-049: `get_session_message_envelopes` lifted from codelet-napi into
// codelet-core. Validates the public free function exists and produces
// envelopes that parse as MessageEnvelope JSON.
// ============================================================================

#[path = "tests/blob_deduplication_across_envelopes.rs"]
mod blob_deduplication_across_envelopes;
#[path = "tests/compacted_session_envelopes_can.rs"]
mod compacted_session_envelopes_can;
#[path = "tests/image_blob_storage_and.rs"]
mod image_blob_storage_and;
#[path = "tests/merge_preserves_references.rs"]
mod merge_preserves_references;
#[path = "tests/one_byte_over_threshold.rs"]
mod one_byte_over_threshold;
#[path = "tests/resume_compacted_session.rs"]
mod resume_compacted_session;
#[path = "tests/resume_session_after_closing.rs"]
mod resume_session_after_closing;
#[path = "tests/search_command_history.rs"]
mod search_command_history;
#[path = "tests/synthetic_compaction_envelope_parses.rs"]
mod synthetic_compaction_envelope_parses;
#[path = "tests/thinking_blob_storage_and.rs"]
mod thinking_blob_storage_and;
#[path = "tests/token_and_compaction_state.rs"]
mod token_and_compaction_state;
