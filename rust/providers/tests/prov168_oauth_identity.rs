//! Feature: spec/features/current-anthropic-oauth-protocol.feature

#[path = "support/prov168.rs"]
mod support;

use codelet_providers::claude::ClaudeProvider;
use codelet_providers::claude_oauth::build_oauth_headers;
use proptest::prelude::*;
use support::{oauth, refreshing, Environment, TestResult};

#[test]
#[serial_test::serial]
fn oauth_identity_follows_the_installed_release() -> TestResult {
    // @step Given Claude Code is installed with a supported release
    let environment = Environment::new()?;
    environment.install("2.1.316")?;
    let first = oauth()?;

    // @step When OAuth identity is resolved after the installed release changes
    environment.install("2.1.317")?;
    let second = oauth()?;

    // @step Then the advertised version changes without rebuilding fspec
    assert_eq!(
        first.client().headers()["user-agent"],
        "claude-cli/2.1.316 (external, cli)"
    );
    assert_eq!(
        second.client().headers()["user-agent"],
        "claude-cli/2.1.317 (external, cli)"
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn explicit_deployment_version_is_used_consistently() -> TestResult {
    // @step Given an explicit supported Claude Code deployment version is configured
    let _environment = Environment::new()?;
    std::env::set_var("FSPEC_CLAUDE_CODE_VERSION", "2.2.100");

    // @step When both OAuth providers and the reference headers are constructed
    let providers = [oauth()?, refreshing()?];
    let headers = build_oauth_headers("sk-ant-oat01-prov168", None)?;

    // @step Then all three use the configured identity without a bundled version
    let expected = "claude-cli/2.2.100 (external, cli)";
    for provider in providers {
        assert_eq!(provider.client().headers()["user-agent"], expected);
    }
    assert_eq!(
        headers.get("user-agent").map(String::as_str),
        Some(expected)
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn unavailable_or_invalid_versions_fail_closed() -> TestResult {
    // @step Given the authoritative version source is unavailable malformed or older than the supported baseline
    let environment = Environment::new()?;
    for version in [
        None,
        Some(""),
        Some("garbage"),
        Some("2.1.3"),
        Some("2.1.316\r\nx-app: evil"),
        Some("2.1.316.1"),
        Some("2.1.316-beta"),
        Some("2.01.316"),
        Some("18446744073709551616.1.316"),
    ] {
        match version {
            Some(version) => std::env::set_var("FSPEC_CLAUDE_CODE_VERSION", version),
            None => std::env::remove_var("FSPEC_CLAUDE_CODE_VERSION"),
        }

        // @step When an OAuth provider is constructed
        for result in [oauth(), refreshing()] {
            // @step Then construction fails with installation or update guidance and no fallback
            let Err(error) = result else {
                return Err("invalid or unavailable versions must fail closed".into());
            };
            let message = error.to_string();
            assert!(message.contains("claude update"), "{message}");
            assert!(message.contains("FSPEC_CLAUDE_CODE_VERSION"), "{message}");
            assert!(!error.is_retryable());
        }
        assert!(build_oauth_headers("sk-ant-oat01-prov168", None).is_err());
    }
    std::env::remove_var("FSPEC_CLAUDE_CODE_VERSION");
    for invalid in ["garbage", "2.1.3", "2.1.316-beta"] {
        environment.install(invalid)?;
        assert!(oauth().is_err());
    }
    let mut runner = proptest::test_runner::TestRunner::new(proptest::test_runner::Config {
        cases: 32,
        failure_persistence: None,
        ..Default::default()
    });
    runner
        .run(
            &"[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\r\\n[[:print:]]{0,24}",
            |invalid| {
                std::env::set_var("FSPEC_CLAUDE_CODE_VERSION", invalid);
                prop_assert!(oauth().is_err());
                Ok(())
            },
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
#[serial_test::serial]
fn api_key_authentication_is_independent_of_claude_code() -> TestResult {
    // @step Given Claude Code is unavailable and no deployment version is configured
    let _environment = Environment::new()?;

    // @step When an API key provider is constructed
    let provider =
        ClaudeProvider::from_api_key_with_model("sk-ant-api-prov168", "claude-opus-5-5")?;

    // @step Then construction succeeds without an OAuth identity
    assert!(!provider.is_oauth_mode());
    assert!(!provider.client().headers().contains_key("authorization"));
    assert!(!provider.client().headers().contains_key("x-app"));
    Ok(())
}
