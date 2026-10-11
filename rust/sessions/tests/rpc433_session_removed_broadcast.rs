//! RPC-433 — Daemon-side session removal broadcast + in-memory
//! live-session snapshot (manager-level half).
//!
//! Feature: spec/features/session-events-daemon-removed-broadcast.feature
//!
//!   Scenario: Destroying an unknown session id pushes no removal event
//!   Scenario: The in-memory live-session snapshot excludes destroyed
//!     sessions
//!
//! This file owns the SessionManager-level scenarios of RPC-433:
//! the `session_removed_tx` broadcast (fired by `destroy_session` only
//! when the in-memory map actually lost a session — rule R2) and
//! `SessionManager::live_session_infos()` (in-memory-only snapshot,
//! parity with the BUG-206 liveness gate — rule R3). The wire-side
//! fan-out scenarios live in the sibling feature
//! session-events-daemon-fanout (one test file per feature, ACDD 1:1).
//!
//! RED PHASE NOTE (history): `SessionManager::session_removed_tx()`
//! and `SessionManager::live_session_infos()` did not exist when this
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
/// RPC-385 / PROV-101.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// Serialises tests that swap the process-global data directory so a
/// parallel test cannot observe a `SessionManager::new()` loading another
/// test's persisted state (PROV-118/119 precedent).
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
// Scenario: Destroying an unknown session id pushes no removal event
// =============================================================================

/// Feature: spec/features/session-events-daemon-removed-broadcast.feature
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn destroying_an_unknown_session_id_pushes_no_removal_event() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a session manager with a subscriber on the removed session broadcast
    // (manager-level equivalent of the wire stream: a subscriber on the
    // removed broadcast, created before the live session exists so only
    // removals can arrive on it)
    let (_data_dir, manager) = manager_with_seeded_cache()?;
    let mut removed_rx = manager.session_removed_tx().subscribe();

    // @step When destroy_session is called for a session id that is not in the manager's live map
    // (The pre-existing contract is that an unknown id surfaces
    // `Err("Session not found: …")` — pinned by
    // rust/sessions/tests/handle_impl.rs; the scenario's assertion is
    // about the BROADCAST, not the return value.)
    let unknown = Uuid::new_v4().to_string();
    let _destroy_result = manager.destroy_session(&unknown);

    // @step Then no removal event is observed on the removed broadcast
    assert!(
        removed_rx.try_recv().is_err(),
        "destroying an unknown session id must NOT fire the removed broadcast"
    );

    // R2 (supporting invariant): a LIVE destroy fires exactly one event
    // carrying the destroyed id — no more, no less.
    let live = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(&live, "anthropic/claude-opus-4-5", ".", "RPC-433 live")
        .await
        .map_err(|e| format!("create_session_with_id: {e}"))?;
    manager
        .destroy_session(&live)
        .map_err(|e| format!("destroy_session(live): {e}"))?;
    let got = tokio::time::timeout(Duration::from_millis(500), removed_rx.recv())
        .await
        .map_err(|_| "timed out waiting for the session-removed broadcast".to_string())?
        .map_err(|e| format!("removed broadcast recv error: {e}"))?;
    assert_eq!(
        got.value, live,
        "the removed broadcast must carry the destroyed id"
    );
    assert!(
        removed_rx.try_recv().is_err(),
        "destroy_session must fire the removed broadcast exactly once per live session"
    );
    Ok(())
}

// =============================================================================
// Scenario: The in-memory live-session snapshot excludes destroyed sessions
// =============================================================================

/// Feature: spec/features/session-events-daemon-removed-broadcast.feature
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_session_infos_excludes_destroyed_sessions() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given the daemon's session manager holds two live sessions
    let (_data_dir, manager) = manager_with_seeded_cache()?;
    let id_a = Uuid::new_v4().to_string();
    let id_b = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(&id_a, "anthropic/claude-opus-4-5", ".", "RPC-433 live A")
        .await
        .map_err(|e| format!("create A: {e}"))?;
    manager
        .create_session_with_id(&id_b, "anthropic/claude-opus-4-5", ".", "RPC-433 live B")
        .await
        .map_err(|e| format!("create B: {e}"))?;
    let infos = manager.live_session_infos();
    let ids: Vec<String> = infos.iter().map(|i| i.id.clone()).collect();
    assert!(
        ids.iter().any(|s| s == &id_a) && ids.iter().any(|s| s == &id_b),
        "precondition: live_session_infos() must list both live sessions; got {ids:?}"
    );

    // @step When one of those sessions is destroyed
    // (destroy_session's manifest-on-disk persistence is exactly WHY the
    // snapshot must be in-memory-only: a disk-merged snapshot would
    // still list session A after its close)
    manager
        .destroy_session(&id_a)
        .map_err(|e| format!("destroy A: {e}"))?;

    // @step Then the in-memory live-session snapshot excludes the destroyed session and retains the other
    let after: Vec<String> = manager
        .live_session_infos()
        .iter()
        .map(|i| i.id.clone())
        .collect();
    assert!(
        after.iter().all(|s| s != &id_a) && after.iter().any(|s| s == &id_b),
        "the snapshot must reflect live changes (removal of A, retention of B); got {after:?}"
    );
    Ok(())
}
