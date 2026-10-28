//! Feature: spec/features/codex-oauth-login.feature
//! Primary feature: spec/features/prov159-login-lifecycle.feature
//! A11's primary embedded-backend test is in prov159_rpc_lifecycle.rs.

#[path = "prov159_support/network.rs"]
pub mod network;
pub mod prov159_support;

use codelet_providers::codex::codex_auth::{delete_codex_auth, read_codex_auth, write_codex_auth};
use codelet_providers::codex::codex_device_auth::{device_auth_login, DeviceAuthConfig};
use codelet_providers::codex::codex_oauth_server::{browser_oauth_login_inner, OAuthServerConfig};
use network::{bounded, RefreshServer};
use prov159_support::{Fixture, TestResult};
use serial_test::serial;
use tokio::net::TcpListener;

fn config(listener: TcpListener, issuer: String, state: &str) -> OAuthServerConfig {
    OAuthServerConfig {
        listener,
        issuer_url: issuer,
        open_browser: false,
        timeout_ms: 5000,
        pkce: None,
        state: Some(state.to_owned()),
    }
}

fn response() -> String {
    use base64::Engine;
    let payload =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"chatgpt_account_id":"B"}"#);
    serde_json::json!({"id_token": format!("e30.{payload}.signature"),
        "access_token": "access-B", "refresh_token": "refresh-B", "expires_in": 3600})
    .to_string()
}

#[tokio::test]
#[serial]
async fn scenario_a10_cancel_stops_pending_authentication() -> TestResult {
    // @step Given a Codex login attempt is waiting for a browser callback, polling a device code, exchanging tokens or awaiting persistence
    // @step When the user cancels the attempt, logs out or starts a replacement login before releasing the pending operation
    // @step Then the cancelled attempt does not persist credentials or report successful login
    // @step And its callback listener, polling tasks and pending exchange work are stopped and cleaned up
    // @step And cancelling an older attempt cannot cancel its replacement
    // @step And UI generation guards also discard late results from the cancelled attempt
    let fixture = Fixture::new()?;
    for device in [false, true] {
        fixture.seed("A")?;
        let issuer = RefreshServer::with_body(response()).await?;
        let device_issuer = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::path(
            "/api/accounts/deviceauth/usercode",
        ))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "device_auth_id": "device", "user_code": "code", "interval": 1
            })),
        )
        .mount(&device_issuer)
        .await;
        wiremock::Mock::given(wiremock::matchers::path("/api/accounts/deviceauth/token"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "authorization_code": "code", "code_verifier": "verifier"
                })),
            )
            .mount(&device_issuer)
            .await;
        wiremock::Mock::given(wiremock::matchers::path("/oauth/token"))
            .respond_with(
                wiremock::ResponseTemplate::new(307)
                    .insert_header("location", format!("{}/oauth/token", issuer.uri)),
            )
            .mount(&device_issuer)
            .await;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let pending = if device {
            tokio::spawn(device_auth_login(DeviceAuthConfig {
                issuer_url: device_issuer.uri(),
                timeout_ms: 5000,
                poll_interval_override_ms: Some(1),
                slow_down_increment_override_ms: None,
                display_fn: Some(Box::new(|_, _| ())),
            }))
        } else {
            tokio::spawn(browser_oauth_login_inner(config(
                listener,
                issuer.uri.clone(),
                "attempt",
            )))
        };
        if !device {
            reqwest::get(format!(
                "http://{address}/auth/callback?code=code&state=attempt"
            ))
            .await?;
        }
        bounded(issuer.started.notified()).await?;
        delete_codex_auth()?;
        issuer.release.add_permits(1);
        let result = bounded(pending).await??;
        assert!(
            result.is_err(),
            "cancelled authentication returned false success"
        );
        assert!(
            read_codex_auth()?.and_then(|auth| auth.tokens).is_none(),
            "cancelled authentication persisted late tokens"
        );
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a11_cancel_allows_immediate_callback_port_reuse() -> TestResult {
    let _fixture = Fixture::new()?;
    let issuer = RefreshServer::with_body(response()).await?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let pending = tokio::spawn(browser_oauth_login_inner(config(
        listener,
        issuer.uri.clone(),
        "old",
    )));
    reqwest::get(format!("http://{address}/probe")).await?;
    delete_codex_auth()?;
    tokio::task::yield_now().await;
    let replacement = TcpListener::bind(address).await;
    pending.abort();
    let _ = pending.await;
    assert!(
        replacement.is_ok(),
        "local cancellation left the callback port occupied"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a17_local_logout_does_not_require_browser_cookie_logout() -> TestResult {
    // @step Given Codex account A is logged in locally and the browser retains its OpenAI SSO session
    // @step When the user logs out locally and completes a new browser authorization using the retained SSO session
    // @step Then the new login may legitimately select the same account without another password prompt
    // @step And the new local login supersedes the old credential generation and cancelled attempts
    // @step And old local credentials and account-scoped client state cannot authorize new requests after logout
    // @step And local logout does not claim to clear browser cookies, force an account chooser or revoke remote sessions
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    let issuer = RefreshServer::with_body(response()).await?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let mut auth = read_codex_auth()?.ok_or("local account missing")?;
    auth.tokens = None;
    write_codex_auth(&auth)?;
    let pending = tokio::spawn(browser_oauth_login_inner(config(
        listener,
        issuer.uri.clone(),
        "sso",
    )));
    reqwest::Client::new()
        .get(format!(
            "http://{address}/auth/callback?code=sso-code&state=sso"
        ))
        .header("cookie", "retained-sso=opaque")
        .send()
        .await?;
    bounded(issuer.started.notified()).await?;
    issuer.release.add_permits(1);
    assert_eq!(bounded(pending).await???.account_id, "B");
    assert_eq!(
        read_codex_auth()?
            .and_then(|auth| auth.tokens)
            .ok_or("SSO login missing")?
            .account_id,
        "B"
    );
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture.directory.path().join("auth.json"))?)?;
    assert_eq!(document["OPENAI_API_KEY"], "sk-independent");
    assert_eq!(document["unrelated"]["keep"], true);
    Ok(())
}
