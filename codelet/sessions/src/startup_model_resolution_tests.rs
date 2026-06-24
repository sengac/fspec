//! PROV-120 resolver unit tests.
//!
//! Feature: spec/features/startup-model-initialization-first-available.feature
//!
//! All fixtures are built in-memory (no network, no filesystem). Sections
//! are passed in `list_providers` DISPLAY order (cloud/custom first, then
//! profiles appended) so the tests prove the resolver reorders for
//! SELECTION on its own.

use super::{resolve_startup_model, ResolvedStartupModel};
use codelet_rpc_types::{ModelEntry, ProviderInfo};

/// Build a plain (non-custom) `ModelEntry` with the given id.
fn model(id: &str) -> ModelEntry {
    ModelEntry {
        id: id.to_string(),
        display_name: id.to_string(),
        context_window: 128_000,
        supports_reasoning: false,
        supports_vision: false,
        is_custom: false,
    }
}

/// Build a cloud / built-in provider section (`profile_name = None`,
/// reachable). Zero models models an uncredentialed cloud provider (the
/// approved "has >=1 model" credential proxy).
fn cloud(key: &str, models: Vec<ModelEntry>) -> ProviderInfo {
    ProviderInfo {
        key: key.to_string(),
        display_name: key.to_string(),
        models,
        profile_name: None,
        is_unreachable: false,
    }
}

/// Build a local-server profile section keyed `"{provider}:{profile}"`.
fn profile(provider: &str, name: &str, models: Vec<ModelEntry>, unreachable: bool) -> ProviderInfo {
    ProviderInfo {
        key: format!("{provider}:{name}"),
        display_name: format!("{provider}: {name}"),
        models,
        profile_name: Some(name.to_string()),
        is_unreachable: unreachable,
    }
}

fn resolved(s: &str) -> ResolvedStartupModel {
    ResolvedStartupModel {
        model_string: s.to_string(),
    }
}

/// Scenario: Fresh launch with no persisted model selects the first available reachable cloud model
#[test]
fn fresh_launch_selects_first_available_cloud_model() {
    // @step Given no persisted model is recorded
    let persisted: Option<&str> = None;
    // @step And exactly one cloud provider section is reachable and credentialed with at least one model
    let sections = vec![cloud("anthropic", vec![model("claude-opus-4")])];
    // @step And no other reachable model-bearing sections exist
    // (only the single anthropic section is present)

    // @step When startup model initialization runs
    let result = resolve_startup_model(&sections, persisted);

    // @step Then the default model resolves to that cloud provider's first model
    assert_eq!(result, Some(resolved("anthropic/claude-opus-4")));
    // @step And the resolved model is committed before the bootstrap session is created
    // @step And the bootstrap create_session succeeds without opening the model selector
    // (commit/create_session ordering is asserted in the bootstrap wiring task; here a
    //  non-None resolution is the precondition that makes the commit possible.)
    assert!(result.is_some());
}

/// Scenario: Persisted model that still matches a credentialed section is restored exactly
#[test]
fn persisted_model_is_restored_exactly() {
    // @step Given a persisted model is recorded
    let persisted = Some("anthropic/claude-opus-4");
    // @step And the persisted model matches a reachable credentialed section that still contains that model
    // @step And another reachable model-bearing section exists earlier in build order
    // (the openai profile sorts BEFORE cloud for selection and would be first-available)
    let sections = vec![
        cloud("anthropic", vec![model("claude-opus-4")]),
        profile("openai", "work", vec![model("Qwen3-80B")], false),
    ];

    // @step When startup model initialization runs
    let result = resolve_startup_model(&sections, persisted);

    // @step Then the default model resolves to the persisted model
    assert_eq!(result, Some(resolved("anthropic/claude-opus-4")));
    // @step And the default model is not the first-available model from the earlier section
    assert_ne!(result, Some(resolved("openai:work/Qwen3-80B")));
    // @step And the bootstrap create_session succeeds without opening the model selector
    assert!(result.is_some());
}

/// Scenario: Persisted model whose provider lost credentials falls back to first-available
#[test]
fn persisted_model_lost_credentials_falls_back_to_first_available() {
    // @step Given a persisted model is recorded
    let persisted = Some("openai:gone/missing-model");
    // @step And the persisted model's provider no longer has credentials or the model no longer exists
    // (no section keyed openai:gone is present)
    // @step And a different reachable credentialed section with at least one model exists
    let sections = vec![cloud("anthropic", vec![model("claude-opus-4")])];

    // @step When startup model initialization runs
    let result = resolve_startup_model(&sections, persisted);

    // @step Then the persisted model is not restored
    assert_ne!(result, Some(resolved("openai:gone/missing-model")));
    // @step And the default model resolves to the first-available reachable model
    assert_eq!(result, Some(resolved("anthropic/claude-opus-4")));
    // @step And the bootstrap create_session succeeds without opening the model selector
    assert!(result.is_some());
}

/// Scenario: Unreachable zero-model local profiles are skipped in favour of a reachable cloud model
#[test]
fn unreachable_zero_model_profiles_are_skipped() {
    // @step Given no persisted model is recorded
    let persisted: Option<&str> = None;
    // @step And two local profile sections are unreachable and contain zero models
    // @step And one cloud provider section is reachable and credentialed with at least one model
    let sections = vec![
        cloud("anthropic", vec![model("claude-opus-4")]),
        profile("openai", "down-1", vec![], true),
        profile("openai", "down-2", vec![], true),
    ];

    // @step When startup model initialization runs
    let result = resolve_startup_model(&sections, persisted);

    // @step Then the unreachable zero-model profile sections are excluded from selection
    // @step And the default model resolves to the cloud provider's first model
    assert_eq!(result, Some(resolved("anthropic/claude-opus-4")));
    // @step And the bootstrap create_session succeeds without opening the model selector
    assert!(result.is_some());
}

/// Scenario: First-available honours build order so a reachable profile precedes cloud
#[test]
fn first_available_honours_build_order_profile_before_cloud() {
    // @step Given no persisted model is recorded
    let persisted: Option<&str> = None;
    // @step And a local profile section is reachable and credentialed with at least one model
    // @step And a cloud provider section is reachable and credentialed with at least one model
    // (passed in DISPLAY order: cloud first, profile appended — resolver must reorder)
    let sections = vec![
        cloud("anthropic", vec![model("claude-opus-4")]),
        profile("openai", "work", vec![model("Qwen3-80B")], false),
    ];

    // @step When startup model initialization runs
    let result = resolve_startup_model(&sections, persisted);

    // @step Then the default model resolves to the profile section's first model
    assert_eq!(result, Some(resolved("openai:work/Qwen3-80B")));
    // @step And the default model is not the cloud provider's model
    assert_ne!(result, Some(resolved("anthropic/claude-opus-4")));
    // @step And the bootstrap create_session succeeds without opening the model selector
    assert!(result.is_some());
}

/// Scenario: Genuinely zero reachable model-bearing sections leaves no default and declines
#[test]
fn zero_model_bearing_sections_resolves_none() {
    // @step Given no persisted model is recorded
    let persisted: Option<&str> = None;
    // @step And all local profile sections are unreachable with zero models
    // @step And no cloud provider section is credentialed
    // @step And no reachable model-bearing section exists
    let sections = vec![
        profile("openai", "down-1", vec![], true),
        profile("openai", "down-2", vec![], true),
    ];

    // @step When startup model initialization runs
    let result = resolve_startup_model(&sections, persisted);

    // @step Then no default model is committed
    assert_eq!(result, None);
    // @step And the bootstrap create_session declines with an empty SessionId
    // (a None resolution means no default is committed -> the PROV-101 decline path)
    assert!(result.is_none());
}

/// Scenario: Anthropic is never substituted as a hardcoded default when not credentialed
#[test]
fn anthropic_is_never_substituted_as_hardcoded_default() {
    // @step Given no persisted model is recorded
    let persisted: Option<&str> = None;
    // @step And anthropic is not credentialed
    // (anthropic present but with zero models == uncredentialed proxy -> excluded)
    // @step And a different reachable credentialed provider section with at least one model exists
    let sections = vec![
        cloud("anthropic", vec![]),
        cloud("openai", vec![model("gpt-4o")]),
    ];

    // @step When startup model initialization runs
    let result = resolve_startup_model(&sections, persisted);

    // @step Then the default model resolves to the reachable credentialed provider's first model
    assert_eq!(result, Some(resolved("openai/gpt-4o")));
    // @step And no anthropic or claude model is substituted as a hardcoded default
    let model_string = result.unwrap().model_string;
    assert!(!model_string.contains("anthropic"));
    assert!(!model_string.contains("claude"));
}

/// Scenario: Ambiguous multi-provider credential state is not silently resolved by provider priority
#[test]
fn ambiguous_multi_provider_with_no_models_resolves_none() {
    // @step Given no persisted model is recorded
    let persisted: Option<&str> = None;
    // @step And multiple providers are credentialed in an ambiguous state with no reachable model-bearing section
    // (multiple cloud providers, each with zero models -> no model-bearing section)
    let sections = vec![cloud("anthropic", vec![]), cloud("openai", vec![])];

    // @step When startup model initialization runs
    let result = resolve_startup_model(&sections, persisted);

    // @step Then no default model is committed by provider-priority preference
    assert_eq!(result, None);
    // @step And the bootstrap create_session declines with an empty SessionId
    assert!(result.is_none());
}
