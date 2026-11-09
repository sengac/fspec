// =========================================================================
// BUG-136: set_model_direct stores provider-prefixed model string
// Feature: spec/features/set-model-direct-stores-provider-prefixed-model-string.feature
// =========================================================================

/// Scenario: Custom model with slashes in model_id round-trips through
/// the AgentManager handler capture (i.e. through `selected_model_string`)
/// without losing the provider prefix.
///
/// This is the regression scenario for the original Fireworks AI
/// failure. `AgentManager::register_handler` captures
/// `selected_model_string()` and forwards it to
/// `create_session_with_id()`, which then splits at the first `/`.
/// Before BUG-136, `set_model_direct` stored only the bare model id so
/// `create_session_with_id` mis-interpreted the first slash segment
/// ("accounts") as the provider name.
#[test]
fn test_bug136_agent_manager_round_trips_slashed_model_id() {
    // @step Given a provider manager has been configured via set_model_direct with provider "openai" and model id "accounts/fireworks/models/kimi-k2-06-instruct"
    let mut manager = test_manager(ProviderType::OpenAI);
    manager
        .set_model_direct(
            "openai",
            "accounts/fireworks/models/kimi-k2-06-instruct",
            Some(131_072),
            Some(4_096),
            None,
        )
        .expect("set_model_direct should succeed");

    // @step When AgentManager registers its spawn handler using selected_model_string
    let captured = manager.selected_model_string();

    // @step Then the captured model string is "openai/accounts/fireworks/models/kimi-k2-06-instruct"
    assert_eq!(
        captured.as_deref(),
        Some("openai/accounts/fireworks/models/kimi-k2-06-instruct"),
        "selected_model_string must include the provider prefix so \
             create_session_with_id parses it as provider='openai' + \
             model='accounts/fireworks/...' rather than mis-splitting at \
             the first '/' and treating 'accounts' as the provider name"
    );

    // @step And passing that captured model string to create_session_with_id resolves the provider as "openai"
    let captured = captured.expect("captured string is Some");
    let (resolved_provider, resolved_model) = captured
        .split_once('/')
        .expect("captured string contains at least one '/'");
    assert_eq!(resolved_provider, "openai");
    assert_eq!(
        resolved_model,
        "accounts/fireworks/models/kimi-k2-06-instruct"
    );

    // @step And no "Unknown provider: 'accounts'" error is raised
    // Implicit: if the BUG-136 regression returned — bare model id
    // stored — `split_once('/')` above would have produced
    // ("accounts", "fireworks/...") and `create_session_with_id` would
    // have tried to resolve "accounts" as a provider, producing the
    // original "Unknown provider: 'accounts'" failure.
    assert_ne!(
        resolved_provider, "accounts",
        "BUG-136 regression: model string mis-split and 'accounts' \
             would reach create_session_with_id as the provider name"
    );
}

/// Scenario: set_model_direct stores the full provider-prefixed model string
///
/// Uses the `openai` provider slug because that is the code path
/// Fireworks AI / OpenRouter-style configurations hit in practice
/// (they're served via OpenAI-compatible endpoints and `set_model_direct`
/// with `provider_id = "openai"`). The point of this test is the
/// `selected_model_string()` / `selected_model_id()` round-trip, not
/// custom-provider registration.
#[test]
fn test_bug136_set_model_direct_stores_provider_prefixed_model_string() {
    // @step Given a fresh provider manager
    let mut manager = test_manager(ProviderType::OpenAI);

    // @step When set_model_direct is called with provider "openai" and model id "llama-3.1-70b"
    manager
        .set_model_direct("openai", "llama-3.1-70b", None, None, None)
        .expect("set_model_direct should succeed");

    // @step Then selected_model_string returns "openai/llama-3.1-70b"
    assert_eq!(
        manager.selected_model_string().as_deref(),
        Some("openai/llama-3.1-70b")
    );

    // @step And selected_model_id returns "llama-3.1-70b"
    assert_eq!(
        manager.selected_model_id().as_deref(),
        Some("llama-3.1-70b")
    );
}

/// Scenario: selected_model_id preserves slashes inside the model id
///
/// Regression guard for the original Fireworks AI / OpenRouter failure:
/// a model id that itself contains slashes must be handed verbatim to
/// `get_openai()` / `get_claude()` etc., with only the single
/// `{provider}/` prefix trimmed off.
#[test]
fn test_bug136_selected_model_id_preserves_internal_slashes() {
    // @step Given a fresh provider manager
    let mut manager = test_manager(ProviderType::OpenAI);

    // @step When set_model_direct is called with provider "openai" and model id "accounts/fireworks/models/kimi-k2-06-instruct"
    manager
        .set_model_direct(
            "openai",
            "accounts/fireworks/models/kimi-k2-06-instruct",
            None,
            None,
            None,
        )
        .expect("set_model_direct should succeed");

    // @step Then selected_model_string returns "openai/accounts/fireworks/models/kimi-k2-06-instruct"
    assert_eq!(
        manager.selected_model_string().as_deref(),
        Some("openai/accounts/fireworks/models/kimi-k2-06-instruct")
    );

    // @step And selected_model_id returns "accounts/fireworks/models/kimi-k2-06-instruct"
    assert_eq!(
        manager.selected_model_id().as_deref(),
        Some("accounts/fireworks/models/kimi-k2-06-instruct")
    );
}

/// Scenario: Codex models continue to work after the fix
#[test]
fn test_bug136_codex_models_unaffected() {
    // @step Given a fresh provider manager
    let mut manager = test_manager(ProviderType::Codex);

    // @step When set_model_direct is called with provider "codex" and model id "gpt-5-codex"
    manager
        .set_model_direct("codex", "gpt-5-codex", None, None, None)
        .expect("set_model_direct should succeed");

    // @step Then selected_model_string returns "codex/gpt-5-codex"
    assert_eq!(
        manager.selected_model_string().as_deref(),
        Some("codex/gpt-5-codex")
    );

    // @step And selected_model_id returns "gpt-5-codex"
    assert_eq!(manager.selected_model_id().as_deref(), Some("gpt-5-codex"));
}

/// Scenario: with_provider_and_model also emits a registry-format composite
///
/// The compaction and DeepSearch paths re-hydrate a ProviderManager via
/// `with_provider_and_model` using the parent's bare model id. This test
/// confirms those managers also produce a registry-format string — any
/// path that later feeds the value back into AgentManager /
/// create_session_with_id must stay consistent with the `select_model`
/// and `set_model_direct` formats.
#[test]
#[serial_test::serial]
fn test_bug136_with_provider_and_model_emits_composite() {
    // @step Given valid "anthropic" credentials
    std::env::set_var("ANTHROPIC_API_KEY", "bug-136-test-key");

    // @step When with_provider_and_model is called with provider "claude" and model id "claude-opus-4-6"
    let manager = ProviderManager::with_provider_and_model(
        "claude",
        Some("claude-opus-4-6"),
        Some(200_000),
        Some(8_192),
    )
    .expect("with_provider_and_model should succeed with ANTHROPIC_API_KEY");

    // @step Then selected_model_string returns "claude/claude-opus-4-6"
    assert_eq!(
        manager.selected_model_string().as_deref(),
        Some("claude/claude-opus-4-6")
    );

    // @step And selected_model_id returns "claude-opus-4-6"
    assert_eq!(
        manager.selected_model_id().as_deref(),
        Some("claude-opus-4-6")
    );
}
