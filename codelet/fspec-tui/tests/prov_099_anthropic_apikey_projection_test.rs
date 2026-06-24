#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/anthropic-oauth-vs-api-key-classification.feature
//!
//! PROV-099 — Projection-layer half of the fix. `project_one` must NOT
//! classify anthropic as OAuth-logged-in merely because it is
//! `configured`: a present env api key (`masked_key = Some`) means an
//! api-key configuration, so `has_oauth_tokens` must be false and no
//! "Logout from OAuth" row is emitted. When anthropic is configured
//! WITHOUT an env api key (`masked_key = None`, i.e. the OAuth auth
//! file), the logout row still appears. Anthropic always exposes BOTH
//! its OAuth login rows AND an api-key row.
//!
//! These are pure projection tests over hand-built
//! `ProviderCredentialInfo` records — no env, no filesystem, no network.

use codelet_fspec_tui::views::provider_settings::projection::project_display_infos;
use codelet_rpc_types::ProviderCredentialInfo;

/// Build a configured anthropic credential with the given masked_key /
/// source, mirroring what the backend produces for the two cases.
fn anthropic_configured(masked_key: Option<&str>, source: Option<&str>) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: "anthropic".to_string(),
        display_name: "Anthropic".to_string(),
        configured: true,
        credential_type: "oauth".to_string(),
        model_count: 4,
        masked_key: masked_key.map(ToString::to_string),
        source: source.map(ToString::to_string),
    }
}

// ============================================================================
// Scenario: Anthropic with an env API key is not classified as OAuth-logged-in
// ============================================================================

#[test]
fn anthropic_with_env_api_key_is_not_oauth_logged_in() {
    // @step Given a configured anthropic credential whose masked_key is Some
    let info = anthropic_configured(Some("sk-ant-••••••••mnop"), Some("env"));

    // @step When project_display_infos projects the credential
    let display = project_display_infos(&[info], &[]);
    let anthropic = display
        .iter()
        .find(|d| d.id == "anthropic")
        .expect("anthropic display info present");

    // @step Then the anthropic display info has_oauth_tokens is false
    assert!(
        !anthropic.has_oauth_tokens,
        "a present env api key must not classify anthropic as OAuth-logged-in"
    );

    // @step And the anthropic display info oauth_status_label is None
    assert!(
        anthropic.oauth_status_label.is_none(),
        "no Logout row when an env api key is present; got {:?}",
        anthropic.oauth_status_label
    );

    // @step And the anthropic display info requires_api_key is true
    assert!(
        anthropic.requires_api_key,
        "anthropic must still offer the api-key row"
    );
}

// ============================================================================
// Scenario: Anthropic configured without an env API key shows the logout row
// ============================================================================

#[test]
fn anthropic_configured_without_env_api_key_shows_logout_row() {
    // @step Given a configured anthropic credential whose masked_key is None
    let info = anthropic_configured(None, None);

    // @step When project_display_infos projects the credential
    let display = project_display_infos(&[info], &[]);
    let anthropic = display
        .iter()
        .find(|d| d.id == "anthropic")
        .expect("anthropic display info present");

    // @step Then the anthropic display info has_oauth_tokens is true
    assert!(
        anthropic.has_oauth_tokens,
        "anthropic configured via OAuth file (no env key) must be OAuth-logged-in"
    );

    // @step And the anthropic display info oauth_status_label is Some("Logout from OAuth [Anthropic]")
    assert_eq!(
        anthropic.oauth_status_label.as_deref(),
        Some("Logout from OAuth [Anthropic]"),
        "anthropic OAuth login must surface the logout row"
    );
}

// ============================================================================
// Scenario: Anthropic offers both OAuth login rows and an api-key row
// ============================================================================

#[test]
fn anthropic_offers_both_oauth_login_rows_and_api_key_row() {
    // @step Given a configured anthropic credential whose masked_key is Some
    let info = anthropic_configured(Some("sk-ant-••••••••mnop"), Some("env"));

    // @step When project_display_infos projects the credential
    let display = project_display_infos(&[info], &[]);
    let anthropic = display
        .iter()
        .find(|d| d.id == "anthropic")
        .expect("anthropic display info present");

    // @step Then the anthropic display info oauth_login_methods is non-empty
    assert!(
        !anthropic.oauth_login_methods.is_empty(),
        "anthropic must always expose OAuth login rows (browser + code)"
    );

    // @step And the anthropic display info requires_api_key is true
    assert!(
        anthropic.requires_api_key,
        "anthropic must always offer the api-key row alongside OAuth login"
    );
}
