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

use codelet_core::session_manager_handle::SessionManagerHandle;
use fixtures::{facade_for, TestDirectory, TestResult};
use serial_test::serial;

#[path = "fixtures/rpc348_facade.rs"]
mod fixtures;

// =============================================================================
// Scenario: Creating a session on a custom model resolves the facade at creation time
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn creating_session_on_custom_model_resolves_facade() -> TestResult {
    // @step Given a registered custom provider "my-llm" with explicit facade "openai" and model "llama-3.1-70b"
    let fixture = TestDirectory::new()?;

    // @step And the session manager default model is "my-llm/llama-3.1-70b"
    // @step When I create a new session
    let (manager, sid) = fixture.manager_with_default("my-llm/llama-3.1-70b")?;

    // @step Then the inner session provider manager reports facade override "openai"
    let facade = facade_for(&manager, &sid).await?;
    assert_eq!(
        facade.as_deref(),
        Some("openai"),
        "creation path should resolve the custom facade via the shared helper, got {facade:?}"
    );
    Ok(())
}

// =============================================================================
// Scenario: Switching to a custom model with an explicit facade stores that facade
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn switching_to_custom_model_with_explicit_facade_stores_it() -> TestResult {
    // @step Given a session created on "anthropic/claude-opus-4-5"
    let fixture = TestDirectory::new()?;
    let (manager, sid) = fixture.manager_with_default("anthropic/claude-opus-4-5")?;
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step And a registered custom provider "my-llm" with explicit facade "openai" and model "llama-3.1-70b"
    // (installed by TestDirectory)

    // @step When I switch the session model to provider "my-llm" model "llama-3.1-70b" via set_model
    let result = handle.set_model(&sid, "my-llm", "llama-3.1-70b");

    // @step Then set_model returns Ok
    assert!(result.is_ok(), "set_model should return Ok, got {result:?}");

    // @step And the inner session provider manager reports facade override "openai"
    let facade = facade_for(&manager, &sid).await?;
    assert_eq!(
        facade.as_deref(),
        Some("openai"),
        "explicit facade should be stored, got {facade:?}"
    );
    Ok(())
}

// =============================================================================
// Scenario: Switching to a custom model with no explicit facade derives the facade from api_style
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn switching_to_custom_model_derives_facade_from_api_style() -> TestResult {
    // @step Given a session created on "anthropic/claude-opus-4-5"
    let fixture = TestDirectory::new()?;
    let (manager, sid) = fixture.manager_with_default("anthropic/claude-opus-4-5")?;
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step And a registered custom provider "claude-compat" with no explicit facade and api_style "anthropic_messages" and model "opus-compat"
    // (installed by install_custom_providers)

    // @step When I switch the session model to provider "claude-compat" model "opus-compat" via set_model
    let result = handle.set_model(&sid, "claude-compat", "opus-compat");

    // @step Then set_model returns Ok
    assert!(result.is_ok(), "set_model should return Ok, got {result:?}");

    // @step And the inner session provider manager reports facade override "claude"
    let facade = facade_for(&manager, &sid).await?;
    assert_eq!(
        facade.as_deref(),
        Some("claude"),
        "facade should be derived from anthropic_messages api_style, got {facade:?}"
    );
    Ok(())
}

// =============================================================================
// Scenario: Switching from a custom model to a registry model clears the stale facade
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn switching_from_custom_to_registry_clears_facade() -> TestResult {
    // @step Given a session that has been switched to a custom provider "my-llm" model "llama-3.1-70b" whose inner facade override is "openai"
    let fixture = TestDirectory::new()?;
    let (manager, sid) = fixture.manager_with_default("anthropic/claude-opus-4-5")?;
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    handle.set_model(&sid, "my-llm", "llama-3.1-70b")?;
    assert_eq!(
        facade_for(&manager, &sid).await?.as_deref(),
        Some("openai"),
        "precondition: custom facade should be set before the registry switch"
    );

    // @step When I switch the session model to provider "google" model "gemini-2.5-pro" via set_model
    let result = handle.set_model(&sid, "google", "gemini-2.5-pro");

    // @step Then set_model returns Ok
    assert!(result.is_ok(), "set_model should return Ok, got {result:?}");

    // @step And the inner session provider manager reports no facade override
    let facade = facade_for(&manager, &sid).await?;
    assert_eq!(
        facade, None,
        "stale custom facade must be cleared on a registry switch, got {facade:?}"
    );
    Ok(())
}

// =============================================================================
// Scenario: Switching between two registry models leaves no facade override
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn switching_between_registry_models_leaves_no_facade() -> TestResult {
    // @step Given a session created on "anthropic/claude-opus-4-5" whose inner provider manager reports no facade override
    let fixture = TestDirectory::new()?;
    let (manager, sid) = fixture.manager_with_default("anthropic/claude-opus-4-5")?;
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    assert_eq!(
        facade_for(&manager, &sid).await?,
        None,
        "precondition: a plain registry session has no facade override"
    );

    // @step When I switch the session model to provider "google" model "gemini-2.5-pro" via set_model
    let result = handle.set_model(&sid, "google", "gemini-2.5-pro");

    // @step Then set_model returns Ok
    assert!(result.is_ok(), "set_model should return Ok, got {result:?}");

    // @step And the inner session provider manager reports no facade override
    let facade = facade_for(&manager, &sid).await?;
    assert_eq!(
        facade, None,
        "registry-to-registry switch must not introduce a facade, got {facade:?}"
    );
    Ok(())
}
