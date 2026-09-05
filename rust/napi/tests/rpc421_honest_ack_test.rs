#![cfg(not(feature = "noop"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/honest-compaction-acknowledgement.feature
//
// This test file validates the acceptance criteria defined in the feature file.
// Scenarios map directly to Gherkin scenarios.
//
// RPC-421: compact_session (BOTH twins — rust/sessions/src/handle_impl.rs
// and rust/napi/src/session_bindings.rs session_compact) must stop shipping
// fabricated reduction numbers measured at the post-clear trough, BEFORE the
// agent builds the DAG summary. The result becomes an acknowledgement on the
// UNCHANGED wire schema: real original_tokens, compacted_tokens 0,
// compression_ratio 0.0, turns 0. The CompactionComplete chunk (CMPCT-038
// apply-site emission) is the single source of truth for the numbers.
//
// This is the only test crate that sees codelet-cli, codelet-sessions and
// codelet-napi at once — same rationale as cmpct038/cmpct039.
//
// Written BEFORE the implementation — all three scenarios MUST FAIL against
// the current producers, which ship the trough measurement (red phase).

use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use rig::message::{AssistantContent, Message, UserContent};
use rig::OneOrMany;
use tokio::sync::Mutex;

/// Serializes the tests that swap the process-global data directory
/// (`codelet_common::set_data_directory`), mirroring the DATA_DIR_GUARD
/// pattern from cmpct039_ratio_clamp_test.rs.
static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

/// Seed a populated conversation with several user/assistant messages and a
/// non-zero input-token count, mirroring rpc418_compact_session.rs.
async fn seed_populated(session: &codelet_sessions::background_session::BackgroundSession) {
    let mut inner = session.inner.lock().await;
    inner.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Please refactor the auth module")),
    });
    inner.messages.push(Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::text("Sure, reading the files now")),
    });
    inner.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Also add rate limiting")),
    });
    inner.messages.push(Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::text("Done, here is the plan")),
    });
    inner.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Looks good, ship it")),
    });
    inner.token_tracker.input_tokens = 5000;
    inner.token_tracker.output_tokens = 1000;
}

// ========================================
// Scenario: compact_session returns an acknowledgement instead of fabricated
// reduction numbers
// ========================================
//
// Exercises the REAL sync trait method `compact_session` through
// `SessionManagerHandle` on a fresh SessionManager (Noop hooks — no agent
// loop). multi-thread runtime is REQUIRED: compact_session bridges
// sync->async via block_in_place.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_compact_session_returns_acknowledgement_not_fabricated_reduction() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());

    // @step Given a populated session with several user and assistant messages
    manager.set_default_model("anthropic/claude-opus-4-5");
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid: SessionId = handle.create_session(None);
    {
        let session = manager.get_session(&sid.value).expect("session must exist");
        seed_populated(&session).await;
    }

    // @step When the session is compacted through the session manager handle
    let result = handle
        .compact_session(&sid)
        .expect("compact_session must succeed on a populated session");

    // @step Then the returned CompactionResult original_tokens is greater than zero
    assert!(
        result.original_tokens > 0,
        "original_tokens must be the real pre-compaction snapshot, got {}",
        result.original_tokens
    );

    // @step And the returned CompactionResult compacted_tokens is exactly 0
    assert_eq!(
        result.compacted_tokens, 0,
        "compacted_tokens must be the acknowledgement sentinel 0 — the DAG \
         summary does not exist yet at RPC-return time, got {}",
        result.compacted_tokens
    );

    // @step And the returned CompactionResult compression_ratio is exactly 0.0
    assert_eq!(
        result.compression_ratio, 0.0,
        "compression_ratio must be the acknowledgement sentinel 0.0, never a \
         fabricated trough-measured reduction, got {}",
        result.compression_ratio
    );

    // @step And the returned CompactionResult turns_summarized and turns_kept are 0
    assert_eq!(result.turns_summarized, 0);
    assert_eq!(result.turns_kept, 0);
}

// ========================================
// Scenario: NAPI session_compact returns the same acknowledgement shape
// ========================================
//
// Exercises the REAL `session_compact` NAPI binding against the global
// SessionManager singleton (cmpct039 precedent). Noop hooks — the
// post-compaction `send_input("Continue")` failure is logged, not fatal.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_napi_session_compact_returns_acknowledgement_shape() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = codelet_napi::session_bindings::SessionManager::instance();

    // @step Given a populated NAPI session with several user and assistant messages
    manager.set_default_model("anthropic/claude-opus-4-5");
    let handle: &dyn SessionManagerHandle = manager;
    let sid = handle.create_session(None);
    {
        let session = manager.get_session(&sid.value).expect("session must exist");
        seed_populated(&session).await;
    }

    // @step When the session is compacted through the NAPI session_compact binding
    let result = codelet_napi::session_compact(sid.value.clone())
        .await
        .expect("session_compact must succeed on a populated session");

    // @step Then the returned CompactionResult original_tokens is greater than zero
    assert!(
        result.original_tokens > 0,
        "originalTokens must be the real pre-compaction snapshot, got {}",
        result.original_tokens
    );

    // @step And the returned CompactionResult compacted_tokens is exactly 0
    assert_eq!(
        result.compacted_tokens, 0,
        "compactedTokens must be the acknowledgement sentinel 0, got {}",
        result.compacted_tokens
    );

    // @step And the returned CompactionResult compression_ratio is exactly 0.0
    assert_eq!(
        result.compression_ratio, 0.0,
        "compressionRatio must be the acknowledgement sentinel 0.0 — no fake \
         reduction reaches JavaScript, got {}",
        result.compression_ratio
    );
}

