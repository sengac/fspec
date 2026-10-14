//! RPC-436 — Daemon-side session cleared broadcast (manager-level
//! liveness gate).
//!
//! Feature: spec/features/session-events-daemon-cleared-broadcast.feature
//!
//!   Scenario: Clearing an unknown session id pushes no cleared event
//!   Scenario: Clearing a live session pushes exactly one cleared event
//!     carrying its id
//!
//! This file owns the SessionManager-level scenarios of RPC-436:
//! the `session_cleared_tx` broadcast fired by the clear path ONLY when
//! the clear actually succeeded (rule R2) and the exactly-once contract
//! for a live clear (rule R3: the event fires inside
//! `BackgroundSession::clear_history` via the owning-manager
//! resolution, so every clear entry point emits exactly one event).
//! The wire-side fan-out scenarios live in the sibling feature
//! session-events-cleared-fan-out (one test file per feature, ACDD 1:1).
//!
//! RED PHASE NOTE: `SessionManager::session_cleared_tx()` and
//! `SessionManager::clear_session_history()` did not exist when this
//! file was written, so it failed to COMPILE until the implementation
//! landed. That was the intended red signal.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use codelet_sessions::SessionManager;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Trimmed offline models.dev catalog (anthropic/openai/google). Seeded into
/// the temp cache so registry validation is fully offline — same fixture as
/// RPC-433 / RPC-385 / PROV-101.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// Serialises tests that swap the process-global data directory so a
/// parallel test cannot observe a `SessionManager::new()` loading another
/// test's persisted state (PROV-118/119 / RPC-433 precedent).
static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

/// Set dummy creds so `ProviderCredentials::detect()` passes offline.
fn set_dummy_credentials() {
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");
}

/// Build a manager rooted in a fresh data dir whose model cache is
/// pre-seeded from the offline fixture, with the default model set so
/// `create_session*` succeeds (PROV-101 decline does not fire).
fn manager_with_seeded_cache() -> Result<(tempfile::TempDir, Arc<SessionManager>), String> {
    set_dummy_credentials();
    let data_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).map_err(|e| e.to_string())?;
    // RPC-423 precedent: reset the persistence stores BEFORE setting the
    // data directory so `init_session_store()` creates a fresh
    // SessionStore pointing at the new temp dir (the store caches its
    // sessions_dir on first touch).
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let manager = Arc::new(SessionManager::new());
    Ok((data_dir, manager))
}

// =============================================================================
// Scenario: Clearing an unknown session id pushes no cleared event
// =============================================================================

/// Feature: spec/features/session-events-daemon-cleared-broadcast.feature
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clearing_an_unknown_session_id_pushes_no_cleared_event() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a session manager with a subscriber on the cleared session broadcast
    // (manager-level equivalent of the wire stream: a subscriber on the
    // cleared broadcast, created before the live session exists so only
    // clears can arrive on it)
    let (_data_dir, manager) = manager_with_seeded_cache()?;
    let mut cleared_rx = manager.session_cleared_tx().subscribe();

    // @step When clear_session_history is called for a session id that is not in the manager's live map
    // (The pre-existing contract is that an unknown id surfaces
    // `Err("Session not found: …")` — pinned by
    // rust/sessions/tests/handle_impl.rs; the scenario's assertion is
    // about the BROADCAST, not the return value.)
    let unknown = Uuid::new_v4().to_string();
    let clear_result = manager.clear_session_history(&unknown);

    // @step Then the clear returns the existing not-found error
    let err = clear_result.expect_err(
        "clear_session_history on an unknown id must return the existing not-found error",
    );
    assert!(
        err.contains("Session not found"),
        "the clear error must preserve the existing 'Session not found' contract; got {err:?}"
    );

    // @step And no cleared event is observed on the cleared broadcast
    assert!(
        cleared_rx.try_recv().is_err(),
        "clearing an unknown session id must NOT fire the cleared broadcast"
    );
    Ok(())
}

// =============================================================================
// Scenario: Clearing a live session pushes exactly one cleared event
// carrying its id
// =============================================================================

/// Feature: spec/features/session-events-daemon-cleared-broadcast.feature
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clearing_a_live_session_pushes_exactly_one_cleared_event() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given the session manager holds one live session
    let (_data_dir, manager) = manager_with_seeded_cache()?;
    let live = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(&live, "anthropic/claude-opus-4-5", ".", "RPC-436 live")
        .await
        .map_err(|e| format!("create_session_with_id: {e}"))?;
    let mut cleared_rx = manager.session_cleared_tx().subscribe();

    // @step When clear_session_history is called for that session
    manager
        .clear_session_history(&live)
        .map_err(|e| format!("clear_session_history(live): {e}"))?;

    // @step Then exactly one cleared event is observed on the cleared broadcast, carrying the cleared session's id
    let got = tokio::time::timeout(Duration::from_millis(500), cleared_rx.recv())
        .await
        .map_err(|_| "timed out waiting for the session-cleared broadcast".to_string())?
        .map_err(|e| format!("cleared broadcast recv error: {e}"))?;
    assert_eq!(
        got.value, live,
        "the cleared broadcast must carry the cleared id"
    );
    assert!(
        cleared_rx.try_recv().is_err(),
        "clear_session_history must fire the cleared broadcast exactly once per clear"
    );

    // @step And a second clear of the same id pushes exactly one more cleared event (clears are repeatable — unlike destroy, a cleared session stays live)
    manager
        .clear_session_history(&live)
        .map_err(|e| format!("second clear_session_history: {e}"))?;
    let got2 = tokio::time::timeout(Duration::from_millis(500), cleared_rx.recv())
        .await
        .map_err(|_| "timed out waiting for the second session-cleared broadcast".to_string())?
        .map_err(|e| format!("cleared broadcast recv error: {e}"))?;
    assert_eq!(
        got2.value, live,
        "the second cleared broadcast must carry the cleared id"
    );
    assert!(
        cleared_rx.try_recv().is_err(),
        "the second clear must fire the cleared broadcast exactly once"
    );
    Ok(())
}
