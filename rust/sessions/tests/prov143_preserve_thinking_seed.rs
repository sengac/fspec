//! PROV-143: session creation seeds the preserve-thinking flag from the profile.
//!
//! Feature: spec/features/profile-preserve-thinking-session-seeding.feature
//!
//! When a session is created against a profile model
//! (`openai:<profile>/<model>`), the session's `preserve_thinking_enabled`
//! flag is seeded from the profile's stored `preserveThinking` value BEFORE
//! the first user message is dispatched:
//! - `preserveThinking: true`  ⇒ the flag is seeded `true`
//! - `preserveThinking: false` ⇒ the flag is seeded `false`
//! - key absent                ⇒ the flag is seeded `false` (the default —
//!   thinking blocks are stripped from the outgoing chat history)
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
fn manager_with_seeded_cache(
    user_dir_config: &str,
) -> Result<(Vec<tempfile::TempDir>, Arc<SessionManager>), String> {
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

/// Create a session against `openai:spark/o3` and read back the seeded
/// `preserve_thinking_enabled` flag from the inner session.
fn seeded_flag(manager: &Arc<SessionManager>) -> bool {
    manager.set_default_model("openai:spark/o3");
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let sid = handle.create_session(None);
    assert!(
        !sid.value.is_empty(),
        "PROV-143: profile-model session creation must succeed"
    );
    let session = manager.get_session(&sid.value).expect("session must exist");
    let flag = {
        let guard = session
            .inner
            .try_lock()
            .expect("idle session lock");
        guard.preserve_thinking_enabled
    };
    flag
}

// =============================================================================
// Scenario: Sessions from profiles seed the preserve-thinking flag
//   (preserveThinking: true ⇒ flag true)
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_against_profile_with_preserve_thinking_true_seeds_on() -> Result<(), String> {
    let _guard = ENV_AND_DATA_DIR_GUARD.lock().await;

    // @step Given a local-server profile with preserveThinking = true
    let (_dirs, manager) = manager_with_seeded_cache(
        r#"{
          "providers": {
            "openai": {
              "profiles": {
                "spark": {
                  "baseUrl": "http://spark:8001",
                  "apiKey": "test",
                  "contextWindow": 262144,
                  "preserveThinking": true
                }
              }
            }
          }
        }"#,
    )?;

    // @step When a session is created against a model of that profile
    // @step Then the session seeds preserve_thinking_enabled = true
    assert!(
        seeded_flag(&manager),
        "PROV-143: preserveThinking=true must seed the flag ON"
    );
    Ok(())
}

// =============================================================================
// Scenario: Sessions from profiles seed the preserve-thinking flag
//   (preserveThinking: false ⇒ flag false)
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_against_profile_with_preserve_thinking_false_seeds_off() -> Result<(), String> {
    let _guard = ENV_AND_DATA_DIR_GUARD.lock().await;

    // @step Given a local-server profile with preserveThinking = false
    let (_dirs, manager) = manager_with_seeded_cache(
        r#"{
          "providers": {
            "openai": {
              "profiles": {
                "spark": {
                  "baseUrl": "http://spark:8001",
                  "apiKey": "test",
                  "contextWindow": 262144,
                  "preserveThinking": false
                }
              }
            }
          }
        }"#,
    )?;

    // @step When a session is created against a model of that profile
    // @step Then the session seeds preserve_thinking_enabled = false
    assert!(
        !seeded_flag(&manager),
        "PROV-143: preserveThinking=false must seed the flag OFF"
    );
    Ok(())
}

// =============================================================================
// Scenario: Sessions from profiles seed the preserve-thinking flag
//   (key absent ⇒ flag false — the default is stripped)
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_against_profile_without_preserve_thinking_key_seeds_off() -> Result<(), String> {
    let _guard = ENV_AND_DATA_DIR_GUARD.lock().await;

    // @step Given a local-server profile with no preserveThinking key
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
    // @step Then the session seeds preserve_thinking_enabled = false
    // @step And a profile without the key seeds the flag false
    assert!(
        !seeded_flag(&manager),
        "PROV-143: an absent preserveThinking key must seed the flag OFF (default stripped)"
    );
    Ok(())
}
