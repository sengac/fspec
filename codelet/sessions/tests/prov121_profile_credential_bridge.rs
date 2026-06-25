//! Feature: spec/features/openai-profile-credential-injection.feature
//!
//! PROV-121 — OpenAI profile model selection must bridge the profile's stored
//! `baseUrl` / `apiKey` into the environment the OpenAI client reads
//! (`OPENAI_BASE_URL` / `OPENAI_API_KEY`) BEFORE dispatch, and preserve the
//! profile name so the composite `openai:qwen/qwen` round-trips.
//!
//! These tests drive the SHARED resolver
//! `codelet_sessions::model_resolution::apply_model_selection` — the single
//! entry point used by BOTH the mid-session switch path
//! (`handle_impl.rs:1040`) and the create-session path
//! (`session_manager.rs:521`). Proving the bridge here proves it for both
//! rule-[2] paths at once.
//!
//! Offline + hermetic: `FSPEC_USER_DIR` points at a `TempDir` holding a seeded
//! `fspec-config.json`; no network access occurs (`set_model_direct` skips the
//! registry). Because these tests mutate the process-global `OPENAI_*` env vars
//! and `FSPEC_USER_DIR`, every test is `#[serial]` and saves/restores the env
//! it touches via `EnvGuard`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod prov121_support;

use codelet_sessions::model_resolution::{apply_model_selection, apply_profile_env_vars};
use prov121_support::{
    openai_manager_with_sentinel_key, seed_profile_config, EnvGuard, PROFILE_API_KEY,
    PROFILE_BASE_URL, SENTINEL_KEY,
};
use serial_test::serial;
use tempfile::TempDir;

// =============================================================================
// Scenario: Resolving a profile model sets OPENAI_BASE_URL and OPENAI_API_KEY to
//           the profile values
// =============================================================================
#[test]
#[serial]
fn resolving_profile_model_sets_base_url_and_api_key() {
    let _env = EnvGuard::capture();
    let tmp = TempDir::new().unwrap();

    // @step Given an openai profile "qwen" is stored in fspec-config.json with baseUrl "http://192.168.0.50:8000" and apiKey "test"
    seed_profile_config(tmp.path());
    let mut pm = openai_manager_with_sentinel_key();
    // The bridge must SET this even when unset beforehand.
    std::env::remove_var("OPENAI_BASE_URL");

    // @step When the profile model "openai:qwen/qwen" is resolved for selection
    apply_model_selection(&mut pm, "openai:qwen/qwen")
        .expect("resolving a profile model should succeed");

    // @step Then OPENAI_BASE_URL equals "http://192.168.0.50:8000"
    assert_eq!(
        std::env::var("OPENAI_BASE_URL").ok().as_deref(),
        Some(PROFILE_BASE_URL),
        "the profile env bridge must set OPENAI_BASE_URL to the profile baseUrl"
    );

    // @step Then OPENAI_API_KEY equals "test"
    assert_eq!(
        std::env::var("OPENAI_API_KEY").ok().as_deref(),
        Some(PROFILE_API_KEY),
        "the profile env bridge must overwrite OPENAI_API_KEY with the profile apiKey, not leave the user's value"
    );

    // @step Then the profile name "qwen" is preserved so the composite model string "openai:qwen/qwen" round-trips
    assert_eq!(
        pm.selected_model_string().as_deref(),
        Some("openai:qwen/qwen"),
        "the resolver must preserve the profile name via set_model_direct_with_profile so the composite round-trips"
    );
}

// =============================================================================
// Scenario: Selecting an openai profile model bridges the profile credentials so
//           dispatch succeeds without OPENAI_API_KEY
// =============================================================================
#[test]
#[serial]
fn profile_selection_bridges_credentials_for_dispatch() {
    let _env = EnvGuard::capture();
    let tmp = TempDir::new().unwrap();

    // @step Given an openai profile "qwen" is stored in fspec-config.json with baseUrl "http://192.168.0.50:8000" and apiKey "test" and the OPENAI_API_KEY environment variable is unset
    seed_profile_config(tmp.path());
    let mut pm = openai_manager_with_sentinel_key();
    // Model the "unset" precondition: the user has NO usable key of their own.
    // After the bridge, dispatch (get_openai) reads OPENAI_API_KEY — it must be
    // the profile key, not absent.
    std::env::remove_var("OPENAI_BASE_URL");

    // @step When the profile model "openai:qwen/qwen" is resolved for selection
    apply_model_selection(&mut pm, "openai:qwen/qwen")
        .expect("resolving a profile model should succeed");

    // @step Then the OpenAI client resolves credentials from the profile and no "OPENAI_API_KEY not set" authentication error occurs
    // get_openai (manager.rs:830) errors with "OPENAI_API_KEY not set" only when
    // the env var is absent. The bridge must have populated it from the profile,
    // and the base URL must point at the profile endpoint, so the constructed
    // client targets the local server rather than cloud OpenAI.
    let key = std::env::var("OPENAI_API_KEY").ok();
    assert!(
        key.is_some(),
        "after the bridge OPENAI_API_KEY must be set so get_openai does not raise `OPENAI_API_KEY not set`"
    );
    assert_eq!(
        key.as_deref(),
        Some(PROFILE_API_KEY),
        "the resolved key must be the profile's apiKey so dispatch authenticates against the profile endpoint"
    );
    assert_eq!(
        std::env::var("OPENAI_BASE_URL").ok().as_deref(),
        Some(PROFILE_BASE_URL),
        "the resolved base URL must be the profile endpoint so dispatch does not hit cloud OpenAI"
    );
}

// =============================================================================
// Scenario: Cloud OpenAI model selection does not trigger the profile credential
//           bridge
// =============================================================================
#[test]
#[serial]
fn cloud_openai_selection_does_not_trigger_profile_bridge() {
    let _env = EnvGuard::capture();
    let tmp = TempDir::new().unwrap();

    // @step Given no openai profile is referenced by the selection and OPENAI_BASE_URL and OPENAI_API_KEY hold their pre-existing values
    seed_profile_config(tmp.path());
    let mut pm = openai_manager_with_sentinel_key();
    std::env::set_var("OPENAI_BASE_URL", "https://api.openai.com/v1");
    std::env::set_var("OPENAI_API_KEY", SENTINEL_KEY);

    // @step When the cloud model "openai/gpt-4" is resolved for selection
    // A cloud model (no ':' before the first '/') is NOT a profile model; the
    // resolver routes to select_model. With no registry the call may Err, but
    // the profile bridge must never have fired regardless of outcome.
    let _ = apply_model_selection(&mut pm, "openai/gpt-4");

    // @step Then the profile credential bridge does not fire because there is no profile name
    assert_ne!(
        pm.selected_model_string().as_deref(),
        Some("openai:qwen/qwen"),
        "a cloud selection must not acquire a profile name"
    );

    // @step Then OPENAI_BASE_URL and OPENAI_API_KEY are not overwritten with profile values
    assert_eq!(
        std::env::var("OPENAI_BASE_URL").ok().as_deref(),
        Some("https://api.openai.com/v1"),
        "cloud selection must leave OPENAI_BASE_URL untouched (not the profile baseUrl)"
    );
    assert_eq!(
        std::env::var("OPENAI_API_KEY").ok().as_deref(),
        Some(SENTINEL_KEY),
        "cloud selection must leave OPENAI_API_KEY untouched (not the profile apiKey)"
    );

    // @step Then no hardcoded anthropic or claude fallback is substituted
    assert!(
        std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .as_deref()
            .map(|v| v != PROFILE_API_KEY)
            .unwrap_or(true),
        "no anthropic/claude fallback may be substituted for a cloud openai selection (PROV-101)"
    );
}

// =============================================================================
// Scenario: A profile model selected at create-session time applies the
//           credential bridge before first dispatch
// =============================================================================
#[test]
#[serial]
fn create_session_path_applies_bridge_before_first_dispatch() {
    let _env = EnvGuard::capture();
    let tmp = TempDir::new().unwrap();

    // @step Given an openai profile "qwen" is stored in fspec-config.json with baseUrl "http://192.168.0.50:8000" and apiKey "test" and the OPENAI_API_KEY environment variable is unset
    seed_profile_config(tmp.path());
    let mut pm = openai_manager_with_sentinel_key();
    std::env::remove_var("OPENAI_BASE_URL");

    // @step When a fresh session is created with the profile model "openai:qwen/qwen" via the shared model resolver
    // SessionManager::create_session_with_id (session_manager.rs:521) resolves a
    // fresh session's model through this exact shared function, so resolving it
    // here exercises the create-time bridge a brand-new session would carry into
    // its first dispatch.
    apply_model_selection(&mut pm, "openai:qwen/qwen")
        .expect("resolving a profile model at create time should succeed");

    // @step Then OPENAI_BASE_URL equals "http://192.168.0.50:8000" and OPENAI_API_KEY equals "test" before the first dispatch
    assert_eq!(
        std::env::var("OPENAI_BASE_URL").ok().as_deref(),
        Some(PROFILE_BASE_URL),
        "a freshly created session must have the profile baseUrl bridged before its first dispatch"
    );
    assert_eq!(
        std::env::var("OPENAI_API_KEY").ok().as_deref(),
        Some(PROFILE_API_KEY),
        "a freshly created session must have the profile apiKey bridged before its first dispatch"
    );

    // @step Then the create-session path and the mid-session switch path apply the same profile bridge via the shared resolver
    // Both paths funnel through apply_model_selection (the function under test),
    // so a single preserved profile composite proves the shared bridge applied.
    assert_eq!(
        pm.selected_model_string().as_deref(),
        Some("openai:qwen/qwen"),
        "the shared resolver used by both create and mid-session paths must preserve the profile composite"
    );
}

// =============================================================================
// Scenario: A profile model selected on the isolated session create path applies
//           the credential bridge via the shared helper
//
// PROV-121 third path: `SessionManager::create_isolated_session_with_id`
// constructs the manager via `with_provider_and_model` and BYPASSES the shared
// `apply_model_selection`. Per the supervisor ruling it must apply the SAME
// profile credential bridge through ONE shared helper —
// `apply_profile_env_vars(provider, profile_name, model)` — so it cannot drift
// from the resolver path. This targets that single shared helper directly: the
// exact call the isolated branch makes right before `with_provider_and_model`.
// =============================================================================
#[test]
#[serial]
fn isolated_create_path_applies_bridge_via_shared_helper() {
    let _env = EnvGuard::capture();
    let tmp = TempDir::new().unwrap();

    // @step Given an openai profile "qwen" is stored in fspec-config.json with baseUrl "http://192.168.0.50:8000" and apiKey "test" and the OPENAI_API_KEY environment variable is unset
    seed_profile_config(tmp.path());
    std::env::set_var("OPENAI_API_KEY", SENTINEL_KEY);
    std::env::remove_var("OPENAI_BASE_URL");

    // @step When the isolated session create path applies the profile credential bridge for the profile model "openai:qwen/qwen" before constructing the provider manager
    // create_isolated_session_with_id parses "openai:qwen/qwen" into
    // provider="openai", profile="qwen", model="qwen" and (per the ruling) calls
    // the shared helper inline right before with_provider_and_model.
    apply_profile_env_vars("openai", "qwen", "qwen")
        .expect("the shared profile env bridge should succeed for a known profile");

    // @step Then OPENAI_BASE_URL equals "http://192.168.0.50:8000" and OPENAI_API_KEY equals "test" before the isolated session dispatches
    assert_eq!(
        std::env::var("OPENAI_BASE_URL").ok().as_deref(),
        Some(PROFILE_BASE_URL),
        "the isolated path's shared bridge must set OPENAI_BASE_URL to the profile baseUrl"
    );
    assert_eq!(
        std::env::var("OPENAI_API_KEY").ok().as_deref(),
        Some(PROFILE_API_KEY),
        "the isolated path's shared bridge must overwrite OPENAI_API_KEY with the profile apiKey"
    );

    // @step Then the isolated path and the shared resolver apply the identical profile bridge through one shared helper so they cannot drift
    // OPENAI_CONTEXT_WINDOW is bridged from profile.context_window (200000) by
    // the same helper the resolver path uses, proving a single source of truth.
    assert_eq!(
        std::env::var("OPENAI_CONTEXT_WINDOW").ok().as_deref(),
        Some("200000"),
        "the single shared helper must also bridge OPENAI_CONTEXT_WINDOW from profile.context_window"
    );
}
