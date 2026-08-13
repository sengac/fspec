#![cfg(not(feature = "noop"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/compression-ratio-clamping.feature
//
// This test file validates the acceptance criteria defined in the feature file.
// Scenarios map directly to Gherkin scenarios.
//
// CMPCT-039: the shared `compression_ratio()` helper in
// rust/cli/src/interactive_helpers.rs must clamp its result to [0.0, 1.0]
// so no producer can ever ship a negative ratio on the wire. This single
// test crate covers the helper AND both live producers because codelet-napi
// is the one test crate that depends on codelet-cli (the helper),
// codelet-sessions (compact_session / embedded + websocket transports) and
// codelet-napi (session_compact) at once — the same rationale as
// cmpct038_measurement_basis_test.rs.
//
// Written BEFORE the clamp lands — the growth-case tests MUST FAIL against
// the unclamped helper (red phase): the helper returns -0.6 for (1000, 1600)
// and both producers ship large negative percentages for tiny sessions.

use std::sync::Arc;

use codelet_cli::interactive_helpers::compression_ratio;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use rig::message::{Message, UserContent};
use rig::OneOrMany;
use tokio::sync::Mutex;

/// Serializes the tests that swap the process-global data directory
/// (`codelet_common::set_data_directory`), mirroring the DATA_DIR_GUARD
/// pattern from rust/sessions/tests/rpc418_compact_session.rs.
static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

/// Seed a session's inner conversation down to a single tiny user message
/// with a token count far below the injected compaction instruction.
async fn seed_tiny_session(session: &codelet_sessions::background_session::BackgroundSession) {
    let mut inner = session.inner.lock().await;
    inner.messages.clear();
    inner.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("hi")),
    });
    // Tiny original context: far below the tokens of the compaction
    // instruction that execute_compaction injects.
    inner.token_tracker.input_tokens = 10;
    inner.token_tracker.output_tokens = 0;
}

// ========================================
// Scenario: Helper clamps a context-growth ratio to zero
// ========================================

#[test]
fn test_helper_clamps_context_growth_ratio_to_zero() {
    // @step Given a compaction where the compacted token count 1600 exceeds the original token count 1000
    let original_tokens: u64 = 1000;
    let compacted_tokens: u64 = 1600;

    // @step When the compression ratio is calculated by the shared helper
    let ratio = compression_ratio(original_tokens, compacted_tokens);

    // @step Then the helper returns exactly 0.0
    assert_eq!(
        ratio, 0.0,
        "compression_ratio(1000, 1600) must clamp to exactly 0.0, got {ratio}"
    );

    // @step And the helper never returns a negative value
    assert!(
        ratio >= 0.0,
        "compression_ratio must never return a negative value, got {ratio}"
    );
}

// ========================================
// Scenario: Helper reports a normal reduction unchanged
// ========================================

#[test]
fn test_helper_reports_normal_reduction_unchanged() {
    // @step Given a compaction where the original token count 1000 shrinks to a compacted token count of 400
    let original_tokens: u64 = 1000;
    let compacted_tokens: u64 = 400;

    // @step When the compression ratio is calculated by the shared helper
    let ratio = compression_ratio(original_tokens, compacted_tokens);

    // @step Then the helper returns 0.6 representing 60 percent of tokens removed
    assert!(
        (ratio - 0.6).abs() < 1e-12,
        "compression_ratio(1000, 400) must be 0.6 (60% removed), got {ratio}"
    );
}

// ========================================
// Scenario: Helper returns zero when the original token count is zero
// ========================================

#[test]
fn test_helper_returns_zero_when_original_is_zero() {
    // @step Given a compaction where the original token count is 0
    let original_tokens: u64 = 0;
    let compacted_tokens: u64 = 500;

    // @step When the compression ratio is calculated by the shared helper
    let ratio = compression_ratio(original_tokens, compacted_tokens);

    // @step Then the helper returns exactly 0.0 via the division guard
    assert_eq!(
        ratio, 0.0,
        "compression_ratio(0, 500) must return 0.0 via the zero-original guard, got {ratio}"
    );
}

// ========================================
// Scenario: Helper returns zero when compacted tokens equal original tokens
// ========================================

#[test]
fn test_helper_returns_zero_when_compacted_equals_original() {
    // @step Given a compaction where the compacted token count equals the original token count of 800
    let original_tokens: u64 = 800;
    let compacted_tokens: u64 = 800;

    // @step When the compression ratio is calculated by the shared helper
    let ratio = compression_ratio(original_tokens, compacted_tokens);

    // @step Then the helper returns exactly 0.0
    assert_eq!(
        ratio, 0.0,
        "compression_ratio(800, 800) must be exactly 0.0 (no reduction), got {ratio}"
    );
}

// ========================================
// Scenario: compact_session RPC result never ships a negative ratio when
// context grows
// ========================================
//
// Exercises the REAL sync trait method `compact_session` through
// `SessionManagerHandle` on a fresh SessionManager (Noop hooks — no agent
// loop). This is the producer at rust/sessions/src/handle_impl.rs that
// feeds both the embedded and websocket transports. multi-thread runtime is
// REQUIRED: compact_session bridges sync->async via block_in_place.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_compact_session_growth_case_ships_zero_not_negative_ratio() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());

    // @step Given a tiny session whose injected compaction instruction exceeds its original token count
    manager.set_default_model("anthropic/claude-opus-4-5");
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid: SessionId = handle.create_session(None);
    {
        let session = manager.get_session(&sid.value).expect("session must exist");
        seed_tiny_session(&session).await;
    }

    // @step When the user compacts the session through the session manager handle
    let result = handle
        .compact_session(&sid)
        .expect("compact_session must succeed on a populated session");

    // RPC-421: the RPC result is now an acknowledgement (compacted_tokens 0);
    // the old `compacted_tokens > original_tokens` growth probe moved to the
    // CompactionComplete chunk producers. The non-negativity guarantee under
    // context growth is what this scenario pins.

    // @step Then the returned CompactionResult compression_ratio is exactly 0.0
    assert_eq!(
        result.compression_ratio, 0.0,
        "compression_ratio must clamp to exactly 0.0 when context grows, got {}",
        result.compression_ratio
    );

    // @step And the returned CompactionResult compression_ratio is not negative
    assert!(
        result.compression_ratio >= 0.0,
        "compression_ratio must never be negative on the wire, got {}",
        result.compression_ratio
    );
}

// ========================================
// Scenario: NAPI session_compact result never ships a negative ratio when
// context grows
// ========================================
//
// Exercises the REAL `session_compact` NAPI binding against the global
// SessionManager singleton (the same instance the binding looks up). Hooks
// are the default Noop hooks in this test binary, so no agent loop is
// spawned and the post-compaction `send_input("Continue")` failure is
// logged, not fatal.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_napi_session_compact_growth_case_ships_zero_not_negative_ratio() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = codelet_napi::session_bindings::SessionManager::instance();

    // @step Given a tiny NAPI session whose injected compaction instruction exceeds its original token count
    manager.set_default_model("anthropic/claude-opus-4-5");
    let handle: &dyn SessionManagerHandle = manager;
    let sid = handle.create_session(None);
    {
        let session = manager.get_session(&sid.value).expect("session must exist");
        seed_tiny_session(&session).await;
    }

    // @step When the user compacts the session through the NAPI session_compact binding
    let result = codelet_napi::session_compact(sid.value.clone())
        .await
        .expect("session_compact must succeed on a populated session");

    // RPC-421: the RPC result is now an acknowledgement (compacted_tokens 0);
    // see the sessions-handle scenario above for the rationale.

    // @step Then the returned CompactionResult compression_ratio is exactly 0.0
    assert_eq!(
        result.compression_ratio, 0.0,
        "compression_ratio must clamp to exactly 0.0 when context grows, got {}",
        result.compression_ratio
    );

    // @step And the returned CompactionResult compression_ratio is not negative
    assert!(
        result.compression_ratio >= 0.0,
        "compression_ratio must never be negative on the wire, got {}",
        result.compression_ratio
    );
}
