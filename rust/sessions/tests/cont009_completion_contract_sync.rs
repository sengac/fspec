//! Feature: spec/features/completion-contract-dispatch-arming.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_core::persistence::reset_stores_for_tests;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::background_session::BackgroundSession;
use codelet_sessions::SessionManager;
use tokio::sync::Mutex;

const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

async fn fresh_session() -> (
    tempfile::TempDir,
    Arc<SessionManager>,
    Arc<BackgroundSession>,
) {
    // @step Given the fresh_session() helper in cont009_completion_contract_sync.rs
    // @step When the helper creates a temp data directory
    let data_dir = tempfile::tempdir().expect("tempdir");
    // @step Then it must create a cache/ subdirectory inside the temp data dir
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    // @step And it must write the prov101_models.json fixture content to cache/models.json
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("write models.json");
    // @step And it must call reset_stores_for_tests() before setting the data directory
    // RPC-423: reset stores BEFORE setting data directory so init_session_store()
    // creates a fresh SessionStore pointing to the new temp dir.
    reset_stores_for_tests();
    codelet_common::set_data_directory(data_dir.path().to_path_buf()).expect("set data dir");
    // Hermetic: seed a fake key so anthropic credential validation passes
    // regardless of the developer's shell (no ambient ANTHROPIC_API_KEY).
    std::env::set_var("ANTHROPIC_API_KEY", "cont009-fake-key");
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-opus-4-5");
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let sid = handle.create_session(None);
    // @step And the subsequent create_session() call must return a valid non-empty session ID
    assert!(
        !sid.value.is_empty(),
        "create_session must return a non-empty session id once the default model is set and the cache is seeded"
    );
    let session = manager
        .get_session(&sid.value)
        .expect("session must exist after create_session");
    (data_dir, manager, session)
}

// ============================================================================
// Scenario: Chrome continue state syncs into the inner session and arms the
// registry
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn continue_state_syncs_inner_and_arms_registry() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a BackgroundSession with auto-continue enabled and budget 5 via its chrome state
    let (_dir, _manager, session) = fresh_session().await;
    session.set_continue_state(true, 5);

    // @step When the dispatch-site sync helper runs for a real user message
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
    }

    // @step Then the inner session has continue_enabled true and continue_budget 5
    {
        let inner = session.inner.lock().await;
        assert!(
            inner.continue_enabled,
            "inner continue_enabled must be synced from chrome state"
        );
        assert_eq!(
            inner.continue_budget, 5,
            "inner continue_budget must be synced from chrome state"
        );
    }

    // @step And the done() registry reports the session as armed
    assert!(
        codelet_tools::is_continue_armed(session.id),
        "registry must be armed when chrome continue toggle is on"
    );
}

// ============================================================================
// Scenario: A goal alone arms the registry and registers the goal spec
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn goal_alone_arms_registry_and_registers_goal_spec() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a BackgroundSession with the continue toggle off and a chrome goal with text and verify command
    let (_dir, _manager, session) = fresh_session().await;
    session.set_continue_state(false, 10);
    session.set_goal_state(Some((
        "ship the arming fix".to_string(),
        Some("cargo check -p codelet-sessions".to_string()),
    )));

    // @step When the dispatch-site sync helper runs for a real user message
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
    }

    // @step Then the inner session goal matches the chrome goal text and verify command
    {
        let inner = session.inner.lock().await;
        let goal = inner.goal.as_ref().expect("inner goal must be set");
        assert_eq!(goal.text, "ship the arming fix");
        assert_eq!(
            goal.verify.as_deref(),
            Some("cargo check -p codelet-sessions")
        );
    }

    // @step And the done() registry reports the session as armed
    assert!(
        codelet_tools::is_continue_armed(session.id),
        "a goal alone must arm the registry (derived mode Goal)"
    );

    // @step And the done() registry returns the goal spec with the same text and verify command
    let spec = codelet_tools::get_session_goal(session.id).expect("registry goal spec must be set");
    assert_eq!(spec.text, "ship the arming fix");
    assert_eq!(
        spec.verify.as_deref(),
        Some("cargo check -p codelet-sessions")
    );
}

// ============================================================================
// Scenario: Neither continue nor goal leaves the registry disarmed
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn neither_continue_nor_goal_disarms_registry() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a BackgroundSession with the continue toggle off and no chrome goal
    let (_dir, _manager, session) = fresh_session().await;
    session.set_continue_state(false, 10);
    session.set_goal_state(None);
    // Stale armed state from a hypothetical previous turn — the helper must
    // actively disarm, not merely leave the default in place.
    codelet_tools::set_continue_armed(session.id, true);

    // @step When the dispatch-site sync helper runs for a real user message
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
    }

    // @step Then the done() registry reports the session as disarmed
    assert!(
        !codelet_tools::is_continue_armed(session.id),
        "registry must be disarmed when neither continue nor goal is set"
    );

    // @step And the done() registry returns no goal spec
    assert!(
        codelet_tools::get_session_goal(session.id).is_none(),
        "registry must hold no goal spec"
    );
}

// ============================================================================
// Scenario: A new real user turn resets the zero-progress nudge counter
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn new_user_turn_resets_nudge_counter() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a BackgroundSession whose inner session consumed 3 zero-progress nudges in the previous turn
    let (_dir, _manager, session) = fresh_session().await;
    {
        let mut inner = session.inner.lock().await;
        inner.continue_nudges_used = 3;
    }

    // @step When the dispatch-site sync helper runs for a real user message
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
    }

    // @step Then the inner session has continue_nudges_used 0
    let inner = session.inner.lock().await;
    assert_eq!(
        inner.continue_nudges_used, 0,
        "per-turn nudge counter must be reset for a real user message"
    );
}

// ============================================================================
// Scenario: Clearing the chrome goal clears the inner goal and disarms the
// registry
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clearing_chrome_goal_clears_inner_and_registry() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a BackgroundSession whose inner session and registry carry a previously synced goal
    let (_dir, _manager, session) = fresh_session().await;
    session.set_continue_state(false, 10);
    session.set_goal_state(Some(("finish the port".to_string(), None)));
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
        assert!(inner.goal.is_some(), "precondition: inner goal synced");
    }
    assert!(
        codelet_tools::is_continue_armed(session.id),
        "precondition: registry armed by the goal"
    );

    // @step And the chrome goal has since been cleared with the continue toggle off
    session.set_goal_state(None);

    // @step When the dispatch-site sync helper runs for a real user message
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
    }

    // @step Then the inner session has no goal
    {
        let inner = session.inner.lock().await;
        assert!(
            inner.goal.is_none(),
            "inner goal must be cleared when chrome goal is cleared"
        );
    }

    // @step And the done() registry reports the session as disarmed
    assert!(
        !codelet_tools::is_continue_armed(session.id),
        "registry must disarm once the goal is cleared and continue is off"
    );

    // @step And the done() registry returns no goal spec
    assert!(
        codelet_tools::get_session_goal(session.id).is_none(),
        "registry goal spec must be removed"
    );
}

// ============================================================================
// Scenario: An unchanged chrome goal is not re-applied
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unchanged_chrome_goal_is_not_reapplied() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a BackgroundSession whose chrome goal was already synced into the inner session
    let (_dir, _manager, session) = fresh_session().await;
    session.set_goal_state(Some((
        "keep rejections intact".to_string(),
        Some("true".to_string()),
    )));
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
        assert!(inner.goal.is_some(), "precondition: inner goal synced");
    }

    // @step And the inner session has recorded 2 done() rejections for that goal
    {
        let mut inner = session.inner.lock().await;
        inner.done_rejections = 2;
    }

    // @step When the dispatch-site sync helper runs again with the same chrome goal
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
    }

    // @step Then the inner session still has 2 done() rejections
    let inner = session.inner.lock().await;
    assert_eq!(
        inner.done_rejections, 2,
        "an unchanged chrome goal must not re-call set_goal (which resets done_rejections)"
    );

    // @step And the inner session goal is unchanged
    let goal = inner.goal.as_ref().expect("inner goal must still be set");
    assert_eq!(goal.text, "keep rejections intact");
    assert_eq!(goal.verify.as_deref(), Some("true"));
}
