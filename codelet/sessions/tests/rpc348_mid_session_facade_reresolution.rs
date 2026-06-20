//! RPC-348: mid-session facade re-resolution for custom models.
//!
//! Feature: spec/features/mid-session-facade-reresolution.feature
//!
//! FOLLOW-UP to RPC-343. These tests drive the real
//! `codelet_sessions::SessionManager` through `SessionManagerHandle::set_model`
//! (and `create_session`) and assert that selecting a custom (registered
//! custom-provider) model re-resolves the per-selection facade override on the
//! inner request-issuing provider manager — closing the shared port gap where
//! both the creation path and the mid-session path passed `facade=None`.
//!
//! Offline-testable. A custom provider config is written under a redirected
//! `FSPEC_HOME` (`.fspec/providers/*.json`) so
//! `codelet_providers::custom::derive_facade_for_custom` and
//! `custom_provider_registered` discover it without network access. Dummy
//! credential env vars satisfy credential detection for the anthropic/google
//! registry switches. Each test mutates the process-global `HOME`/`FSPEC_HOME`
//! env vars, so they run `#[serial]` (mirrors the RPC-347 surface tests).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use serial_test::serial;
use tempfile::TempDir;

const MY_LLM_JSON: &str = r#"{
  "name": "my-llm",
  "display_name": "Test my-llm",
  "base_url": "https://api.example.com/v1",
  "facade": "openai",
  "api_key_env_var": "MY_LLM_API_KEY",
  "tool_style": "openai",
  "api_style": "openai_chat",
  "models": { "llama-3.1-70b": { "id": "llama-3.1-70b" } }
}"#;

const CLAUDE_COMPAT_JSON: &str = r#"{
  "name": "claude-compat",
  "display_name": "Test claude-compat",
  "base_url": "https://api.anthropic.com",
  "api_key_env_var": "ANTHROPIC_API_KEY",
  "tool_style": "claude",
  "api_style": "anthropic_messages",
  "models": { "opus-compat": { "id": "opus-compat" } }
}"#;

/// Redirect `HOME`/`FSPEC_HOME` at a throwaway dir and install the two custom
/// provider configs under `<home>/.fspec/providers/`. The returned `TempDir`
/// must be held for the duration of the test. Also sets dummy credentials so
/// anthropic/google credential detection passes offline.
fn install_custom_providers() -> TempDir {
    let home = tempfile::tempdir().expect("home tempdir");
    let fspec = home.path().join(".fspec");
    let credentials = fspec.join("credentials");
    let providers = fspec.join("providers");
    std::fs::create_dir_all(&credentials).unwrap();
    std::fs::create_dir_all(&providers).unwrap();
    std::fs::write(providers.join("my-llm.json"), MY_LLM_JSON).unwrap();
    std::fs::write(providers.join("claude-compat.json"), CLAUDE_COMPAT_JSON).unwrap();

    std::env::set_var("HOME", home.path());
    std::env::set_var("FSPEC_HOME", &credentials);
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");
    std::env::set_var("MY_LLM_API_KEY", "my-llm-test-dummy-key");
    home
}

/// Build a manager rooted in a throwaway data dir, set its default model, and
/// (optionally) create one real session. Returns the manager and the new
/// session id (empty id when `create` is false).
async fn manager_with_default(
    default_model: &str,
    create: bool,
) -> (Arc<SessionManager>, SessionId) {
    let data_dir = tempfile::tempdir().expect("data tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model(default_model);
    let sid = if create {
        let handle: &dyn SessionManagerHandle = manager.as_ref();
        let sid = handle.create_session(None);
        assert!(
            !sid.value.is_empty(),
            "session creation must succeed offline (empty id means create_session errored)"
        );
        sid
    } else {
        SessionId::new("")
    };
    (manager, sid)
}

/// Read the inner provider manager's facade override for a session.
async fn facade_for(manager: &Arc<SessionManager>, sid: &SessionId) -> Option<String> {
    let session = manager.get_session(&sid.value).expect("session must exist");
    let inner = session.inner.lock().await;
    inner
        .provider_manager()
        .facade_override()
        .map(str::to_string)
}

// =============================================================================
// Scenario: Creating a session on a custom model resolves the facade at creation time
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn creating_session_on_custom_model_resolves_facade() {
    // @step Given a registered custom provider "my-llm" with explicit facade "openai" and model "llama-3.1-70b"
    let _home = install_custom_providers();

    // @step And the session manager default model is "my-llm/llama-3.1-70b"
    // @step When I create a new session
    let (manager, sid) = manager_with_default("my-llm/llama-3.1-70b", true).await;

    // @step Then the inner session provider manager reports facade override "openai"
    let facade = facade_for(&manager, &sid).await;
    assert_eq!(
        facade.as_deref(),
        Some("openai"),
        "creation path should resolve the custom facade via the shared helper, got {facade:?}"
    );
}

// =============================================================================
// Scenario: Switching to a custom model with an explicit facade stores that facade
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn switching_to_custom_model_with_explicit_facade_stores_it() {
    // @step Given a session created on "anthropic/claude-opus-4-5"
    let _home = install_custom_providers();
    let (manager, sid) = manager_with_default("anthropic/claude-opus-4-5", true).await;
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step And a registered custom provider "my-llm" with explicit facade "openai" and model "llama-3.1-70b"
    // (installed by install_custom_providers)

    // @step When I switch the session model to provider "my-llm" model "llama-3.1-70b" via set_model
    let result = handle.set_model(&sid, "my-llm", "llama-3.1-70b");

    // @step Then set_model returns Ok
    assert!(result.is_ok(), "set_model should return Ok, got {result:?}");

    // @step And the inner session provider manager reports facade override "openai"
    let facade = facade_for(&manager, &sid).await;
    assert_eq!(
        facade.as_deref(),
        Some("openai"),
        "explicit facade should be stored, got {facade:?}"
    );
}

// =============================================================================
// Scenario: Switching to a custom model with no explicit facade derives the facade from api_style
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn switching_to_custom_model_derives_facade_from_api_style() {
    // @step Given a session created on "anthropic/claude-opus-4-5"
    let _home = install_custom_providers();
    let (manager, sid) = manager_with_default("anthropic/claude-opus-4-5", true).await;
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step And a registered custom provider "claude-compat" with no explicit facade and api_style "anthropic_messages" and model "opus-compat"
    // (installed by install_custom_providers)

    // @step When I switch the session model to provider "claude-compat" model "opus-compat" via set_model
    let result = handle.set_model(&sid, "claude-compat", "opus-compat");

    // @step Then set_model returns Ok
    assert!(result.is_ok(), "set_model should return Ok, got {result:?}");

    // @step And the inner session provider manager reports facade override "claude"
    let facade = facade_for(&manager, &sid).await;
    assert_eq!(
        facade.as_deref(),
        Some("claude"),
        "facade should be derived from anthropic_messages api_style, got {facade:?}"
    );
}

// =============================================================================
// Scenario: Switching from a custom model to a registry model clears the stale facade
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn switching_from_custom_to_registry_clears_facade() {
    // @step Given a session that has been switched to a custom provider "my-llm" model "llama-3.1-70b" whose inner facade override is "openai"
    let _home = install_custom_providers();
    let (manager, sid) = manager_with_default("anthropic/claude-opus-4-5", true).await;
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    handle
        .set_model(&sid, "my-llm", "llama-3.1-70b")
        .expect("switch to custom model should succeed");
    assert_eq!(
        facade_for(&manager, &sid).await.as_deref(),
        Some("openai"),
        "precondition: custom facade should be set before the registry switch"
    );

    // @step When I switch the session model to provider "google" model "gemini-2.5-pro" via set_model
    let result = handle.set_model(&sid, "google", "gemini-2.5-pro");

    // @step Then set_model returns Ok
    assert!(result.is_ok(), "set_model should return Ok, got {result:?}");

    // @step And the inner session provider manager reports no facade override
    let facade = facade_for(&manager, &sid).await;
    assert_eq!(
        facade, None,
        "stale custom facade must be cleared on a registry switch, got {facade:?}"
    );
}

// =============================================================================
// Scenario: Switching between two registry models leaves no facade override
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn switching_between_registry_models_leaves_no_facade() {
    // @step Given a session created on "anthropic/claude-opus-4-5" whose inner provider manager reports no facade override
    let _home = install_custom_providers();
    let (manager, sid) = manager_with_default("anthropic/claude-opus-4-5", true).await;
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    assert_eq!(
        facade_for(&manager, &sid).await,
        None,
        "precondition: a plain registry session has no facade override"
    );

    // @step When I switch the session model to provider "google" model "gemini-2.5-pro" via set_model
    let result = handle.set_model(&sid, "google", "gemini-2.5-pro");

    // @step Then set_model returns Ok
    assert!(result.is_ok(), "set_model should return Ok, got {result:?}");

    // @step And the inner session provider manager reports no facade override
    let facade = facade_for(&manager, &sid).await;
    assert_eq!(
        facade, None,
        "registry-to-registry switch must not introduce a facade, got {facade:?}"
    );
}
