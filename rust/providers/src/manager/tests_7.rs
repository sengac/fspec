// =========================================================================
// BUG-137: AgentManager spawn fails for profile-qualified OpenAI Fireworks
// models (e.g. kimi k2.6). Fix: preserve profile name through
// set_model_direct so selected_model_string() emits
// "provider:profile/model".
// Feature: spec/features/agentmanager-spawn-fails-for-profile-qualified-openai-fireworks-models-e-g-kimi-k2-6.feature
// =========================================================================

/// Scenario: set_model_direct with profile_name emits profile-qualified composite
#[test]
fn test_bug137_set_model_direct_with_profile_emits_profile_composite() {
    // @step Given a ProviderManager is created with model registry support
    let mut manager = test_manager(ProviderType::OpenAI);

    // @step When set_model_direct is called with provider_id "openai", model_id "accounts/fireworks/models/kimi-k2p6", and profile_name "fireworks"
    manager
        .set_model_direct_with_profile(
            "openai",
            "accounts/fireworks/models/kimi-k2p6",
            Some("fireworks"),
            Some(200_000),
            Some(16_384),
            None,
        )
        .expect("set_model_direct_with_profile should succeed");

    // @step Then selected_model_string() returns "openai:fireworks/accounts/fireworks/models/kimi-k2p6"
    assert_eq!(
        manager.selected_model_string().as_deref(),
        Some("openai:fireworks/accounts/fireworks/models/kimi-k2p6"),
        "selected_model_string must include the profile segment so \
             create_session_with_id detects profile format (':' before '/') \
             and routes subordinate through set_model_direct instead of \
             select_model"
    );

    // @step And selected_model_id() returns "accounts/fireworks/models/kimi-k2p6"
    assert_eq!(
        manager.selected_model_id().as_deref(),
        Some("accounts/fireworks/models/kimi-k2p6")
    );
}

/// Scenario: set_model_direct without profile_name emits plain provider/model composite
#[test]
fn test_bug137_set_model_direct_without_profile_emits_plain_composite() {
    // @step Given a ProviderManager is created with model registry support
    let mut manager = test_manager(ProviderType::Codex);

    // @step When set_model_direct is called with provider_id "codex", model_id "gpt-5-codex", and no profile_name
    manager
        .set_model_direct_with_profile("codex", "gpt-5-codex", None, None, None, None)
        .expect("set_model_direct_with_profile should succeed");

    // @step Then selected_model_string() returns "codex/gpt-5-codex"
    let composite = manager.selected_model_string();
    assert_eq!(composite.as_deref(), Some("codex/gpt-5-codex"));

    // @step And the composite contains no colon
    assert!(
        !composite.as_ref().expect("composite").contains(':'),
        "composite must not contain ':' when no profile is set"
    );
}

/// Scenario: Legacy set_model_direct (no profile) emits plain composite
/// Regression guard: backward-compatible 5-arg call path must behave as before.
#[test]
fn test_bug137_legacy_set_model_direct_is_plain_composite() {
    // @step Given a ProviderManager
    let mut manager = test_manager(ProviderType::OpenAI);

    // @step When legacy set_model_direct(5 args) is called
    manager
        .set_model_direct(
            "openai",
            "accounts/fireworks/models/kimi-k2p6",
            None,
            None,
            None,
        )
        .expect("legacy set_model_direct should succeed");

    // @step Then the composite has no profile segment (profile name defaults to None)
    assert_eq!(
        manager.selected_model_string().as_deref(),
        Some("openai/accounts/fireworks/models/kimi-k2p6")
    );
}

/// Scenario: select_model (cloud) does not inject a profile segment
#[test]
#[serial_test::serial]
fn test_bug137_select_model_emits_no_profile_segment() {
    // @step Given a ProviderManager is created with model registry support
    // @step And the ANTHROPIC_API_KEY environment variable is set
    std::env::set_var("ANTHROPIC_API_KEY", "bug-137-test-key");
    let mut manager = test_manager_with_registry_and_credentials();

    // @step When select_model is called with "anthropic/claude-sonnet-4"
    let result = manager.select_model("anthropic/claude-sonnet-4");
    std::env::remove_var("ANTHROPIC_API_KEY");
    assert!(
        result.is_ok(),
        "select_model should succeed: {:?}",
        result.err()
    );

    // @step Then selected_model_string() returns a composite without a colon before the first slash
    let composite = manager.selected_model_string().expect("composite");
    let first_slash = composite.find('/').expect("composite has '/'");
    let first_colon = composite.find(':');
    assert!(
        first_colon.is_none() || first_colon.unwrap() > first_slash,
        "cloud select_model must not inject 'provider:profile/' — got {composite:?}"
    );
}

/// Scenario: AgentManager spawn round-trips profile-qualified Fireworks model
///
/// Exercises the round-trip that originally failed in the screenshot:
/// spawner PM → selected_model_string() → create_session_with_id parser.
/// The subordinate path checks `contains(':') && find(':') < find('/')`
/// to detect profile models. This test verifies the captured string
/// satisfies that invariant.
#[test]
fn test_bug137_agent_manager_round_trips_profile_qualified_model() {
    // @step Given a spawner session whose ProviderManager was configured via set_model_direct with profile_name "fireworks" on provider "openai" and model_id "accounts/fireworks/models/kimi-k2p6"
    let mut manager = test_manager(ProviderType::OpenAI);
    manager
        .set_model_direct_with_profile(
            "openai",
            "accounts/fireworks/models/kimi-k2p6",
            Some("fireworks"),
            Some(200_000),
            Some(16_384),
            None,
        )
        .expect("set_model_direct_with_profile should succeed");

    // @step When AgentManager captures selected_model_string() and passes it to create_session_with_id
    let captured = manager.selected_model_string().expect("captured composite");

    // @step Then the subordinate path detects profile format by finding ':' before '/'
    let colon_idx = captured.find(':');
    let slash_idx = captured.find('/');
    assert!(
        colon_idx.is_some() && slash_idx.is_some(),
        "captured composite {captured:?} must contain both ':' and '/'"
    );
    assert!(
        colon_idx.unwrap() < slash_idx.unwrap(),
        "captured composite {captured:?} must have ':' before '/' so \
             create_session_with_id treats it as a profile model"
    );

    // @step And set_model_direct is used for the subordinate instead of select_model
    // Simulate create_session_with_id parsing:
    let is_profile_model = captured.contains(':') && captured.find(':') < captured.find('/');
    assert!(
        is_profile_model,
        "captured composite must be classified as profile model by \
             session_manager::create_session_with_id"
    );

    // @step And no "Model '...' not found in provider 'openai'" error is raised
    // (implicit — the profile branch skips registry validation)
    let colon = captured.find(':').unwrap();
    let slash = captured.find('/').unwrap();
    let provider = &captured[..colon];
    let profile = &captured[colon + 1..slash];
    let model = &captured[slash + 1..];
    assert_eq!(provider, "openai");
    assert_eq!(profile, "fireworks");
    assert_eq!(model, "accounts/fireworks/models/kimi-k2p6");
}
