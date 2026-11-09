//! PROV-172: limits too large for the wire are rejected, never clamped.
//!
//! Feature: spec/features/custom-provider-napi-model-limits.feature

use super::*;

// =========================================================================
// Scenario: A declared limit larger than the wire maximum fails the provider listing
// =========================================================================
#[test]
#[serial]
fn a_declared_limit_larger_than_the_wire_maximum_fails_the_provider_listing() {
    // @step Given a custom provider "acme" whose model "opus-4.7" declares context_window 5000000000
    let fx = DiscoveryFixture::new();
    let _api_key = EnvGuard::set("ANTHROPIC_API_KEY", "sk-not-real");
    write_claude_rhai_config(
        fx.project_root(),
        "acme",
        Some(5_000_000_000),
        Some(8192),
        None,
        None,
        None,
    );

    // @step When list_providers_info() resolves per-model limits
    let result = list_providers_info();

    // @step Then listing fails with an error naming "models.opus-4.7.context_window" and 5000000000
    let error = result
        .as_ref()
        .err()
        .map(ToString::to_string)
        .unwrap_or_else(|| panic!("listing must fail, got {result:?}"));
    assert!(
        error.contains("models.opus-4.7.context_window") && error.contains("5000000000"),
        "error must name the field and the value: {error}"
    );

    // @step And no model entry reports a context window of 4294967295
    let clamped = result
        .into_iter()
        .flatten()
        .flat_map(|provider| provider.models)
        .any(|model| model.context_window == Some(u32::MAX));
    assert!(
        !clamped,
        "a too-large limit must never be listed as u32::MAX"
    );
}
