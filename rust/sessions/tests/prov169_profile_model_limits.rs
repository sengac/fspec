//! PROV-169: local-server profile models report only declared context windows.
//!
//! Feature: spec/features/profile-model-context-window.feature

use codelet_sessions::profile_sections::{merge_profile_models, CustomModelDef};

fn custom(id: &str, context_window: Option<u32>) -> Result<CustomModelDef, serde_json::Error> {
    let mut value = serde_json::json!({ "id": id });
    if let Some(context_window) = context_window {
        value["contextWindow"] = serde_json::json!(context_window);
    }
    serde_json::from_value(value)
}

#[test]
fn a_discovered_model_of_a_profile_without_a_declared_context_window_has_none() {
    // @step Given a local-server profile that declares no context window
    let profile_context_window = None;

    // @step When its discovered model "qwen3" is merged into the provider section
    let entries = merge_profile_models(vec!["qwen3".into()], &[], profile_context_window);

    // @step Then the model entry "qwen3" has no context window
    assert_eq!(entries.len(), 1);
    assert_eq!(
        (entries[0].id.as_str(), entries[0].context_window),
        ("qwen3", None)
    );
}

#[test]
fn a_profiles_declared_context_window_applies_to_its_discovered_models() {
    // @step Given a local-server profile that declares a context window of 32768
    let profile_context_window = Some(32_768);

    // @step When its discovered model "qwen3" is merged into the provider section
    let entries = merge_profile_models(vec!["qwen3".into()], &[], profile_context_window);

    // @step Then the model entry "qwen3" has a context window of 32768
    assert_eq!(entries[0].context_window, Some(32_768));
}

#[test]
fn a_custom_models_own_context_window_wins_over_the_profiles() -> Result<(), serde_json::Error> {
    // @step Given a local-server profile that declares no context window
    let profile_context_window = None;

    // @step And a custom model "big" that declares a context window of 131072
    let models = [custom("big", Some(131_072))?];

    // @step When the custom model is merged into the provider section
    let entries = merge_profile_models(Vec::new(), &models, profile_context_window);

    // @step Then the model entry "big" has a context window of 131072
    assert_eq!(
        (entries[0].id.as_str(), entries[0].context_window),
        ("big", Some(131_072))
    );
    Ok(())
}
