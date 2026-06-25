//! PROV-122 — model selection persists `tui.lastUsedModel` to fspec-config.json.
//!
//! Feature: spec/features/model-selection-persists-last-used-model.feature
//!
//! PROV-120 implemented the READ side; this suite drives the WRITE side. The
//! writer (`save_persisted_model_string_to` / env-resolved
//! `save_persisted_model_string`) does a key-preserving read-merge-write of
//! `tui.lastUsedModel`, mirroring the TS `modelSelectionService.selectModel`.
//!
//! Most scenarios exercise the path-injectable writer core against a throwaway
//! `TempDir` (offline, no env, no shared state). The "no active session" path
//! (scenario 4) exercises the REAL `SessionManager::set_default_model` wiring,
//! pointing BOTH the global data dir and `FSPEC_USER_DIR` at one temp dir so it
//! provably writes both stores; that test is serialized and restores the env.
//!
//! RED phase: the writer does not exist yet, so this file fails to compile —
//! that compile failure is the proof the WRITE side is unimplemented.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::sync::{Arc, Mutex};

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::last_used_model_persistence::{
    load_persisted_model_string_from, save_persisted_model_string, save_persisted_model_string_to,
};
use codelet_sessions::SessionManager;
use serde_json::Value;
use tempfile::TempDir;

/// Serializes the scenario-4 test that swaps the process-global data dir and the
/// `FSPEC_USER_DIR` env var so a parallel test cannot observe the redirect.
static ENV_GUARD: Mutex<()> = Mutex::new(());

/// Read `fspec-config.json` from `dir` as a JSON value, or `None` when absent.
fn read_config(dir: &std::path::Path) -> Option<Value> {
    let content = fs::read_to_string(dir.join("fspec-config.json")).ok()?;
    serde_json::from_str(&content).ok()
}

// =============================================================================
// Scenario: Persisting a model when no config file exists creates it
// =============================================================================
#[test]
fn persisting_when_no_config_exists_creates_it() {
    // @step Given the user directory has no fspec-config.json
    let dir = TempDir::new().unwrap();
    assert!(read_config(dir.path()).is_none(), "precondition: no config");

    // @step When the model "anthropic/claude-opus-4-8" is persisted as the last used model
    save_persisted_model_string_to(dir.path(), "anthropic/claude-opus-4-8")
        .expect("persist should succeed");

    // @step Then fspec-config.json is created
    let config = read_config(dir.path()).expect("fspec-config.json must be created");

    // @step And tui.lastUsedModel equals "anthropic/claude-opus-4-8"
    assert_eq!(
        config["tui"]["lastUsedModel"].as_str(),
        Some("anthropic/claude-opus-4-8")
    );
}

// =============================================================================
// Scenario: Persisting a model preserves all other config keys
// =============================================================================
#[test]
fn persisting_preserves_all_other_config_keys() {
    // @step Given fspec-config.json already has providers, research, and tui.fallbackImageModel keys
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("fspec-config.json"),
        r#"{"providers":{"openai":{"profiles":{}}},"research":{"enabled":true},"tui":{"fallbackImageModel":"anthropic/claude-haiku"}}"#,
    )
    .unwrap();

    // @step When the model "anthropic/claude-opus-4-8" is persisted as the last used model
    save_persisted_model_string_to(dir.path(), "anthropic/claude-opus-4-8")
        .expect("persist should succeed");

    // @step Then tui.lastUsedModel equals "anthropic/claude-opus-4-8"
    let config = read_config(dir.path()).unwrap();
    assert_eq!(
        config["tui"]["lastUsedModel"].as_str(),
        Some("anthropic/claude-opus-4-8")
    );

    // @step And the providers, research, and tui.fallbackImageModel keys are unchanged
    assert_eq!(
        config["providers"]["openai"]["profiles"],
        serde_json::json!({})
    );
    assert_eq!(config["research"]["enabled"], Value::Bool(true));
    assert_eq!(
        config["tui"]["fallbackImageModel"].as_str(),
        Some("anthropic/claude-haiku")
    );
}

// =============================================================================
// Scenario: Selecting a model with an active session persists the choice
// =============================================================================
#[test]
fn active_session_selection_persists_and_round_trips() {
    // @step Given an active session is using model "anthropic/claude-sonnet-4"
    // The active-session switch path (handle_impl::set_model) persists the newly
    // built `provider_id/model_id` string via this exact writer after the
    // in-memory switch succeeds. Seed the prior selection to model the "before".
    let dir = TempDir::new().unwrap();
    save_persisted_model_string_to(dir.path(), "anthropic/claude-sonnet-4")
        .expect("seed prior selection");

    // @step When the user selects model "anthropic/claude-opus-4-8" and the switch succeeds
    save_persisted_model_string_to(dir.path(), "anthropic/claude-opus-4-8")
        .expect("persist on successful switch");

    // @step Then fspec-config.json tui.lastUsedModel equals "anthropic/claude-opus-4-8"
    let config = read_config(dir.path()).unwrap();
    assert_eq!(
        config["tui"]["lastUsedModel"].as_str(),
        Some("anthropic/claude-opus-4-8")
    );

    // @step And reloading the persisted model returns "anthropic/claude-opus-4-8"
    assert_eq!(
        load_persisted_model_string_from(dir.path()).as_deref(),
        Some("anthropic/claude-opus-4-8")
    );
}

// =============================================================================
// Scenario: Selecting a model with no active session writes both stores
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_session_selection_writes_both_stores() {
    let _guard = ENV_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let saved_user_dir = std::env::var("FSPEC_USER_DIR").ok();

    // @step Given there is no active session
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_path_buf();
    codelet_common::set_data_directory(dir.clone()).expect("set data dir");
    std::env::set_var("FSPEC_USER_DIR", &dir);
    let manager = Arc::new(SessionManager::new());
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step When the user selects model "anthropic/claude-opus-4-8" as the default
    handle.set_default_model("anthropic/claude-opus-4-8");

    // @step Then fspec-config.json tui.lastUsedModel equals "anthropic/claude-opus-4-8"
    let config = read_config(&dir).expect("fspec-config.json must be written");
    assert_eq!(
        config["tui"]["lastUsedModel"].as_str(),
        Some("anthropic/claude-opus-4-8")
    );

    // @step And default-model.json records model "anthropic/claude-opus-4-8"
    let legacy: Value =
        serde_json::from_str(&fs::read_to_string(dir.join("default-model.json")).unwrap()).unwrap();
    assert_eq!(legacy["model"].as_str(), Some("anthropic/claude-opus-4-8"));

    // restore env
    match saved_user_dir {
        Some(v) => std::env::set_var("FSPEC_USER_DIR", v),
        None => std::env::remove_var("FSPEC_USER_DIR"),
    }
}

// =============================================================================
// Scenario: An empty model string is never persisted
// =============================================================================
#[test]
fn empty_model_string_is_never_persisted() {
    // @step Given fspec-config.json has no tui.lastUsedModel key
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("fspec-config.json"), r#"{"theme":"dark"}"#).unwrap();
    let before = fs::read_to_string(dir.path().join("fspec-config.json")).unwrap();

    // @step When an empty model string is passed to the persist path
    save_persisted_model_string_to(dir.path(), "   ").expect("empty persist is a no-op success");

    // @step Then fspec-config.json is left untouched
    let after = fs::read_to_string(dir.path().join("fspec-config.json")).unwrap();
    assert_eq!(before, after, "an empty model must not rewrite the config");

    // @step And no tui.lastUsedModel key is written
    let config = read_config(dir.path()).unwrap();
    assert!(config.get("tui").is_none(), "no tui key must be created");
}

// =============================================================================
// Scenario: A profile-qualified model selection round-trips
// =============================================================================
#[test]
fn profile_qualified_selection_round_trips() {
    // @step Given the user directory has no fspec-config.json
    let dir = TempDir::new().unwrap();
    assert!(read_config(dir.path()).is_none(), "precondition: no config");

    // @step When the model "openai:qwen/Qwen3-80B" is persisted as the last used model
    // Use the env-resolved writer to also prove FSPEC_USER_DIR resolution.
    let _guard = ENV_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let saved_user_dir = std::env::var("FSPEC_USER_DIR").ok();
    std::env::set_var("FSPEC_USER_DIR", dir.path());
    save_persisted_model_string("openai:qwen/Qwen3-80B").expect("persist should succeed");

    // @step Then tui.lastUsedModel equals "openai:qwen/Qwen3-80B"
    let config = read_config(dir.path()).unwrap();
    assert_eq!(
        config["tui"]["lastUsedModel"].as_str(),
        Some("openai:qwen/Qwen3-80B")
    );

    // @step And reloading the persisted model returns "openai:qwen/Qwen3-80B"
    assert_eq!(
        load_persisted_model_string_from(dir.path()).as_deref(),
        Some("openai:qwen/Qwen3-80B")
    );

    match saved_user_dir {
        Some(v) => std::env::set_var("FSPEC_USER_DIR", v),
        None => std::env::remove_var("FSPEC_USER_DIR"),
    }
}
