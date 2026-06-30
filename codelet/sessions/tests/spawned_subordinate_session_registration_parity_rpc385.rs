//! RPC-385 — Spawned subordinate agents are not registered/visible in the
//! Rust TUI (SessionManager broadcast half).
//!
//! Feature: spec/features/agentview-spawned-subordinate-session-registration.feature
//!
//! This file owns the BACKEND-broadcast scenario (#1) of the feature:
//!
//!   Scenario: Creating a session broadcasts a session-created event
//!
//! Placement rationale: the broadcast under test (`session_created_tx`) is a
//! `SessionManager`-owned channel that lives in `codelet-sessions`, exactly
//! like the existing `chunks_tx` / `status_changes_tx` senders that are also
//! exercised from this crate's tests (see `tui002_default_thinking_level.rs`,
//! `prov118_no_session_default_model.rs`). Driving a real `SessionManager`
//! against the shared offline models.dev fixture lets us assert the broadcast
//! fires from `create_session_with_id` without standing up the whole TUI.
//!
//! RED PHASE NOTE: `SessionManager::session_created_tx()` does not yet exist,
//! so this test will FAIL TO COMPILE until Approach A's backend broadcast is
//! added. That is the intended red signal for scenario #1 — the accessor and
//! the fire site inside `create_session_with_id` are the implementation
//! deliverables this test pins.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

use codelet_sessions::SessionManager;
use uuid::Uuid;

/// Trimmed offline models.dev catalog (anthropic/openai/google). Seeded into
/// the temp cache so registry validation is fully offline — shared with
/// PROV-101 / TUI-002.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// Serialises tests that swap the process-global data directory so a parallel
/// test cannot observe a `SessionManager::new()` loading another test's
/// persisted `default-model.json`. Mirrors the PROV-118/119 precedent.
static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

/// Set dummy creds so `ProviderCredentials::detect()` passes offline.
fn set_dummy_credentials() {
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");
}

/// Build a manager rooted in a fresh data dir whose model cache is pre-seeded
/// from the offline fixture, with the default model set so `create_session*`
/// succeeds (PROV-101 decline does not fire).
fn manager_with_seeded_cache() -> Result<(tempfile::TempDir, Arc<SessionManager>), String> {
    set_dummy_credentials();
    let data_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).map_err(|e| e.to_string())?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let manager = Arc::new(SessionManager::new());
    Ok((data_dir, manager))
}

// =============================================================================
// Scenario: Creating a session broadcasts a session-created event
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn creating_a_session_broadcasts_a_session_created_event() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a SessionManager with a subscriber on the session-created broadcast
    let (_data_dir, manager) = manager_with_seeded_cache()?;
    let mut session_created_rx = manager.session_created_tx().subscribe();

    // @step When a session is created via create_session_with_id
    let id = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(&id, "anthropic/claude-opus-4-5", ".", "Agent test")
        .await?;

    // @step Then the subscriber receives the new session id
    let received = tokio::time::timeout(Duration::from_millis(500), session_created_rx.recv())
        .await
        .map_err(|_| "timed out waiting for session-created broadcast".to_string())?
        .map_err(|e| format!("broadcast recv error: {e}"))?;
    assert_eq!(
        received.id, id,
        "the broadcast payload must carry the SessionId of the just-created session"
    );
    Ok(())
}
