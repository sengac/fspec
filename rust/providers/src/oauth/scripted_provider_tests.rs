//! Tests for ScriptedOAuthProvider (PROV-060)
//!
//! Feature: spec/features/shared-oauth-building-blocks.feature
//! Scenario: Scripted OAuth provider executes Rhai flow functions

use crate::oauth::script_provider::{ScriptProviderConfig, ScriptedOAuthProvider};
use rhai::{Dynamic, Map};

fn test_config() -> ScriptProviderConfig {
    ScriptProviderConfig {
        name: "test-provider".to_string(),
        display_name: "Test Provider".to_string(),
        script: "test.rhai".to_string(),
        auth_url: Some("https://auth.example.com/authorize".to_string()),
        token_url: Some("https://auth.example.com/token".to_string()),
        client_id: Some("test-client-id".to_string()),
        redirect_uri: Some("http://localhost:0/callback".to_string()),
        scopes: Some("read write".to_string()),
        flow: Some("authorization_code".to_string()),
        credential_file: Some("test_auth.json".to_string()),
    }
}

const TEST_SCRIPT: &str = r#"
fn build_authorization_request(config) {
    let pkce = oauth::generate_pkce();
    let state = oauth::generate_state();
    #{
        url: config.auth_url + "?client_id=" + config.client_id,
        pkce_verifier: pkce.verifier,
        state: state,
        challenge: pkce.challenge
    }
}

fn exchange_code(config, code, pkce_verifier) {
    #{
        access_token: "at_" + code,
        refresh_token: "rt_" + code,
        expires_in: 3600
    }
}

fn refresh_token(config, current_tokens) {
    #{
        access_token: "refreshed_" + current_tokens.access_token,
        refresh_token: current_tokens.refresh_token,
        expires_in: 3600
    }
}

fn poll_for_token(config, device_data) {
    #{
        status: "success",
        access_token: "device_at",
        refresh_token: "device_rt",
        expires_in: 3600
    }
}

fn needs_refresh(tokens) {
    tokens.expires_at < 100
}
"#;

// @step Given a ScriptedOAuthProvider loaded from a .rhai script defining all five OAuth functions

#[tokio::test]
async fn scripted_provider_build_authorization_request() {
    // @step Given a ScriptedOAuthProvider loaded from a .rhai script defining all five OAuth functions
    let provider = ScriptedOAuthProvider::from_script(TEST_SCRIPT, test_config()).unwrap();

    // @step When build_authorization_request is called
    let result = provider.build_authorization_request().await.unwrap();

    // @step Then it returns an authorization URL with PKCE challenge and state
    let url = result.get("url").unwrap().clone().into_string().unwrap();
    assert!(url.starts_with("https://auth.example.com/authorize"));
    assert!(url.contains("client_id=test-client-id"));

    let verifier = result
        .get("pkce_verifier")
        .unwrap()
        .clone()
        .into_string()
        .unwrap();
    assert!(!verifier.is_empty());

    let state = result.get("state").unwrap().clone().into_string().unwrap();
    assert!(!state.is_empty());

    let challenge = result
        .get("challenge")
        .unwrap()
        .clone()
        .into_string()
        .unwrap();
    assert!(!challenge.is_empty());
}

#[tokio::test]
async fn scripted_provider_exchange_code() {
    let provider = ScriptedOAuthProvider::from_script(TEST_SCRIPT, test_config()).unwrap();

    // @step And exchange_code, refresh_token, poll_for_token, and needs_refresh each execute correctly via spawn_blocking
    let result = provider
        .exchange_code("auth_code_123", "verifier_456")
        .await
        .unwrap();

    let at = result
        .get("access_token")
        .unwrap()
        .clone()
        .into_string()
        .unwrap();
    assert_eq!(at, "at_auth_code_123");

    let rt = result
        .get("refresh_token")
        .unwrap()
        .clone()
        .into_string()
        .unwrap();
    assert_eq!(rt, "rt_auth_code_123");
}

#[tokio::test]
async fn scripted_provider_refresh_token() {
    let provider = ScriptedOAuthProvider::from_script(TEST_SCRIPT, test_config()).unwrap();

    let mut tokens = Map::new();
    tokens.insert("access_token".into(), Dynamic::from("old_at".to_string()));
    tokens.insert("refresh_token".into(), Dynamic::from("old_rt".to_string()));

    // @step And exchange_code, refresh_token, poll_for_token, and needs_refresh each execute correctly via spawn_blocking
    let result = provider.refresh_token(tokens).await.unwrap();

    let at = result
        .get("access_token")
        .unwrap()
        .clone()
        .into_string()
        .unwrap();
    assert_eq!(at, "refreshed_old_at");
}

#[tokio::test]
async fn scripted_provider_poll_for_token() {
    let provider = ScriptedOAuthProvider::from_script(TEST_SCRIPT, test_config()).unwrap();

    let mut device_data = Map::new();
    device_data.insert("device_code".into(), Dynamic::from("dc_123".to_string()));

    // @step And exchange_code, refresh_token, poll_for_token, and needs_refresh each execute correctly via spawn_blocking
    let result = provider.poll_for_token(device_data).await.unwrap();

    let status = result.get("status").unwrap().clone().into_string().unwrap();
    assert_eq!(status, "success");

    let at = result
        .get("access_token")
        .unwrap()
        .clone()
        .into_string()
        .unwrap();
    assert_eq!(at, "device_at");
}

#[tokio::test]
async fn scripted_provider_needs_refresh() {
    let provider = ScriptedOAuthProvider::from_script(TEST_SCRIPT, test_config()).unwrap();

    // @step And exchange_code, refresh_token, poll_for_token, and needs_refresh each execute correctly via spawn_blocking
    let mut tokens_expired = Map::new();
    tokens_expired.insert("expires_at".into(), Dynamic::from(50_i64));
    assert!(provider.needs_refresh(tokens_expired).await.unwrap());

    let mut tokens_valid = Map::new();
    tokens_valid.insert("expires_at".into(), Dynamic::from(200_i64));
    assert!(!provider.needs_refresh(tokens_valid).await.unwrap());
}

#[test]
fn scripted_provider_rejects_invalid_script() {
    let bad_script = "fn broken( { }";
    let result = ScriptedOAuthProvider::from_script(bad_script, test_config());
    assert!(result.is_err());
}
