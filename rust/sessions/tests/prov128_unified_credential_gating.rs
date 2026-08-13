//! PROV-128 — unify credential gating between section availability and model
//! population (TS parity).
//!
//! Feature: spec/features/unified-credential-gating.feature
//!
//! Rust split its credential decision in two: the section-header `available`
//! flag (`custom::list_providers_info` → `ProviderCredentials::detect` →
//! `has_openai`/`has_codex`/`has_github_copilot`) DID honour OAuth files, but the
//! model-population gate (`cloud_models::provider_has_credentials` →
//! `resolve_credential`) honoured OAuth only for `anthropic`. Result: OAuth-only
//! providers (codex, github-copilot) could show a credentialed header with ZERO
//! models. TS derives a SINGLE `hasCredentials` per section
//! (`cloudSectionBuilder.ts`) used for both keep-the-section and populate-it.
//!
//! These tests seed isolated OAuth credential files (`CODEX_HOME`, `FSPEC_HOME`)
//! and clear the API-key env vars so the OAuth-only path is exercised
//! deterministically and offline. Each asserts the population gate decision
//! EQUALS the header-availability flag — the single-source-of-truth contract.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::sync::Mutex;

use codelet_providers::ProviderCredentials;
use codelet_sessions::cloud_models::provider_has_credentials;

/// Serializes tests that mutate process-global env vars (`CODEX_HOME`,
/// `FSPEC_HOME`, and the API-key vars) so a parallel test in this binary cannot
/// observe another test's seeded credential state.
static ENV_GUARD: Mutex<()> = Mutex::new(());

/// The API-key environment variables that must be cleared so only the seeded
/// OAuth files decide credential presence.
const API_KEY_VARS: &[&str] = &[
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "COPILOT_API_KEY",
    "GITHUB_COPILOT_API_KEY",
];

/// Clear every API-key env var so the OAuth-only path is exercised.
fn clear_api_keys() {
    for var in API_KEY_VARS {
        std::env::remove_var(var);
    }
}

/// Seed a Codex OAuth `auth.json` (OAuth tokens, no cached API key) under an
/// isolated `CODEX_HOME`. Mirrors what the Codex device-auth flow writes.
fn seed_codex_oauth() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp CODEX_HOME");
    let auth = r#"{
        "tokens": {
            "id_token": "id-tok",
            "access_token": "access-tok",
            "refresh_token": "refresh-tok",
            "account_id": "acct-123"
        }
    }"#;
    fs::write(dir.path().join("auth.json"), auth).expect("write codex auth.json");
    std::env::set_var("CODEX_HOME", dir.path());
    dir
}

/// Seed a GitHub Copilot OAuth `copilot_auth.json` under an isolated
/// `FSPEC_HOME` (`$FSPEC_HOME` is used verbatim as the credentials dir).
fn seed_copilot_oauth() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp FSPEC_HOME");
    fs::write(
        dir.path().join("copilot_auth.json"),
        r#"{"github_oauth_token":"gho_prov128_test"}"#,
    )
    .expect("write copilot_auth.json");
    std::env::set_var("FSPEC_HOME", dir.path());
    dir
}

/// Seed a Claude OAuth `claude_auth.json` under an isolated `FSPEC_HOME`.
fn seed_claude_oauth() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp FSPEC_HOME");
    fs::write(
        dir.path().join("claude_auth.json"),
        r#"{"access_token":"claude-access","refresh_token":"claude-refresh","expires":9999999999999}"#,
    )
    .expect("write claude_auth.json");
    std::env::set_var("FSPEC_HOME", dir.path());
    dir
}

/// Point `CODEX_HOME` and `FSPEC_HOME` at empty temp dirs so no OAuth file of
/// any kind resolves (the negative case).
fn seed_empty_homes() -> (tempfile::TempDir, tempfile::TempDir) {
    let codex = tempfile::tempdir().expect("create empty CODEX_HOME");
    let fspec = tempfile::tempdir().expect("create empty FSPEC_HOME");
    std::env::set_var("CODEX_HOME", codex.path());
    std::env::set_var("FSPEC_HOME", fspec.path());
    (codex, fspec)
}

/// The header-availability flag as computed by `custom::list_providers_info`
/// (same `ProviderCredentials::detect()` source), for the given provider.
fn header_available(provider_id: &str) -> bool {
    let creds = ProviderCredentials::detect();
    match provider_id {
        "openai" => creds.has_openai(),
        "anthropic" => creds.has_claude(),
        "codex" => creds.has_codex(),
        "github-copilot" => creds.has_github_copilot(),
        _ => false,
    }
}

// =============================================================================
// Scenario: Codex OAuth alone satisfies the model-population gate
// =============================================================================
#[test]
fn codex_oauth_alone_satisfies_the_gate() {
    let _guard = ENV_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given a Codex OAuth credential file is present
    let _codex = seed_codex_oauth();
    // isolate FSPEC_HOME (no copilot/claude auth) — must NOT disturb CODEX_HOME
    let _fspec_empty = tempfile::tempdir().expect("empty fspec home");
    std::env::set_var("FSPEC_HOME", _fspec_empty.path());

    // @step And no OPENAI_API_KEY is set in the environment
    clear_api_keys();

    // @step When the model-population gate is evaluated for the "codex" provider
    let gate = provider_has_credentials("codex");

    // @step Then the gate reports "codex" as credentialed
    assert!(
        gate,
        "PROV-128: with a Codex OAuth file and no OPENAI_API_KEY, the population gate must report codex as credentialed",
    );

    // @step And the gate decision equals the header availability flag for "codex"
    assert_eq!(
        gate,
        header_available("codex"),
        "PROV-128: population gate and header-availability flag must agree for codex (single source of truth)",
    );
}

// =============================================================================
// Scenario: GitHub Copilot OAuth alone satisfies the model-population gate
// =============================================================================
#[test]
fn github_copilot_oauth_alone_satisfies_the_gate() {
    let _guard = ENV_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given a GitHub Copilot OAuth token file is present
    let _copilot = seed_copilot_oauth();
    // isolate CODEX_HOME so no stray codex auth resolves
    let _codex_empty = tempfile::tempdir().expect("empty codex home");
    std::env::set_var("CODEX_HOME", _codex_empty.path());

    // @step And no Copilot API key is set in the environment
    clear_api_keys();

    // @step When the model-population gate is evaluated for the "github-copilot" provider
    let gate = provider_has_credentials("github-copilot");

    // @step Then the gate reports "github-copilot" as credentialed
    assert!(
        gate,
        "PROV-128: with a GitHub Copilot OAuth token and no API key, the population gate must report github-copilot as credentialed",
    );

    // @step And the gate decision equals the header availability flag for "github-copilot"
    assert_eq!(
        gate,
        header_available("github-copilot"),
        "PROV-128: population gate and header-availability flag must agree for github-copilot (single source of truth)",
    );
}

// =============================================================================
// Scenario: No credentials means both the gate and the header agree the
// provider is uncredentialed
// =============================================================================
#[test]
fn no_credentials_gate_and_header_agree_uncredentialed() {
    let _guard = ENV_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given no credential of any kind exists for "codex" or "github-copilot"
    let (_codex, _fspec) = seed_empty_homes();
    clear_api_keys();

    // @step When the model-population gate is evaluated for each provider
    // @step Then the gate reports each provider as uncredentialed
    // @step And the header availability flag reports each provider as uncredentialed
    for provider in ["codex", "github-copilot"] {
        let gate = provider_has_credentials(provider);
        assert!(
            !gate,
            "PROV-128: with no credentials, the population gate must report {provider} as uncredentialed",
        );
        assert_eq!(
            gate,
            header_available(provider),
            "PROV-128: population gate and header-availability flag must agree for {provider} when uncredentialed",
        );
    }
}

// =============================================================================
// Scenario: Claude OAuth continues to satisfy the anthropic gate
// =============================================================================
#[test]
fn claude_oauth_continues_to_satisfy_the_anthropic_gate() {
    let _guard = ENV_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given a Claude OAuth credential file is present
    let _claude = seed_claude_oauth();
    let _codex_empty = tempfile::tempdir().expect("empty codex home");
    std::env::set_var("CODEX_HOME", _codex_empty.path());

    // @step And no ANTHROPIC_API_KEY is set in the environment
    clear_api_keys();

    // @step When the model-population gate is evaluated for the "anthropic" provider
    let gate = provider_has_credentials("anthropic");

    // @step Then the gate reports "anthropic" as credentialed
    assert!(
        gate,
        "PROV-128: with a Claude OAuth file and no ANTHROPIC_API_KEY, the population gate must report anthropic as credentialed",
    );

    // @step And the gate decision equals the header availability flag for "anthropic"
    assert_eq!(
        gate,
        header_available("anthropic"),
        "PROV-128: population gate and header-availability flag must agree for anthropic (single source of truth)",
    );
}
