//! Feature: spec/features/cloud-model-catalog-population.feature
//!
//! RPC-073 (reopened 2026-06-19): the model selector showed cloud provider
//! rows with EMPTY model lists. `cloud_model_entries` populates each built-in
//! provider's tool-capable models from the models.dev registry, credential-
//! gated, mirroring the TypeScript `buildCloudSections`. These tests exercise
//! the pure helper against a `ModelRegistry::from_response` built from inline
//! models.dev JSON — fully offline, no cache, no network, no tokio runtime.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_providers::models::{ModelRegistry, ModelsDevResponse};
use codelet_sessions::cloud_models::{canonical_to_models_dev, cloud_model_entries};

fn registry_from(json: &str) -> ModelRegistry {
    let resp: ModelsDevResponse = serde_json::from_str(json).expect("valid models.dev json");
    ModelRegistry::from_response(resp)
}

fn anthropic_registry() -> ModelRegistry {
    registry_from(
        r#"{
            "anthropic": {
                "id": "anthropic",
                "name": "Anthropic",
                "env": ["ANTHROPIC_API_KEY"],
                "models": {
                    "claude-opus-4": {
                        "id": "claude-opus-4",
                        "name": "Claude Opus 4",
                        "release_date": "2026-01-01",
                        "reasoning": true,
                        "tool_call": true,
                        "modalities": {"input": ["text", "image"], "output": ["text"]},
                        "limit": {"context": 200000, "output": 64000}
                    },
                    "claude-sonnet-4": {
                        "id": "claude-sonnet-4",
                        "name": "Claude Sonnet 4",
                        "release_date": "2025-05-01",
                        "reasoning": true,
                        "tool_call": true,
                        "limit": {"context": 200000, "output": 64000}
                    },
                    "claude-legacy": {
                        "id": "claude-legacy",
                        "name": "Claude Legacy",
                        "release_date": "2023-01-01",
                        "tool_call": true,
                        "status": "deprecated",
                        "limit": {"context": 100000, "output": 4096}
                    }
                }
            }
        }"#,
    )
}

#[test]
fn credentialed_provider_lists_tool_call_models_newest_first_dropping_deprecated() {
    // @step Given a models.dev registry where "anthropic" lists claude-opus-4 (tool_call, 2026-01-01), claude-sonnet-4 (tool_call, 2025-05-01) and an old deprecated model
    let registry = anthropic_registry();
    // @step And the "anthropic" provider is treated as credentialed
    let has_creds = true;
    // @step When cloud_model_entries is built for "anthropic"
    let entries = cloud_model_entries(&registry, "anthropic", has_creds);
    let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    // @step Then the entries contain exactly claude-opus-4 and claude-sonnet-4
    assert_eq!(ids, vec!["claude-opus-4", "claude-sonnet-4"]);
    // @step And claude-opus-4 appears before claude-sonnet-4 (newest-first)
    assert!(
        ids.iter().position(|i| *i == "claude-opus-4")
            < ids.iter().position(|i| *i == "claude-sonnet-4")
    );
    // @step And the deprecated model is not present
    assert!(!ids.contains(&"claude-legacy"));
}

#[test]
fn only_tool_call_capable_models_are_listed() {
    // @step Given a models.dev registry where "anthropic" lists a tool_call=false chat model alongside tool_call=true models
    let registry = registry_from(
        r#"{
            "anthropic": {
                "id": "anthropic", "name": "Anthropic", "env": ["ANTHROPIC_API_KEY"],
                "models": {
                    "tooly": {"id": "tooly", "name": "Tooly", "tool_call": true, "limit": {"context": 200000, "output": 8192}},
                    "chatty": {"id": "chatty", "name": "Chatty", "tool_call": false, "limit": {"context": 200000, "output": 8192}}
                }
            }
        }"#,
    );
    // @step And the "anthropic" provider is treated as credentialed
    let has_creds = true;
    // @step When cloud_model_entries is built for "anthropic"
    let entries = cloud_model_entries(&registry, "anthropic", has_creds);
    let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    // @step Then the tool_call=false model is excluded
    assert!(!ids.contains(&"chatty"));
    // @step And every returned entry is tool-call capable
    assert_eq!(ids, vec!["tooly"]);
}

#[test]
fn gemini_canonical_slug_is_sourced_from_google_entry() {
    // @step Given a models.dev registry where "google" lists a tool_call gemini model
    let registry = registry_from(
        r#"{
            "google": {
                "id": "google", "name": "Google", "env": ["GEMINI_API_KEY"],
                "models": {
                    "gemini-3-pro": {"id": "gemini-3-pro", "name": "Gemini 3 Pro", "tool_call": true, "limit": {"context": 1048576, "output": 65536}}
                }
            }
        }"#,
    );
    // @step And the "gemini" provider is treated as credentialed
    let has_creds = true;
    // @step When cloud_model_entries is built for "gemini"
    let entries = cloud_model_entries(&registry, "gemini", has_creds);
    // @step Then the entries are non-empty
    assert!(!entries.is_empty());
    // @step And they come from the models.dev "google" provider
    assert_eq!(entries[0].id, "gemini-3-pro");
    assert_eq!(canonical_to_models_dev("gemini"), "google");
}

#[test]
fn uncredentialed_provider_yields_no_models() {
    // @step Given a models.dev registry where "mistral" lists tool_call models
    let registry = registry_from(
        r#"{
            "mistral": {
                "id": "mistral", "name": "Mistral", "env": ["MISTRAL_API_KEY"],
                "models": {
                    "mistral-large": {"id": "mistral-large", "name": "Mistral Large", "tool_call": true, "limit": {"context": 128000, "output": 8192}}
                }
            }
        }"#,
    );
    // @step And the "mistral" provider is treated as NOT credentialed
    let has_creds = false;
    // @step When cloud_model_entries is built for "mistral"
    let entries = cloud_model_entries(&registry, "mistral", has_creds);
    // @step Then the entries are empty
    assert!(entries.is_empty());
}

#[test]
fn provider_absent_from_models_dev_yields_no_models_without_error() {
    // @step Given a models.dev registry that has no "codex" or "google" mapping for codex
    let registry = anthropic_registry();
    // @step And the "codex" provider is treated as credentialed
    let has_creds = true;
    // @step When cloud_model_entries is built for "codex"
    let entries = cloud_model_entries(&registry, "codex", has_creds);
    // @step Then the entries are empty
    assert!(entries.is_empty());
    // @step And no error is raised
    // (cloud_model_entries returns Vec, never panics — reaching here proves it)
}

#[test]
fn model_entry_fields_are_mapped_from_the_models_dev_model() {
    // @step Given a models.dev registry where "anthropic" lists a tool_call model with name "Claude Opus 4", context 200000, reasoning true and image input modality
    let registry = anthropic_registry();
    // @step And the "anthropic" provider is treated as credentialed
    let has_creds = true;
    // @step When cloud_model_entries is built for "anthropic"
    let entries = cloud_model_entries(&registry, "anthropic", has_creds);
    let opus = entries
        .iter()
        .find(|e| e.id == "claude-opus-4")
        .expect("claude-opus-4 present");
    // @step Then the entry display_name is "Claude Opus 4"
    assert_eq!(opus.display_name, "Claude Opus 4");
    // @step And the entry context_window is 200000
    assert_eq!(opus.context_window, 200000);
    // @step And the entry supports_reasoning is true
    assert!(opus.supports_reasoning);
    // @step And the entry supports_vision is true
    assert!(opus.supports_vision);
}

#[test]
fn list_providers_wires_cloud_model_catalog_into_the_model_selector() {
    // @step Given the handle_impl.rs source for SessionManager::list_providers
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest.join("src").join("handle_impl.rs");
    // @step When the source is inspected
    let src = std::fs::read_to_string(&path).expect("read handle_impl.rs");
    // @step Then it references crate::cloud_models::cloud_model_entries
    assert!(
        src.contains("cloud_models::cloud_model_entries"),
        "list_providers must call cloud_model_entries to populate built-in models"
    );
    // @step And it references crate::cloud_models::provider_has_credentials
    assert!(
        src.contains("cloud_models::provider_has_credentials"),
        "list_providers must credential-gate cloud-model population"
    );
    // @step And it no longer leaves built-in providers with unconditionally empty models
    assert!(
        src.contains("build_cloud_registry"),
        "list_providers must build the models.dev registry to fill built-in models"
    );
}
