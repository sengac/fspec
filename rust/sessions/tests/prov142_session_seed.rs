//! PROV-142: session creation seeds auto-continue state from the profile.
//!
//! Feature: spec/features/profile-auto-continue-session-seeding.feature
//!
//! When a session is created against a profile model
//! (`openai:<profile>/<model>`), the session's auto-continue state is seeded
//! from the profile's stored `autoContinue` value BEFORE the first user
//! message is dispatched. The seed happens in the shared session creation
//! helper (`create_background_session_inner`) so both `create_session_with_id`
//! and `create_session_from_manifest` paths get it.
//!
//! Fully offline: a throwaway data dir is seeded with a trimmed models.json
//! fixture so registry validation needs no network. All credential sources
//! are isolated to empty temp dirs (PROV-141 harness pattern); the profile
//! config is isolated via `FSPEC_USER_DIR`.

#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::SessionManager;
use tokio::sync::Mutex as AsyncMutex;

/// Trimmed offline models.dev catalog (openai only, one tool-call model).
/// Seeded into the temp cache so registry validation is offline.
const MODELS_FIXTURE: &str = r#"{
  "openai": {
    "id": "openai",
    "name": "OpenAI",
    "env": ["OPENAI_API_KEY"],
    "models": {
      "o3": {
        "id": "o3",
        "name": "o3",
        "reasoning": true,
        "tool_call": true,
        "attachment": true,
        "temperature": false,
        "limit": { "context": 200000, "output": 100000 }
      }
    }
  }
}
"#;

/// Serializes the tests that swap process-global state: the data directory
/// (`codelet_common::set_data_directory`), the credential env vars, and the
/// credential-file locations (`CODEX_HOME`, `FSPEC_HOME`, `FSPEC_USER_DIR`).
/// Mirrors PROV-141's ENV_AND_DATA_DIR_GUARD. An async mutex (not a std one)
/// so the guard is never held across an await point.
static ENV_AND_DATA_DIR_GUARD: AsyncMutex<()> = AsyncMutex::const_new(());

/// Isolate EVERY credential source and point `FSPEC_USER_DIR` at a temp dir
/// containing ONLY the given config. Returns the temp dirs (kept alive for
/// the test lifetime).
fn isolate_credentials(user_dir_config: &str) -> (
    tempfile::TempDir,
    tempfile::TempDir,
    tempfile::TempDir,
) {
    for var in [
        "ANTHROPIC_API_KEY",
        "CLAUDE_CODE_OAUTH_TOKEN",
        "OPENAI_API_KEY",
        "GOOGLE_GENERATIVE_AI_API_KEY",
        "ZAI_API_KEY",
        "ZAI_PLAN_API_KEY",
    ] {
        std::env::remove_var(var);
    }

    let codex_home = tempfile::tempdir().expect("create empty CODEX_HOME");
    std::env::set_var("CODEX_HOME", codex_home.path());

    let fspec_home = tempfile::tempdir().expect("create empty FSPEC_HOME");
    std::env::set_var("FSPEC_HOME", fspec_home.path());

    let user_dir = tempfile::tempdir().expect("create temp FSPEC_USER_DIR");
    std::fs::write(user_dir.path().join("fspec-config.json"), user_dir_config)
        .expect("write fspec-config.json");
    std::env::set_var("FSPEC_USER_DIR", user_dir.path());

    (codex_home, fspec_home, user_dir)
}

/// Build a manager rooted in a fresh data dir whose model cache is pre-seeded
/// from the offline fixture. Returns the temp dirs (kept alive for the test)
/// and the manager.
fn manager_with_seeded_cache(user_dir_config: &str) -> Result<(Vec<tempfile::TempDir>, Arc<SessionManager>), String> {
    let (codex_home, fspec_home, user_dir) = isolate_credentials(user_dir_config);
    let data_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).map_err(|e| e.to_string())?;
    // RPC-423 precedent: reset the process-global persistence singletons
    // BEFORE swapping the data directory so the next persistence operation
    // re-initialises against the new dir.
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let manager = Arc::new(SessionManager::new());
    Ok((
        vec![codex_home, fspec_home, user_dir, data_dir],
        manager,
    ))
}

// =============================================================================
// Scenario: A session against a profile with autoContinue 300 starts with
//            auto-continue on and budget 300
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_against_profile_with_auto_continue_300_starts_on_with_budget_300()
-> Result<(), String> {
    let _guard = ENV_AND_DATA_DIR_GUARD.lock().await;

    // @step Given a stored profile whose autoContinue value is 300
    let (_dirs, manager) = manager_with_seeded_cache(
        r#"{
          "providers": {
            "openai": {
              "profiles": {
                "spark": {
                  "baseUrl": "http://spark:8001",
                  "apiKey": "test",
                  "contextWindow": 262144,
                  "autoContinue": 300
                }
              }
            }
          }
        }"#,
    )?;

    // @step When a session is created against a model of that profile
    manager.set_default_model("openai:spark/o3");
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let sid = handle.create_session(None);
    assert!(
        !sid.value.is_empty(),
        "PROV-142: profile-model session creation must succeed"
    );

    // @step Then the session's auto-continue is enabled
    // @step And the session's continue budget is 300
    let (enabled, budget) = handle.get_continue_state(&sid);
    assert!(
        enabled,
        "PROV-142: a profile with autoContinue=300 must seed auto-continue ON"
    );
    assert_eq!(
        budget, 300,
        "PROV-142: the seeded budget must be the profile's autoContinue value (300)"
    );
    Ok(())
}

// =============================================================================
// Scenario: A session against a profile with autoContinue 0 starts with
//            auto-continue off
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_against_profile_with_auto_continue_0_starts_off() -> Result<(), String> {
    let _guard = ENV_AND_DATA_DIR_GUARD.lock().await;

    // @step Given a stored profile whose autoContinue value is 0
    let (_dirs, manager) = manager_with_seeded_cache(
        r#"{
          "providers": {
            "openai": {
              "profiles": {
                "spark": {
                  "baseUrl": "http://spark:8001",
                  "apiKey": "test",
                  "contextWindow": 262144,
                  "autoContinue": 0
                }
              }
            }
          }
        }"#,
    )?;

    // @step When a session is created against a model of that profile
    manager.set_default_model("openai:spark/o3");
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let sid = handle.create_session(None);
    assert!(
        !sid.value.is_empty(),
        "PROV-142: profile-model session creation must succeed"
    );

    // @step Then the session's auto-continue is disabled
    let (enabled, _budget) = handle.get_continue_state(&sid);
    assert!(
        !enabled,
        "PROV-142: the explicit-off sentinel (autoContinue=0) must seed auto-continue OFF"
    );
    Ok(())
}

// =============================================================================
// Scenario: A session against a profile without an autoContinue key starts
//            with auto-continue off
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_against_profile_without_auto_continue_key_starts_off() -> Result<(), String> {
    let _guard = ENV_AND_DATA_DIR_GUARD.lock().await;

    // @step Given a stored profile with no autoContinue key
    let (_dirs, manager) = manager_with_seeded_cache(
        r#"{
          "providers": {
            "openai": {
              "profiles": {
                "spark": {
                  "baseUrl": "http://spark:8001",
                  "apiKey": "test",
                  "contextWindow": 262144
                }
              }
            }
          }
        }"#,
    )?;

    // @step When a session is created against a model of that profile
    manager.set_default_model("openai:spark/o3");
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let sid = handle.create_session(None);
    assert!(
        !sid.value.is_empty(),
        "PROV-142: profile-model session creation must succeed"
    );

    // @step Then the session's auto-continue is disabled
    let (enabled, _budget) = handle.get_continue_state(&sid);
    assert!(
        !enabled,
        "PROV-142: an absent autoContinue key must seed auto-continue OFF (today's behavior)"
    );
    Ok(())
}
