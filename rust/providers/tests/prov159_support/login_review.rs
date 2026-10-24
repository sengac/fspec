use super::prov159_support::{Fixture, TestResult};
use base64::Engine;
use codelet_providers::codex::codex_device_auth::DeviceAuthConfig;
use codelet_providers::codex::codex_oauth_server::OAuthServerConfig;
use codelet_providers::codex::oauth_test_support::{configure, OAuthTestConfig, OAuthTestGuard};
use tokio::net::TcpListener;
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

pub const LIFETIME: i64 = 30;

pub fn browser_config(listener: TcpListener, issuer: String) -> OAuthServerConfig {
    OAuthServerConfig {
        listener,
        issuer_url: issuer,
        open_browser: false,
        timeout_ms: 30_000,
        pkce: None,
        state: Some("review".into()),
    }
}

pub fn device_config(issuer: String) -> DeviceAuthConfig {
    DeviceAuthConfig {
        issuer_url: issuer,
        timeout_ms: 30_000,
        poll_interval_override_ms: Some(1),
        slow_down_increment_override_ms: None,
        display_fn: Some(Box::new(|_, _| ())),
    }
}

pub fn configure_split(issuer: &MockServer) -> Result<OAuthTestGuard, Box<dyn std::error::Error>> {
    Ok(configure(OAuthTestConfig {
        issuer: issuer.uri(),
        callback_address: "127.0.0.1:0".parse()?,
        state: "review".into(),
    }))
}

pub async fn issuer(poll_redirect: Option<&str>) -> MockServer {
    let issuer = MockServer::start().await;
    Mock::given(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "device", "user_code": "code", "interval": 1
        })))
        .mount(&issuer)
        .await;
    let poll = match poll_redirect {
        Some(uri) => ResponseTemplate::new(307).insert_header("location", uri),
        None => ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authorization_code": "code", "code_verifier": "verifier"
        })),
    };
    Mock::given(path("/api/accounts/deviceauth/token"))
        .respond_with(poll)
        .mount(&issuer)
        .await;
    let payload =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"chatgpt_account_id":"B"}"#);
    Mock::given(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id_token": format!("e30.{payload}.signature"), "access_token": "opaque-access-B",
            "refresh_token": "refresh-B", "expires_in": LIFETIME
        })))
        .mount(&issuer)
        .await;
    issuer
}

pub fn document(fixture: &Fixture) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(&std::fs::read(
        fixture.directory.path().join("auth.json"),
    )?)?)
}

pub fn seed(fixture: &Fixture) -> TestResult {
    fixture.seed("A")?;
    let mut auth = document(fixture)?;
    auth["fspec_expires_at"] = (chrono::Utc::now().timestamp() + 86_400).into();
    auth["fspec_generation"] = "old-generation".into();
    std::fs::write(
        fixture.directory.path().join("auth.json"),
        serde_json::to_vec(&auth)?,
    )?;
    Ok(())
}

pub fn assert_preserved(auth: &serde_json::Value) {
    assert_eq!(auth["OPENAI_API_KEY"], "sk-independent");
    assert_eq!(auth["unrelated"]["keep"], true);
    assert_ne!(auth["fspec_generation"], "old-generation");
}

pub fn assert_expiry(fixture: &Fixture, before: i64) -> TestResult {
    let auth = document(fixture)?;
    assert_eq!(auth["tokens"]["account_id"], "B");
    assert_eq!(auth["tokens"]["access_token"], "opaque-access-B");
    assert_eq!(auth["tokens"]["refresh_token"], "refresh-B");
    let expiry = auth["fspec_expires_at"]
        .as_i64()
        .ok_or("login discarded the token response lifetime")?;
    assert!((before + LIFETIME..=chrono::Utc::now().timestamp() + LIFETIME).contains(&expiry));
    assert!(auth["last_refresh"].as_str().is_some());
    assert_preserved(&auth);
    Ok(())
}

pub async fn assert_no_exchange(issuer: &MockServer) -> TestResult {
    let requests = issuer
        .received_requests()
        .await
        .ok_or("request recording disabled")?;
    assert!(requests
        .iter()
        .all(|request| request.url.path() != "/oauth/token"));
    Ok(())
}
