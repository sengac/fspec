#[test]
#[serial_test::serial]
fn test_napi_override_takes_priority_over_registry() {
    // @step Given the model registry contains "openai/gpt-4o" with context=128000 and max_output=16384
    std::env::set_var("OPENAI_API_KEY", "fake-key-for-model-005-test");
    let mut manager = test_manager_with_registry_and_credentials();

    // @step When session_set_model is called with context_window=64000 and max_output_tokens=8192
    // First call select_model to store registry values, then override with NAPI params
    let result = manager.select_model("openai/gpt-4o");
    std::env::remove_var("OPENAI_API_KEY");
    assert!(result.is_ok());

    // Verify registry values are stored
    assert_eq!(manager.registry_context_window, Some(128_000));
    assert_eq!(manager.registry_max_output_tokens, Some(16_384));

    // Simulate NAPI override (the NAPI layer calls select_model first,
    // then overwrites with NAPI params if Some)
    manager.override_model_limits(Some(64_000), Some(8_192));

    // @step Then context_window() should return 64000
    assert_eq!(manager.context_window(), Some(64_000));

    // @step And max_output_tokens() should return 8192
    assert_eq!(manager.max_output_tokens(), Some(8_192));
}

// -------------------------------------------------------------------------
// Scenario: Provider-level compile-time constants remain unchanged
// -------------------------------------------------------------------------

#[test]
fn test_provider_constants_remain_unchanged() {
    // @step Then claude::CONTEXT_WINDOW should be 200000
    assert_eq!(claude::CONTEXT_WINDOW, 200_000);

    // @step And gemini::CONTEXT_WINDOW should be 1000000
    assert_eq!(gemini::CONTEXT_WINDOW, 1_000_000);

    // @step And zai::CONTEXT_WINDOW should be 128000
    assert_eq!(zai::CONTEXT_WINDOW, 128_000);

    // @step And copilot::CONTEXT_WINDOW should be 200000
    assert_eq!(copilot::CONTEXT_WINDOW, 200_000);
}

// -------------------------------------------------------------------------
// MODEL-004: Facade Override Tests
// Feature: spec/features/custom-model-registration-and-facade-override-in-model-selector.feature
// -------------------------------------------------------------------------

#[test]
fn test_set_model_direct_stores_facade_override() {
    // @step Given a custom model with facade="codex"
    let mut manager = test_manager(ProviderType::OpenAI);

    // @step When set_model_direct is called with facade_override=Some("codex")
    let result = manager.set_model_direct(
        "openai",
        "my-custom-model",
        Some(32_000),
        Some(4_096),
        Some("codex".to_string()),
    );
    assert!(result.is_ok());

    // @step Then facade_override() should return Some("codex")
    assert_eq!(manager.facade_override(), Some("codex"));
}

#[test]
fn test_set_model_direct_without_facade_leaves_none() {
    // @step Given a profile model without facade override
    let mut manager = test_manager(ProviderType::OpenAI);

    // @step When set_model_direct is called with facade_override=None
    let result = manager.set_model_direct("openai", "my-local-model", None, None, None);
    assert!(result.is_ok());

    // @step Then facade_override() should return None
    assert_eq!(manager.facade_override(), None);
}

#[test]
fn test_set_facade_override_setter() {
    // @step Given a ProviderManager with no facade override
    let mut manager = test_manager(ProviderType::OpenAI);
    assert_eq!(manager.facade_override(), None);

    // @step When set_facade_override is called with Some("gemini")
    manager.set_facade_override(Some("gemini".to_string()));

    // @step Then facade_override() should return Some("gemini")
    assert_eq!(manager.facade_override(), Some("gemini"));

    // @step When set_facade_override is called with None
    manager.set_facade_override(None);

    // @step Then facade_override() should return None
    assert_eq!(manager.facade_override(), None);
}

#[test]
fn test_facade_override_initialized_none_in_all_constructors() {
    // @step Given ProviderManagers created via different constructors
    // test_manager uses direct construction
    let mgr = test_manager(ProviderType::Claude);
    assert_eq!(mgr.facade_override(), None);

    // for_testing constructor
    let mgr = ProviderManager::for_testing(ProviderType::OpenAI, None, None);
    assert_eq!(mgr.facade_override(), None);
}
