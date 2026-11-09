//! BUG-330: Codex declares no static model limits.
//!
//! Feature: spec/features/codex-declares-no-static-model-limits.feature

use codelet_providers::{
    CodexProvider, LlmProvider, ModelLimitsResolver, ProviderManager, ProviderType,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn codex_provider() -> Result<CodexProvider, codelet_providers::ProviderError> {
    CodexProvider::from_api_key("sk-proj-test-key-12345", "gpt-5.1-codex")
}

#[test]
fn the_codex_resolver_declares_no_default_limits() -> TestResult {
    // @step Given the Codex provider's model-limits resolver
    let resolver = codex_provider()?;

    // @step When its default context window and default output limit are read
    let limits = (
        resolver.default_context_window(),
        resolver.default_max_output_tokens(),
    );

    // @step Then both are unavailable
    assert_eq!(limits, (None, None));

    // @step And it does not send max_output_tokens
    assert!(!resolver.should_send_max_output_tokens());
    Ok(())
}

#[test]
fn codex_limits_resolve_as_unavailable_without_catalog_or_user_values() {
    // @step Given a Codex ProviderManager with no registry limits and no user override
    let manager = ProviderManager::for_testing(ProviderType::Codex, None, None);

    // @step When its context window and output limit are resolved
    let limits = (manager.context_window(), manager.max_output_tokens());

    // @step Then both are unavailable
    assert_eq!(limits, (None, None));
}

#[test]
fn the_codex_provider_reports_no_guessed_context_window() -> TestResult {
    // @step Given a Codex provider object
    let provider = codex_provider()?;

    // @step When it is asked for its context window and maximum output tokens
    let limits = (
        LlmProvider::context_window(&provider),
        LlmProvider::max_output_tokens(&provider),
    );

    // @step Then both are unavailable rather than a guessed 272000 and 4096
    assert_eq!(limits, (None, None));
    Ok(())
}

#[test]
fn a_provider_object_with_declared_limits_reports_them() -> TestResult {
    // @step Given a Claude provider object
    let provider = codelet_providers::ClaudeProvider::from_api_key_with_model(
        "sk-ant-test-key",
        "claude-sonnet-4",
    )?;

    // @step When it is asked for its context window and maximum output tokens
    let limits = (
        LlmProvider::context_window(&provider),
        LlmProvider::max_output_tokens(&provider),
    );

    // @step Then they are 200000 and 8192
    assert_eq!(limits, (Some(200_000), Some(8_192)));
    Ok(())
}

/// COV-060: rust/tests/ is not built by any Cargo target; it must not keep a
/// test asserting the removed Codex defaults.
#[test]
fn no_unbuilt_test_asserts_the_removed_codex_default_limits() -> TestResult {
    // @step Given the rust/tests directory, which no Cargo target builds
    let unbuilt = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests");

    // @step When its Rust test sources are read
    let mut offenders = Vec::new();
    if unbuilt.is_dir() {
        for entry in std::fs::read_dir(&unbuilt)? {
            let path = entry?.path();
            if path.extension().is_some_and(|extension| extension == "rs") {
                let source = std::fs::read_to_string(&path)?;
                if source.contains("CodexProvider") && source.contains("272_000") {
                    offenders.push(path.display().to_string());
                }
            }
        }
    }

    // @step Then none of them asserts a Codex context window of 272000
    assert!(
        offenders.is_empty(),
        "unbuilt tests still assert the removed Codex default: {offenders:?}"
    );
    Ok(())
}
