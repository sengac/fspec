//! Feature: spec/features/prov159-login-review.feature

#![cfg(feature = "test-support")]

#[path = "prov159_support/login_review.rs"]
mod login_review;
#[path = "prov159_support/network.rs"]
pub mod network;
pub mod prov159_support;

use codelet_providers::codex::auth_lifecycle::{cancel_login, LoginAttempt};
use codelet_providers::codex::codex_auth::read_codex_auth;
use codelet_providers::codex::codex_device_auth::{
    complete_device_login, device_auth_login, start_device_login,
};
use codelet_providers::codex::codex_oauth_server::browser_oauth_login_inner;
use login_review::*;
use network::{bounded, RefreshServer};
use prov159_support::{tokens, Fixture, TestResult};
use serial_test::serial;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::test]
#[serial]
async fn scenario_browser_login_persists_short_opaque_expiry() -> TestResult {
    // @step Given existing credentials have a stale expiry and unrelated authentication fields
    let fixture = Fixture::new()?;
    seed(&fixture)?;
    let issuer = issuer(None).await;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let before = chrono::Utc::now().timestamp();
    // @step When browser authorization returns an opaque access token with a short lifetime
    let pending = tokio::spawn(browser_oauth_login_inner(browser_config(
        listener,
        issuer.uri(),
    )));
    bounded(reqwest::get(format!(
        "http://{address}/auth/callback?code=code&state=review"
    )))
    .await??;
    assert_eq!(bounded(pending).await???.access_token, "opaque-access-B");
    // @step Then the new tokens and their absolute expiry are persisted together without losing unrelated fields
    assert_expiry(&fixture, before)
}

#[tokio::test]
#[serial]
async fn scenario_direct_device_login_persists_short_opaque_expiry() -> TestResult {
    // @step Given existing credentials have a stale expiry and unrelated authentication fields
    let fixture = Fixture::new()?;
    seed(&fixture)?;
    let issuer = issuer(None).await;
    let before = chrono::Utc::now().timestamp();
    // @step When direct device authorization returns an opaque access token with a short lifetime
    let result = bounded(device_auth_login(device_config(issuer.uri()))).await??;
    assert_eq!(result.access_token, "opaque-access-B");
    // @step Then the new tokens and their absolute expiry are persisted together without losing unrelated fields
    assert_expiry(&fixture, before)
}

#[tokio::test]
#[serial]
async fn scenario_split_device_login_persists_short_opaque_expiry() -> TestResult {
    // @step Given existing credentials have a stale expiry and unrelated authentication fields
    let fixture = Fixture::new()?;
    seed(&fixture)?;
    let issuer = issuer(None).await;
    let _config = configure_split(&issuer)?;
    let before = chrono::Utc::now().timestamp();
    // @step When split device authorization returns an opaque access token with a short lifetime
    let code = bounded(start_device_login()).await??;
    let result = bounded(complete_device_login(&code.device_auth_id)).await??;
    assert_eq!(result.access_token, "opaque-access-B");
    // @step Then the new tokens and their absolute expiry are persisted together without losing unrelated fields
    assert_expiry(&fixture, before)
}

#[tokio::test]
#[serial]
async fn scenario_token_only_commit_discards_stale_expiry() -> TestResult {
    // @step Given existing credentials have a stale expiry and unrelated authentication fields
    let fixture = Fixture::new()?;
    seed(&fixture)?;
    // @step When a caller commits replacement tokens without a lifetime through the public login attempt API
    let attempt = bounded(LoginAttempt::begin()).await??;
    bounded(attempt.commit(&tokens("B"))).await??;
    // @step Then the replacement tokens have no inherited expiry and unrelated fields remain unchanged
    let auth = document(&fixture)?;
    assert_eq!(auth["tokens"]["account_id"], "B");
    assert!(auth.get("fspec_expires_at").is_none());
    assert_preserved(&auth);
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_cancellation_completes_pending_device_poll() -> TestResult {
    let _fixture = Fixture::new()?;
    for split in [false, true] {
        // @step Given a direct or split device login has reached an acknowledged pending poll request
        let poll = RefreshServer::with_body(
            serde_json::json!({"error": "authorization_pending"}).to_string(),
        )
        .await?;
        let issuer = issuer(Some(&format!("{}/pending-poll", poll.uri))).await;
        let _config = configure_split(&issuer)?;
        let pending = if split {
            let code = bounded(start_device_login()).await??;
            tokio::spawn(async move {
                complete_device_login(&code.device_auth_id)
                    .await
                    .map_err(anyhow::Error::from)
            })
        } else {
            tokio::spawn(device_auth_login(device_config(issuer.uri())))
        };
        bounded(poll.started.notified()).await?;
        // @step When the login is cancelled before the issuer releases the poll response
        cancel_login();
        // @step Then the backend returns a cancellation error within a bounded wait without task abortion
        let result = bounded(pending).await??;
        let error = result
            .err()
            .ok_or("cancelled device login reported success")?;
        assert!(error
            .to_string()
            .contains(if split { "changed" } else { "cancelled" }));
        // @step And no credentials are persisted and no subsequent polling or token exchange occurs
        assert!(read_codex_auth()?.and_then(|auth| auth.tokens).is_none());
        poll.release.add_permits(1);
        assert_eq!(poll.requests()?.len(), 1);
        assert_no_exchange(&issuer).await?;
        // @step And a cancelled split device session cannot be completed again
        assert!(bounded(complete_device_login("device")).await?.is_err());
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_cancellation_completes_callback_listener() -> TestResult {
    let _fixture = Fixture::new()?;
    let issuer = issuer(None).await;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let pending = tokio::spawn(browser_oauth_login_inner(browser_config(
        listener,
        issuer.uri(),
    )));
    // @step Given a browser callback listener has acknowledged a probe and has an unfinished connection
    let probe = bounded(reqwest::get(format!("http://{address}/probe"))).await??;
    assert_eq!(probe.status(), reqwest::StatusCode::NOT_FOUND);
    let mut unfinished = bounded(TcpStream::connect(address)).await??;
    unfinished
        .write_all(b"GET /auth/callback HTTP/1.1\r\n")
        .await?;
    // @step When the login is cancelled before any authorization callback arrives
    cancel_login();
    // @step Then the backend returns a cancellation error within a bounded wait without task abortion
    let result = bounded(pending).await??;
    let error = result
        .err()
        .ok_or("cancelled browser login reported success")?;
    assert!(error.to_string().contains("cancelled"));
    // @step And the callback port is immediately reusable and the unfinished connection is closed
    let _replacement = TcpListener::bind(address).await?;
    let mut buffer = [0; 1];
    match bounded(unfinished.read(&mut buffer)).await? {
        Ok(count) => assert_eq!(count, 0),
        Err(error) => assert!(matches!(
            error.kind(),
            std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
        )),
    }
    // @step And no credentials are persisted and no token exchange occurs
    assert!(read_codex_auth()?.and_then(|auth| auth.tokens).is_none());
    assert_no_exchange(&issuer).await
}
