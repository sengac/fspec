//! RPC-343: mid-session model re-resolution.
//!
//! Feature: spec/features/mid-session-model-reresolution.feature
//!
//! These tests drive the real `codelet_sessions::SessionManager` through the
//! `SessionManagerHandle::set_model` path and assert that switching the active
//! model mid-session re-resolves the cached limits (context_window /
//! max_output_tokens / compaction_threshold) and updates the inner
//! request-issuing provider manager — not just the cosmetic label strings.
//!
//! Offline-testable via a CROSS-FAMILY switch. The Claude limits resolver
//! clamps every anthropic model to ctx 200000 / out 8192 with an identical base
//! compaction threshold, so an anthropic→anthropic switch is NOT observable on
//! limits. `google/gemini-2.5-pro` resolves (from `fallback_models.json`) to
//! ctx 1048576 / out 65536 with the gemini 80% compaction rule, so all three
//! cached fields differ. Dummy credential env vars are set so credential
//! detection passes; no network access occurs (select_model only validates
//! against the registry and updates state). Setup mirrors
//! `rpc081_restore_session_messages.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;

/// Ensure credential detection passes offline for both providers under test.
/// Values are dummies — `set_model` re-resolution makes no network calls.
fn set_dummy_credentials() {
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");
}

/// Build a manager rooted in a throwaway data dir, defaulted to opus, and
/// create one real session. Returns the manager plus the new session id.
async fn manager_with_opus_session() -> (Arc<SessionManager>, SessionId) {
    set_dummy_credentials();
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-opus-4-5");
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let sid = handle.create_session(None);
    assert!(
        !sid.value.is_empty(),
        "session creation must succeed offline (empty id means create_session errored)"
    );
    (manager, sid)
}

// =============================================================================
// Scenario: Switching model re-resolves the cached limits to the new model
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switching_model_reresolves_cached_limits() {
    // @step Given a session created on "anthropic/claude-opus-4-5" whose resolved context_window is 200000 and max_output_tokens is 8192
    let (manager, sid) = manager_with_opus_session().await;
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let before = handle.get_session_model(&sid);
    assert_eq!(
        before.context_window, 200000,
        "opus baseline context_window should be 200000, got {}",
        before.context_window
    );
    assert_eq!(
        before.max_output_tokens, 8192,
        "opus baseline max_output_tokens should be 8192, got {}",
        before.max_output_tokens
    );
    let stale_threshold = before.compaction_threshold;

    // @step When I switch the session model to provider "google" model "gemini-2.5-pro" via set_model
    let result = handle.set_model(&sid, "google", "gemini-2.5-pro");

    // @step Then set_model returns Ok
    assert!(result.is_ok(), "set_model should return Ok, got {result:?}");

    // @step And get_session_model reports context_window 1048576 for the session
    let after = handle.get_session_model(&sid);
    assert_eq!(
        after.context_window, 1048576,
        "gemini context_window should be 1048576, got {}",
        after.context_window
    );

    // @step And get_session_model reports max_output_tokens 65536 for the session
    assert_eq!(
        after.max_output_tokens, 65536,
        "gemini max_output_tokens should be 65536, got {}",
        after.max_output_tokens
    );

    // @step And get_session_model reports a compaction_threshold recomputed for the new model rather than the stale claude-derived value
    assert_ne!(
        after.compaction_threshold, stale_threshold,
        "compaction_threshold should be recomputed for gemini, not left at the stale claude value {stale_threshold}"
    );
}

// =============================================================================
// Scenario: Switching model updates the inner provider manager's selected model
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switching_model_updates_inner_provider_manager() {
    // @step Given a session created on "anthropic/claude-opus-4-5"
    let (manager, sid) = manager_with_opus_session().await;
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step When I switch the session model to provider "google" model "gemini-2.5-pro" via set_model
    let result = handle.set_model(&sid, "google", "gemini-2.5-pro");
    assert!(result.is_ok(), "set_model should return Ok, got {result:?}");

    // @step Then the inner session provider manager reports a gemini model id as its selected model
    let session = manager
        .get_session(&sid.value)
        .expect("session must exist");
    let inner = session.inner.lock().await;
    let selected = inner.current_model_id().unwrap_or_default();
    assert!(
        selected.contains("gemini"),
        "inner provider manager selected model should be a gemini model, got {selected:?}"
    );
}

// =============================================================================
// Scenario: Switching to an unknown model fails and leaves the prior limits intact
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switching_to_unknown_model_preserves_prior_limits() {
    // @step Given a session created on "anthropic/claude-opus-4-5" whose resolved context_window is 200000 and max_output_tokens is 8192
    let (manager, sid) = manager_with_opus_session().await;
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let before = handle.get_session_model(&sid);
    assert_eq!(
        before.context_window, 200000,
        "opus baseline context_window should be 200000, got {}",
        before.context_window
    );
    assert_eq!(
        before.max_output_tokens, 8192,
        "opus baseline max_output_tokens should be 8192, got {}",
        before.max_output_tokens
    );

    // @step When I switch the session model to provider "anthropic" model "does-not-exist-model" via set_model
    let result = handle.set_model(&sid, "anthropic", "does-not-exist-model");

    // @step Then set_model returns Err
    assert!(
        result.is_err(),
        "set_model with an unknown model should return Err, got {result:?}"
    );

    // @step And get_session_model still reports context_window 200000 and max_output_tokens 8192 for the session
    let after = handle.get_session_model(&sid);
    assert_eq!(
        after.context_window, 200000,
        "context_window must be left intact after a failed switch, got {}",
        after.context_window
    );
    assert_eq!(
        after.max_output_tokens, 8192,
        "max_output_tokens must be left intact after a failed switch, got {}",
        after.max_output_tokens
    );
}

// =============================================================================
// Scenario: Switching the model on an unknown session reports session not found
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switching_model_on_unknown_session_reports_not_found() {
    // @step Given a SessionManagerHandle with no session registered for the id "nonexistent-uuid"
    let manager = Arc::new(SessionManager::new());
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let sid = SessionId::new("nonexistent-uuid");

    // @step When I switch the model for that id to provider "google" model "gemini-2.5-pro" via set_model
    let result = handle.set_model(&sid, "google", "gemini-2.5-pro");

    // @step Then set_model returns Err containing "Session not found"
    match result {
        Ok(()) => panic!("expected Err for unknown session, got Ok"),
        Err(msg) => assert!(
            msg.contains("Session not found"),
            "expected error containing `Session not found`, got `{msg}`"
        ),
    }
}
