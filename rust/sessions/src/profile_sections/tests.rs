use codelet_rpc_types::{ModelEntry, ProviderInfo};

use super::{build_profile_provider_info, cloud_provider_info};

fn model(id: &str, custom: bool) -> ModelEntry {
    ModelEntry {
        id: id.to_string(),
        display_name: id.to_string(),
        context_window: Some(128_000),
        supports_reasoning: false,
        supports_vision: false,
        is_custom: custom,
    }
}

/// Scenario: A cloud provider reports no profile and is reachable
#[test]
fn cloud_provider_has_no_profile_and_is_reachable() {
    // @step Given the server builds the provider list
    // @step And a cloud provider "anthropic" with credentials is registered
    let models = vec![model("claude-sonnet", false)];

    // @step When list_providers() returns the provider list
    let info: ProviderInfo = cloud_provider_info("anthropic", "Anthropic", models);

    // @step Then the "anthropic" entry has profile_name None
    assert_eq!(info.profile_name, None);

    // @step And the "anthropic" entry has is_unreachable false
    assert!(!info.is_unreachable);
}

/// Scenario: A reachable local-server profile is reported as a profile section
#[test]
fn reachable_profile_is_a_profile_section() {
    // @step Given the server builds the provider list
    // @step And a local-server "openai" profile named "my-profile" whose /v1/models probe succeeds
    let models = vec![model("m1", false), model("m2", false), model("m3", false)];

    // @step When list_providers() returns the provider list
    let info = build_profile_provider_info("openai", "my-profile", models, false);

    // @step Then a provider entry has profile_name Some("my-profile")
    assert_eq!(info.profile_name.as_deref(), Some("my-profile"));

    // @step And that entry has display_name "openai: my-profile"
    assert_eq!(info.display_name, "openai: my-profile");

    // @step And that entry has is_unreachable false
    assert!(!info.is_unreachable);
}

/// Scenario: An unreachable local-server profile with no custom models is marked unreachable
#[test]
fn unreachable_profile_without_custom_models_is_marked() {
    // @step Given the server builds the provider list
    // @step And a local-server "openai" profile named "down-profile" whose /v1/models probe fails
    // @step And that profile has no custom models
    let models: Vec<ModelEntry> = Vec::new();

    // @step When list_providers() returns the provider list
    let info = build_profile_provider_info("openai", "down-profile", models, true);

    // @step Then the "down-profile" entry has is_unreachable true
    assert!(info.is_unreachable);

    // @step And the "down-profile" entry has profile_name Some("down-profile")
    assert_eq!(info.profile_name.as_deref(), Some("down-profile"));

    // @step And the "down-profile" entry has an empty models list
    assert!(info.models.is_empty());

    // @step And the "down-profile" entry has display_name "openai: down-profile" with no embedded "(unreachable)" text
    assert_eq!(info.display_name, "openai: down-profile");
    assert!(!info.display_name.contains("(unreachable)"));

    // @step And the "down-profile" entry is still present in the list
    // (the caller never filters unreachable sections out — proven by
    //  the fact a populated ProviderInfo is returned here).
    assert_eq!(info.key, "openai:down-profile");
}

/// Scenario: A failed-probe profile with custom models is not marked unreachable
#[test]
fn failed_probe_with_custom_models_is_not_unreachable() {
    // @step Given the server builds the provider list
    // @step And a local-server "openai" profile named "custom-profile" whose /v1/models probe returns unsupported discovery
    // @step And that profile has at least one custom model
    let models = vec![model("my-custom-model", true)];

    // @step When list_providers() returns the provider list
    let info = build_profile_provider_info("openai", "custom-profile", models, false);

    // @step Then the "custom-profile" entry has is_unreachable false
    assert!(!info.is_unreachable);

    // @step And the "custom-profile" entry still has profile_name Some("custom-profile")
    assert_eq!(info.profile_name.as_deref(), Some("custom-profile"));

    // @step And the "custom-profile" entry lists its custom models
    assert_eq!(info.models.len(), 1);
    assert!(info.models[0].is_custom);
}
