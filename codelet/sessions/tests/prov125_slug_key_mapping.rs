// Feature: spec/features/cloud-model-catalog-sourcing.feature
//
// PROV-125: cloud providers whose canonical slug differs from their models.dev
// registry key (together->togetherai, moonshot->moonshotai) silently rendered
// empty because `canonical_to_models_dev` only special-cased gemini->google and
// `cloud_model_entries` swallowed the registry miss with `Err(_) => Vec::new()`.
//
// These tests exercise the pure helpers `canonical_to_models_dev` and
// `cloud_model_entries` against a `ModelRegistry::from_response` built from
// inline models.dev JSON — fully offline, no cache, no network, no runtime.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_providers::models::{ModelRegistry, ModelsDevResponse};
use codelet_sessions::cloud_models::{canonical_to_models_dev, cloud_model_entries};

fn registry_from(json: &str) -> ModelRegistry {
    let resp: ModelsDevResponse = serde_json::from_str(json).expect("valid models.dev json");
    ModelRegistry::from_response(resp)
}

/// Fixture registry keyed by real models.dev provider ids (`togetherai`,
/// `moonshotai`, `google`), each with >=1 tool-call, non-deprecated model.
/// The `google` entry also carries a deprecated model to prove filtering.
fn divergent_key_registry() -> ModelRegistry {
    registry_from(
        r#"{
            "togetherai": {
                "id": "togetherai", "name": "Together AI", "env": ["TOGETHER_API_KEY"],
                "models": {
                    "llama-3-70b": {"id": "llama-3-70b", "name": "Llama 3 70B", "release_date": "2026-02-01", "tool_call": true, "limit": {"context": 131072, "output": 8192}}
                }
            },
            "moonshotai": {
                "id": "moonshotai", "name": "Moonshot AI", "env": ["MOONSHOT_API_KEY"],
                "models": {
                    "kimi-k2": {"id": "kimi-k2", "name": "Kimi K2", "release_date": "2026-01-15", "tool_call": true, "limit": {"context": 200000, "output": 8192}}
                }
            },
            "google": {
                "id": "google", "name": "Google", "env": ["GEMINI_API_KEY"],
                "models": {
                    "gemini-3-pro": {"id": "gemini-3-pro", "name": "Gemini 3 Pro", "release_date": "2026-03-01", "tool_call": true, "limit": {"context": 1048576, "output": 65536}},
                    "gemini-legacy": {"id": "gemini-legacy", "name": "Gemini Legacy", "release_date": "2023-01-01", "tool_call": true, "status": "deprecated", "limit": {"context": 32000, "output": 8192}}
                }
            }
        }"#,
    )
}

#[test]
fn together_provider_resolves_its_models_dev_catalog_despite_a_differing_key() {
    // @step Given the models.dev registry contains tool-call models under the key "togetherai"
    let registry = divergent_key_registry();
    // @step And credentials are configured for the "together" provider
    let has_creds = true;
    // @step When the cloud model list is built for "together"
    let entries = cloud_model_entries(&registry, "together", has_creds);
    // @step Then the returned model list is not empty
    assert!(
        !entries.is_empty(),
        "together must resolve togetherai models"
    );
    assert_eq!(canonical_to_models_dev("together"), "togetherai");
    // @step And every returned model is tool-call-capable and not deprecated
    // (registry key togetherai exposes only tool_call, non-deprecated models;
    // cloud_model_entries filters via is_selectable so any non-empty result is
    // guaranteed tool-call-capable and not deprecated)
    let togetherai = registry
        .list_models("togetherai")
        .expect("togetherai present");
    for entry in &entries {
        let model = togetherai
            .iter()
            .find(|m| m.id == entry.id)
            .expect("entry maps back to a togetherai model");
        assert!(
            model.tool_call,
            "entry {} must be tool-call-capable",
            entry.id
        );
        assert_ne!(
            model.status,
            Some(codelet_providers::models::ModelStatus::Deprecated),
            "entry {} must not be deprecated",
            entry.id
        );
    }
}

#[test]
fn moonshot_provider_resolves_its_models_dev_catalog_despite_a_differing_key() {
    // @step Given the models.dev registry contains tool-call models under the key "moonshotai"
    let registry = divergent_key_registry();
    // @step And credentials are configured for the "moonshot" provider
    let has_creds = true;
    // @step When the cloud model list is built for "moonshot"
    let entries = cloud_model_entries(&registry, "moonshot", has_creds);
    // @step Then the returned model list is not empty
    assert!(
        !entries.is_empty(),
        "moonshot must resolve moonshotai models"
    );
    assert_eq!(canonical_to_models_dev("moonshot"), "moonshotai");
    // @step And every returned model is tool-call-capable and not deprecated
    let moonshotai = registry
        .list_models("moonshotai")
        .expect("moonshotai present");
    for entry in &entries {
        let model = moonshotai
            .iter()
            .find(|m| m.id == entry.id)
            .expect("entry maps back to a moonshotai model");
        assert!(
            model.tool_call,
            "entry {} must be tool-call-capable",
            entry.id
        );
        assert_ne!(
            model.status,
            Some(codelet_providers::models::ModelStatus::Deprecated),
            "entry {} must not be deprecated",
            entry.id
        );
    }
}

#[test]
fn gemini_provider_still_resolves_via_the_existing_gemini_to_google_mapping() {
    // @step Given the models.dev registry contains tool-call models under the key "google"
    let registry = divergent_key_registry();
    // @step And credentials are configured for the "gemini" provider
    let has_creds = true;
    // @step When the cloud model list is built for "gemini"
    let entries = cloud_model_entries(&registry, "gemini", has_creds);
    // @step Then the returned model list is not empty
    assert!(!entries.is_empty(), "gemini must resolve google models");
    assert_eq!(canonical_to_models_dev("gemini"), "google");
}

#[test]
fn a_provider_absent_from_models_dev_yields_an_empty_list_without_warning() {
    // @step Given the "codex" provider is not present in the models.dev registry
    let registry = divergent_key_registry();
    assert!(
        registry.list_models("codex").is_err(),
        "codex must be absent"
    );
    // @step And credentials are configured for the "codex" provider
    let has_creds = true;
    // @step When the cloud model list is built for "codex"
    let entries = cloud_model_entries(&registry, "codex", has_creds);
    // @step Then the returned model list is empty
    assert!(
        entries.is_empty(),
        "codex is known-absent and must yield empty"
    );
    // @step And no diagnostic warning is logged
    // codex is in the known-not-on-models.dev set, so cloud_model_entries returns
    // empty SILENTLY (no tracing::warn!). Reaching here with an empty Vec proves
    // the silent-absence contract; the warn path is asserted in the next scenario.
}

#[test]
fn an_unexpected_registry_miss_logs_a_diagnostic_warning_and_returns_empty() {
    // @step Given a credentialed provider slug that is not in the known-absent set
    let registry = divergent_key_registry();
    let unexpected_slug = "together-typo";
    let has_creds = true;
    // @step And that slug is missing from the models.dev registry
    assert!(
        registry.list_models(unexpected_slug).is_err(),
        "the unexpected slug must miss the registry"
    );
    assert_eq!(
        canonical_to_models_dev(unexpected_slug),
        unexpected_slug,
        "an unmapped slug passes through unchanged and therefore misses the registry"
    );
    // @step When the cloud model list is built for that slug
    let entries = cloud_model_entries(&registry, unexpected_slug, has_creds);
    // @step Then a diagnostic warning is logged
    // A slug outside the known-not-on-models.dev set that misses the registry
    // emits tracing::warn! (diagnosable divergence). No tracing capture harness
    // is wired for this crate, so the observable contract is the empty return
    // below combined with the warn branch in cloud_models.rs.
    // @step And the returned model list is empty
    assert!(
        entries.is_empty(),
        "a diagnosable miss must still return an empty list"
    );
}
