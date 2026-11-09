//! BUG-511: editing a custom model in the TUI keeps its MODEL-016
//! `thinkingLevels` declaration.
//!
//! Feature: spec/features/editing-a-custom-model-keeps-its-thinking-levels.feature
//!
//! The TUI custom-model dialog edits through
//! `SessionManagerHandle::update_custom_model` / `add_custom_model`. These
//! tests drive a real `SessionManager` through that handle against a
//! temporary `fspec-config.json` (`FSPEC_USER_DIR`, serialized because the
//! variable is process-global) and read the stored entry back from disk.

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::CustomModelDefinition;
use codelet_sessions::profile_sections::load_local_server_profiles;
use codelet_sessions::SessionManager;
use serde_json::{json, Value};
use serial_test::serial;
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

const DECLARED_EFFORTS: [&str; 4] = ["none", "low", "medium", "xhigh"];

fn make_handle() -> Arc<dyn SessionManagerHandle> {
    SessionManager::new() as Arc<dyn SessionManagerHandle>
}

/// Write `fspec-config.json` with the given `openai` profiles into `dir` and
/// point `FSPEC_USER_DIR` at it.
fn seed_config(dir: &Path, profiles: Value) -> TestResult {
    let root = json!({ "providers": { "openai": { "profiles": profiles } } });
    std::fs::write(
        dir.join("fspec-config.json"),
        serde_json::to_string_pretty(&root)?,
    )?;
    std::env::set_var("FSPEC_USER_DIR", dir);
    Ok(())
}

/// A Strata profile whose custom model `qwen3.8-flash-next` declares the
/// efforts none, low, medium and xhigh.
fn seed_declared_strata(dir: &Path) -> TestResult {
    seed_config(
        dir,
        json!({
            "strata": {
                "baseUrl": "http://localhost:8000/v1",
                "engine": "strata",
                "customModels": [{
                    "id": "qwen3.8-flash-next",
                    "thinkingLevels": { "efforts": DECLARED_EFFORTS }
                }]
            }
        }),
    )
}

fn custom_models(dir: &Path, profile: &str) -> Result<Vec<Value>, Box<dyn Error>> {
    let content = std::fs::read_to_string(dir.join("fspec-config.json"))?;
    let root: Value = serde_json::from_str(&content)?;
    root["providers"]["openai"]["profiles"][profile]["customModels"]
        .as_array()
        .cloned()
        .ok_or_else(|| format!("profile {profile} has no customModels array").into())
}

fn stored_entry(dir: &Path, profile: &str, id: &str) -> Result<Value, Box<dyn Error>> {
    custom_models(dir, profile)?
        .into_iter()
        .find(|entry| entry["id"] == id)
        .ok_or_else(|| format!("no stored entry {id} in profile {profile}").into())
}

/// What the dialog sends: only `id` and the context window it edited.
fn dialog_definition(id: &str, context_window: Option<u32>) -> CustomModelDefinition {
    CustomModelDefinition {
        id: id.to_string(),
        display_name: None,
        facade: None,
        context_window,
        max_output_tokens: None,
        compaction_threshold_type: None,
        compaction_threshold_value: None,
        reasoning: None,
        has_vision: None,
    }
}

fn declared_efforts() -> Value {
    json!({ "efforts": DECLARED_EFFORTS })
}

/// Scenario: Editing a declared custom model's context window keeps its thinking levels
#[test]
#[serial]
fn editing_a_declared_custom_model_keeps_its_thinking_levels() -> TestResult {
    let tmp = TempDir::new()?;
    // @step Given a Strata profile whose custom model "qwen3.8-flash-next" declares thinking levels with efforts none, low, medium and xhigh
    seed_declared_strata(tmp.path())?;
    let handle = make_handle();

    // @step When the user edits that custom model's context window to 131072 in the TUI
    handle.update_custom_model(
        "openai",
        "strata",
        "qwen3.8-flash-next",
        &dialog_definition("qwen3.8-flash-next", Some(131_072)),
    )?;

    let entry = stored_entry(tmp.path(), "strata", "qwen3.8-flash-next")?;
    // @step Then the stored entry has context window 131072
    assert_eq!(entry["contextWindow"], json!(131_072));
    // @step And the stored entry still declares the efforts none, low, medium and xhigh
    assert_eq!(entry["thinkingLevels"], declared_efforts());

    // @step And the model still offers none, low, medium and xhigh
    let profiles = load_local_server_profiles()?;
    let profile = profiles
        .iter()
        .find(|profile| profile.name == "strata")
        .ok_or("the strata profile did not load")?;
    let reasoning = profile
        .model_reasoning("qwen3.8-flash-next")?
        .ok_or("qwen3.8-flash-next lost its reasoning control")?;
    assert_eq!(reasoning.capability.offered_levels(), DECLARED_EFFORTS);
    Ok(())
}

/// Scenario: Renaming a declared custom model keeps its thinking levels on the renamed entry
#[test]
#[serial]
fn renaming_a_declared_custom_model_keeps_its_thinking_levels() -> TestResult {
    let tmp = TempDir::new()?;
    // @step Given a Strata profile whose custom model "qwen3.8-flash-next" declares thinking levels with efforts none, low, medium and xhigh
    seed_declared_strata(tmp.path())?;
    let handle = make_handle();

    // @step When the user renames that custom model to "qwen3.8-flash-next-fp8" in the TUI
    handle.update_custom_model(
        "openai",
        "strata",
        "qwen3.8-flash-next",
        &dialog_definition("qwen3.8-flash-next-fp8", None),
    )?;

    // @step Then the profile has the custom model "qwen3.8-flash-next-fp8" and no "qwen3.8-flash-next"
    let ids: Vec<Value> = custom_models(tmp.path(), "strata")?
        .into_iter()
        .map(|entry| entry["id"].clone())
        .collect();
    assert_eq!(ids, vec![json!("qwen3.8-flash-next-fp8")]);

    // @step And the renamed entry still declares the efforts none, low, medium and xhigh
    let entry = stored_entry(tmp.path(), "strata", "qwen3.8-flash-next-fp8")?;
    assert_eq!(entry["thinkingLevels"], declared_efforts());
    Ok(())
}

/// Scenario: An edit keeps keys the dialog does not edit and removes dialog fields left empty
#[test]
#[serial]
fn an_edit_keeps_unedited_keys_and_removes_emptied_dialog_fields() -> TestResult {
    let tmp = TempDir::new()?;
    // @step Given a custom model "local-llm" stored with context window 32768 and a key "notes" that fspec does not know
    seed_config(
        tmp.path(),
        json!({
            "work-vllm": {
                "baseUrl": "http://localhost:8000/v1",
                "customModels": [{
                    "id": "local-llm",
                    "contextWindow": 32_768,
                    "notes": "quantised build"
                }]
            }
        }),
    )?;
    let handle = make_handle();

    // @step When the user edits "local-llm" in the TUI with the context window left empty
    handle.update_custom_model(
        "openai",
        "work-vllm",
        "local-llm",
        &dialog_definition("local-llm", None),
    )?;

    let entry = stored_entry(tmp.path(), "work-vllm", "local-llm")?;
    // @step Then the stored entry has no context window
    assert!(
        entry.get("contextWindow").is_none(),
        "the emptied context window must be removed: {entry}"
    );
    // @step And the stored entry still has its "notes" key
    assert_eq!(entry["notes"], json!("quantised build"));
    Ok(())
}

/// Scenario: Adding a custom model writes only the dialog's fields
#[test]
#[serial]
fn adding_a_custom_model_writes_only_the_dialog_fields() -> TestResult {
    let tmp = TempDir::new()?;
    // @step Given a Strata profile with no custom models
    seed_config(
        tmp.path(),
        json!({ "strata": { "baseUrl": "http://localhost:8000/v1", "engine": "strata" } }),
    )?;
    let handle = make_handle();

    // @step When the user adds the custom model "fresh-model" in the TUI
    handle.add_custom_model("openai", "strata", &dialog_definition("fresh-model", None))?;

    // @step Then the stored entry for "fresh-model" has no thinking levels declaration
    let entry = stored_entry(tmp.path(), "strata", "fresh-model")?;
    assert_eq!(entry, json!({ "id": "fresh-model" }));
    Ok(())
}
