//! PROV-123 — selecting a model in an ACTIVE session must update the global
//! `SessionManager` default so a NEW session created in the SAME process
//! inherits the just-selected model (no restart required).
//!
//! Feature: spec/features/active-session-model-selection-updates-default.feature
//!
//! Root cause (spec/attachments/PROV-123/PROV-123-analysis.md): the
//! active-session switch path (`handle_impl::set_model`) updated only that one
//! session + persisted `tui.lastUsedModel` (PROV-122) but NEVER touched the
//! in-memory `default_model` RwLock new sessions read via `get_default_model()`.
//! Fix: after the switch succeeds, call `SessionManager::set_default_model(self,
//! &model)` (RwLock + `default-model.json` + `tui.lastUsedModel`), superseding
//! the standalone PROV-122 persist call.
//!
//! Hermetic + offline: shared setup lives in the sibling `prov123_support`
//! module. Data-dir/`FSPEC_USER_DIR`-swapping tests are serialized via `DATA_DIR_GUARD`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod prov123_support;

use std::sync::Mutex;

use codelet_core::session_manager_handle::SessionManagerHandle;
use prov123_support::{
    manager_with_seeded_cache, read_config, read_default_model_json, restore_user_dir,
    seed_qwen_profile,
};

/// Serializes every test that swaps the process-global data directory and the
/// `FSPEC_USER_DIR` env var, so a parallel test cannot observe another test's
/// redirect or persisted files (PROV-118/119 precedent).
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

// =============================================================================
// Scenario: Active-session model switch updates the global default
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_session_switch_updates_global_default() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given the global default model is "anthropic/claude-sonnet-4"
    let (_data_dir, manager, saved) = manager_with_seeded_cache()?;
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step And an active session is running
    let sid = handle.create_session(None);
    assert!(!sid.value.is_empty(), "an active session must be created");

    // @step When the user selects model "anthropic/claude-opus-4-8" in the active session and the switch succeeds
    let result = handle.set_model(&sid, "anthropic", "claude-opus-4-8");
    assert!(
        result.is_ok(),
        "the model switch must succeed, got {result:?}"
    );

    // @step Then the global default model is "anthropic/claude-opus-4-8"
    assert_eq!(
        manager.get_default_model().as_deref(),
        Some("anthropic/claude-opus-4-8"),
        "an active-session switch must update the global default so new sessions inherit it"
    );

    restore_user_dir(saved);
    Ok(())
}

// =============================================================================
// Scenario: A new session inherits the model selected in the active session
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn new_session_inherits_active_session_selection() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given an active session has switched to model "anthropic/claude-opus-4-8"
    let (_data_dir, manager, saved) = manager_with_seeded_cache()?;
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let sid_a = handle.create_session(None);
    assert!(!sid_a.value.is_empty(), "the first session must be created");
    handle
        .set_model(&sid_a, "anthropic", "claude-opus-4-8")
        .map_err(|e| format!("switch must succeed: {e}"))?;

    // @step When the user creates a new session in the same process
    let sid_b = handle.create_session(None);
    assert!(!sid_b.value.is_empty(), "the new session must be created");

    // @step Then the new session is created using "anthropic/claude-opus-4-8"
    let model_b = handle.get_session_model(&sid_b);
    assert_eq!(
        model_b.provider_id, "anthropic",
        "provider must be anthropic"
    );
    assert_eq!(
        model_b.model_id, "claude-opus-4-8",
        "new session must inherit the opus-4-8 selected in the active session"
    );

    // @step And the new session does not use the startup model "anthropic/claude-sonnet-4"
    assert_ne!(
        model_b.model_id, "claude-sonnet-4",
        "new session must NOT fall back to the stale startup model"
    );

    restore_user_dir(saved);
    Ok(())
}

// =============================================================================
// Scenario: The active-session switch persists the new default to disk
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_session_switch_persists_default_to_disk() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given an active session is running
    let (data_dir, manager, saved) = manager_with_seeded_cache()?;
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let sid = handle.create_session(None);
    assert!(!sid.value.is_empty(), "an active session must be created");

    // @step When the user selects model "anthropic/claude-opus-4-8" in the active session and the switch succeeds
    handle
        .set_model(&sid, "anthropic", "claude-opus-4-8")
        .map_err(|e| format!("switch must succeed: {e}"))?;

    // @step Then default-model.json records model "anthropic/claude-opus-4-8"
    let legacy =
        read_default_model_json(data_dir.path()).expect("default-model.json must be written");
    assert_eq!(
        legacy["model"].as_str(),
        Some("anthropic/claude-opus-4-8"),
        "default-model.json must record the just-selected model (set_default_model write side)"
    );

    // @step And fspec-config.json tui.lastUsedModel equals "anthropic/claude-opus-4-8"
    let config = read_config(data_dir.path()).expect("fspec-config.json must be written");
    assert_eq!(
        config["tui"]["lastUsedModel"].as_str(),
        Some("anthropic/claude-opus-4-8"),
        "fspec-config.json tui.lastUsedModel must record the just-selected model"
    );

    restore_user_dir(saved);
    Ok(())
}

// =============================================================================
// Scenario: Creating an isolated session leaves the global default unchanged
// DEVIATION NOTE (sanctioned by supervisor): a worktree-backed isolated session
// needs a real git repo + worktree + agent-loop spawn (heavy/flaky in-process).
// We assert the UNIT GUARANTEE: the isolated create path does NOT call
// `self.set_default_model(...)` while the non-isolated path DOES. The global
// default is mutated ONLY via set_default_model, so this proves it is unchanged.
// =============================================================================
#[test]
fn isolated_create_path_does_not_touch_global_default() {
    // @step Given the global default model is "anthropic/claude-opus-4-8"
    let src = include_str!("../src/session_manager.rs");

    let iso_start = src
        .find("pub async fn create_isolated_session_with_id")
        .expect("create_isolated_session_with_id must exist");
    let non_iso_start = src
        .find("pub async fn create_session_with_id")
        .expect("create_session_with_id must exist");

    let iso_rest = &src[iso_start..];
    let iso_body = &iso_rest[..iso_rest[1..]
        .find("\n    pub ")
        .map(|i| i + 1)
        .unwrap_or(iso_rest.len())];

    let non_iso_rest = &src[non_iso_start..];
    let non_iso_body = &non_iso_rest[..non_iso_rest[1..]
        .find("\n    pub ")
        .map(|i| i + 1)
        .unwrap_or(non_iso_rest.len())];

    // @step When the user creates an isolated session
    // @step Then the global default model is still "anthropic/claude-opus-4-8"
    assert!(
        !iso_body.contains("self.set_default_model("),
        "create_isolated_session_with_id MUST NOT call self.set_default_model — isolated \
         sessions are ephemeral and must not mutate the global default (PROV-123 rule 5)"
    );
    // Control: the non-isolated path MUST call it, proving the assertion above is
    // discriminating and not trivially true.
    assert!(
        non_iso_body.contains("self.set_default_model("),
        "create_session_with_id MUST call self.set_default_model (control for the isolated assertion)"
    );
}

// =============================================================================
// Scenario: A profile-qualified selection updates the default and round-trips to
//           a new session
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn profile_qualified_selection_updates_default_and_round_trips() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given an active session is running
    let (data_dir, manager, saved) = manager_with_seeded_cache()?;
    seed_qwen_profile(data_dir.path());
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let sid = handle.create_session(None);
    assert!(!sid.value.is_empty(), "an active session must be created");

    // @step When the user selects model "openai:qwen/Qwen3-80B" in the active session and the switch succeeds
    let result = handle.set_model(&sid, "openai:qwen", "Qwen3-80B");
    assert!(
        result.is_ok(),
        "the profile-qualified switch must succeed, got {result:?}"
    );

    // @step Then the global default model is "openai:qwen/Qwen3-80B"
    assert_eq!(
        manager.get_default_model().as_deref(),
        Some("openai:qwen/Qwen3-80B"),
        "the global default must hold the profile-qualified composite a new session can resolve"
    );

    // @step And a new session created in the same process resolves "openai:qwen/Qwen3-80B"
    let sid_b = handle.create_session(None);
    assert!(
        !sid_b.value.is_empty(),
        "a new session must resolve the profile-qualified default and be created"
    );
    let model_b = handle.get_session_model(&sid_b);
    assert_eq!(
        model_b.provider_id, "openai",
        "the new session must resolve the openai provider from the profile composite"
    );
    assert_eq!(
        model_b.model_id, "Qwen3-80B",
        "the new session must resolve the Qwen3-80B model from the profile composite"
    );

    restore_user_dir(saved);
    Ok(())
}

// =============================================================================
// Scenario: An empty model selection never overwrites the global default
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn empty_selection_never_overwrites_global_default() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given the global default model is "anthropic/claude-opus-4-8"
    let (data_dir, manager, saved) = manager_with_seeded_cache()?;
    manager.set_default_model("anthropic/claude-opus-4-8");
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let sid = handle.create_session(None);
    assert!(!sid.value.is_empty(), "an active session must be created");

    // @step When an empty model string is applied to the active-session switch path
    // set_model builds `model = "{provider_id}/{model_id}"`; empty ids yield the
    // invalid "/" which the shared resolver rejects BEFORE any state mutation, so
    // set_default_model is never reached with an empty value (PROV-101 invariant).
    let result = handle.set_model(&sid, "", "");
    assert!(
        result.is_err(),
        "an empty selection must be rejected, never applied, got {result:?}"
    );

    // @step Then the global default model is still "anthropic/claude-opus-4-8"
    assert_eq!(
        manager.get_default_model().as_deref(),
        Some("anthropic/claude-opus-4-8"),
        "an empty selection must never overwrite the global default"
    );

    // @step And no empty value is written to disk
    let legacy = read_default_model_json(data_dir.path()).expect("default-model.json must exist");
    assert_eq!(
        legacy["model"].as_str(),
        Some("anthropic/claude-opus-4-8"),
        "default-model.json must still hold opus-4-8 — no empty value persisted"
    );

    restore_user_dir(saved);
    Ok(())
}
