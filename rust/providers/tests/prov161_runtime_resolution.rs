use codelet_providers::{resolve_runtime_options, ProviderManager, ProviderRuntimeOptions};
use serde_json::json;
use serial_test::serial;

#[path = "prov161/support.rs"]
mod support;
use support::{Fixture, TestResult};

fn assert_limits(manager: &ProviderManager, expected: (usize, usize)) {
    assert_eq!(
        (manager.context_window(), manager.max_output_tokens()),
        expected
    );
}

/// Feature: spec/features/provider-runtime-settings-resolution.feature
#[test]
fn scenario_runtime_settings_inherit_field_by_field() -> TestResult {
    // @step Given provider defaults and a selected profile with omitted fields and explicit false values
    let config = json!({"providers":{"claude":{
        "settings":{"streaming":true,"autoContinue":5,"preserveThinking":true,"maxImages":4,
            "loopDetectionEnabled":true,"loopDetectionWindow":160,"loopDetectionMaxRepeats":10,
            "loopDetectionMaxRetries":10,"compactionThreshold":{"type":"percentage","value":75}},
        "profiles":{"careful":{"streaming":false,"maxImages":0,"loopDetectionEnabled":false,
            "loopDetectionMaxRetries":0,"customModels":[{"id":"api/model-v1",
                "autoContinue":2,"preserveThinking":false,"loopDetectionMaxRepeats":3}]}}
    }}});
    // @step When runtime settings are resolved for a selected model
    let options: ProviderRuntimeOptions =
        resolve_runtime_options(&config, "claude", Some("careful"), Some("api/model-v1"))?;
    // @step Then model overrides precede profile overrides and provider defaults without losing explicit false
    assert_eq!(options.streaming, Some(false));
    assert_eq!(options.auto_continue, Some(2));
    assert_eq!(options.preserve_thinking, Some(false));
    assert_eq!(options.max_images, Some(0));
    assert_eq!(options.loop_detection_enabled, Some(false));
    assert_eq!(options.loop_detection_window, Some(160));
    assert_eq!(options.loop_detection_max_repeats, Some(3));
    assert_eq!(options.loop_detection_max_retries, Some(0));
    let serialized = serde_json::to_value(&options)?;
    assert_eq!(
        serialized["compactionThreshold"],
        json!({"type":"percentage","value":75})
    );
    assert_eq!(serialized["preserveThinking"], false);
    assert_eq!(
        serde_json::from_value::<ProviderRuntimeOptions>(serialized)?,
        options
    );
    let unprofiled = resolve_runtime_options(&config, "anthropic", None, Some("api/model-v1"))?;
    assert_eq!(unprofiled.auto_continue, Some(5));
    assert_eq!(unprofiled.streaming, Some(true));
    Ok(())
}

/// Feature: spec/features/provider-runtime-settings-resolution.feature
#[tokio::test]
#[serial]
async fn scenario_catalog_token_defaults_apply_to_every_selection_path() -> TestResult {
    // @step Given matching models.dev context and output limits with no explicit overrides
    let _fixture = Fixture::new(&json!({}))?;
    let mut manager = ProviderManager::with_model_support().await?;
    // @step When ordinary direct and profile-qualified models are selected
    for provider in ["openai", "google", "z-ai"] {
        manager.select_model(&format!("{provider}/catalog-key"))?;
        assert_limits(&manager, (1_000_000, 64_000));
        for profile in [None, Some("careful")] {
            manager.set_model_direct_with_profile(
                provider,
                "api/model-v1",
                profile,
                None,
                None,
                None,
            )?;
            // @step Then all selections use the same catalog context and output limits
            assert_limits(&manager, (1_000_000, 64_000));
            assert_eq!(manager.selected_model_id().as_deref(), Some("api/model-v1"));
        }
    }
    for profile in [None, Some("careful")] {
        manager.set_model_direct_with_profile(
            "codex",
            "api/model-v1",
            profile,
            None,
            None,
            None,
        )?;
        assert_limits(&manager, (1_000_000, 64_000));
    }
    manager.set_model_direct("openai", "ambiguous", None, None, None)?;
    assert_limits(&manager, (128_000, 4_096));
    manager.set_model_direct("openai", "unknown/private/model", None, None, None)?;
    assert_eq!(
        manager.selected_model_id().as_deref(),
        Some("unknown/private/model")
    );
    assert_limits(&manager, (128_000, 4_096));
    let mut synchronous = ProviderManager::with_provider("openai")?;
    synchronous.set_model_direct("openai", "api/model-v1", None, None, None)?;
    assert_limits(&synchronous, (1_000_000, 64_000));
    let reconstructed =
        ProviderManager::with_provider_and_model("openai", Some("api/model-v1"), None, None)?;
    assert_limits(&reconstructed, (1_000_000, 64_000));
    Ok(())
}

/// Feature: spec/features/provider-runtime-settings-resolution.feature
#[tokio::test]
#[serial]
async fn scenario_logical_providers_retain_settings_ownership_across_facades() -> TestResult {
    // @step Given Codex and a custom provider use another provider protocol or catalog alias
    let mut fixture = Fixture::new(&json!({"providers":{
        "openai":{"settings":{"autoContinue":99,"maxImages":99}},
        "codex":{"settings":{"autoContinue":2,"maxImages":1,"streaming":false}},
        "private":{"settings":{"autoContinue":3,"maxImages":0}}
    }}))?;
    let mut manager = ProviderManager::with_model_support().await?;
    // @step When their model limits and runtime options are resolved
    manager.set_model_direct("codex", "api/model-v1", None, None, None)?;
    assert!(!manager.should_send_max_output_tokens());
    let codex = manager.runtime_options().clone();
    let codex_limits = (manager.context_window(), manager.max_output_tokens());
    manager.set_model_direct("private", "smart", None, None, Some("openai".into()))?;
    // @step Then catalog aliases supply metadata without borrowing another logical provider settings
    assert_eq!(codex.auto_continue, Some(2));
    assert_eq!(codex.max_images, Some(1));
    assert_eq!(codex_limits, (1_000_000, 64_000));
    assert_eq!(manager.runtime_options().auto_continue, Some(3));
    assert_eq!(manager.runtime_options().max_images, Some(0));
    assert_eq!(manager.runtime_options().streaming, None);
    assert_eq!(manager.selected_model_id().as_deref(), Some("api/model-v1"));
    assert_eq!(manager.facade_override(), Some("openai"));
    assert!(manager.should_send_max_output_tokens());
    assert_limits(&manager, (1_000_000, 64_000));
    manager.set_facade_override(Some("codex".into()));
    assert!(!manager.should_send_max_output_tokens());
    assert_limits(&manager, (1_000_000, 64_000));
    let reconstructed =
        ProviderManager::with_provider_and_model("openai", Some("api/model-v1"), None, None)?;
    assert_eq!(reconstructed.runtime_options().auto_continue, Some(99));
    fixture.retain_openai_credentials();
    let mut unambiguous = ProviderManager::new()?;
    assert_eq!(unambiguous.runtime_options().auto_continue, Some(99));
    unambiguous.set_model_direct("openai", "api/model-v1", None, None, None)?;
    assert_limits(&unambiguous, (1_000_000, 64_000));
    assert_eq!(unambiguous.runtime_options().auto_continue, Some(99));
    Ok(())
}

/// Feature: spec/features/provider-runtime-settings-resolution.feature
#[tokio::test]
#[serial]
async fn scenario_token_overrides_resolve_independently_and_clear_to_automatic() -> TestResult {
    // @step Given provider profile and model token overrides with matching catalog metadata
    let mut config = json!({"providers":{"openai":{
        "settings":{"contextWindow":900_000,"maxOutputTokens":60_000},
        "profiles":{"careful":{"contextWindow":800_000,"maxOutputTokens":50_000,
            "customModels":[{"id":"api/model-v1","contextWindow":700_000,"maxOutputTokens":40_000}]}}
    }}});
    let fixture = Fixture::new(&config)?;
    let mut manager = ProviderManager::with_model_support().await?;
    // @step When each override is applied and independently cleared
    manager.set_model_direct_with_profile(
        "openai",
        "api/model-v1",
        Some("careful"),
        None,
        None,
        None,
    )?;
    manager.override_model_limits(Some(600_000), None);
    assert_limits(&manager, (600_000, 40_000));
    manager.set_model_direct_with_profile(
        "openai",
        "api/model-v1",
        Some("careful"),
        None,
        None,
        None,
    )?;
    manager.override_model_limits(None, Some(30_000));
    assert_limits(&manager, (700_000, 30_000));
    let reconstructed = ProviderManager::with_provider_and_model(
        "openai",
        Some("api/model-v1"),
        Some(600_000),
        Some(30_000),
    )?;
    assert_limits(&reconstructed, (600_000, 30_000));
    // @step Then each limit follows model profile provider catalog and backend fallback precedence independently
    for (path, field, context, output) in [
        (
            "/providers/openai/profiles/careful/customModels/0",
            "contextWindow",
            800_000,
            40_000,
        ),
        (
            "/providers/openai/profiles/careful/customModels/0",
            "maxOutputTokens",
            800_000,
            50_000,
        ),
        (
            "/providers/openai/profiles/careful",
            "contextWindow",
            900_000,
            50_000,
        ),
        (
            "/providers/openai/profiles/careful",
            "maxOutputTokens",
            900_000,
            60_000,
        ),
        (
            "/providers/openai/settings",
            "contextWindow",
            1_000_000,
            60_000,
        ),
        (
            "/providers/openai/settings",
            "maxOutputTokens",
            1_000_000,
            64_000,
        ),
    ] {
        let object = config
            .pointer_mut(path)
            .and_then(serde_json::Value::as_object_mut)
            .ok_or_else(|| std::io::Error::other("missing override layer"))?;
        object.remove(field);
        fixture.save(&config)?;
        manager.set_model_direct_with_profile(
            "openai",
            "api/model-v1",
            Some("careful"),
            None,
            None,
            None,
        )?;
        assert_limits(&manager, (context, output));
    }
    manager.set_model_direct_with_profile(
        "openai",
        "unknown",
        Some("careful"),
        Some(400_000),
        None,
        None,
    )?;
    assert_limits(&manager, (400_000, 4_096));
    manager.set_model_direct_with_profile(
        "openai",
        "unknown",
        Some("careful"),
        None,
        Some(20_000),
        None,
    )?;
    assert_limits(&manager, (128_000, 20_000));
    manager.set_model_direct("openai", "unknown", None, None, None)?;
    assert_eq!(
        (manager.context_window(), manager.max_output_tokens()),
        (128_000, 4_096)
    );
    Ok(())
}

/// Feature: spec/features/provider-runtime-settings-resolution.feature
#[tokio::test]
#[serial]
async fn scenario_anthropic_catalog_limits_are_not_capped_by_historical_defaults() -> TestResult {
    // @step Given an Anthropic model with catalog context and output limits above historical defaults
    let _fixture = Fixture::new(&json!({}))?;
    let mut manager = ProviderManager::with_model_support().await?;
    // @step When the model is selected without explicit overrides
    manager.select_model("anthropic/catalog-key")?;
    // @step Then the resolved limits retain catalog values rather than blanket 200000 and 8192 ceilings
    assert_eq!(manager.context_window(), 1_000_000);
    assert_eq!(manager.max_output_tokens(), 64_000);
    assert_eq!(manager.raw_model_context_window(), Some(1_000_000));
    assert_eq!(manager.raw_model_max_output_tokens(), Some(64_000));
    manager.set_model_direct_with_profile(
        "claude",
        "api/model-v1",
        Some("careful"),
        None,
        None,
        None,
    )?;
    assert_limits(&manager, (1_000_000, 64_000));
    manager.set_model_direct("claude", "private-claude", None, None, None)?;
    assert_limits(&manager, (200_000, 8_192));
    Ok(())
}
