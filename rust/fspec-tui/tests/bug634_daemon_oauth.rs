//! Feature: spec/features/daemon-oauth-code-and-token-routing.feature

use base64::Engine;
use codelet_fspec_tui::transport::BackendError;
use codelet_fspec_tui::{FspecBackend, WebSocketFspecBackend};
use codelet_test_helpers::{OrFail, TestResult};
use sha2::{Digest, Sha256};

mod common;

#[tokio::test]
async fn headless_login_returns_pkce_and_validates_pasted_state() -> TestResult {
    // @step Given a client connected to a real WebSocket daemon
    let (_dir, service) = common::temp_service()?;
    let (address, server) = common::start_ws_server(service).await?;
    let backend = WebSocketFspecBackend::connect(common::ws_url(address)?)
        .await
        .or_fail("connect")?;
    // @step When Anthropic headless login starts
    let start = backend
        .oauth_headless_start("anthropic".into())
        .await
        .or_fail("headless start")?;
    // @step Then it returns a fresh authorization URL and PKCE verifier
    let url = url::Url::parse(&start.authorize_url).or_fail("authorize URL")?;
    let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(query.get("state"), Some(&start.pkce_verifier));
    assert_eq!(
        query.get("code_challenge_method").map(String::as_str),
        Some("S256")
    );
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(start.pkce_verifier.as_bytes()));
    assert_eq!(query.get("code_challenge"), Some(&challenge));
    assert!(start.pkce_verifier.len() >= 43);
    let next = backend
        .oauth_headless_start("anthropic".into())
        .await
        .or_fail("second start")?;
    assert_ne!(start.pkce_verifier, next.pkce_verifier);
    // @step And completing with a mismatched state returns daemon CSRF validation
    let error = backend
        .oauth_headless_complete("anthropic".into(), "code#wrong".into(), start.pkce_verifier)
        .await
        .err()
        .or_fail("expected invalid state")?;
    assert_eq!(error.to_string(), "CSRF validation failed — state mismatch");
    server.abort();
    Ok(())
}

struct CredentialHome(Option<std::ffi::OsString>);

impl Drop for CredentialHome {
    fn drop(&mut self) {
        match &self.0 {
            Some(value) => std::env::set_var("FSPEC_HOME", value),
            None => std::env::remove_var("FSPEC_HOME"),
        }
    }
}

#[tokio::test]
#[serial_test::serial]
async fn token_reads_and_clearing_use_daemon_credential_storage() -> TestResult {
    // @step Given daemon OAuth credentials in a temporary directory
    let (dir, service) = common::temp_service()?;
    let _home = CredentialHome(std::env::var_os("FSPEC_HOME"));
    std::env::set_var("FSPEC_HOME", dir.path());
    let path = dir.path().join("claude_auth.json");
    std::fs::write(
        &path,
        r#"{"access_token":"test","refresh_token":"test","expires":9999999999999}"#,
    )
    .or_fail("seed credentials")?;
    let (address, server) = common::start_ws_server(service).await?;
    let backend = WebSocketFspecBackend::connect(common::ws_url(address)?)
        .await
        .or_fail("connect")?;
    // @step When the client reads and clears Anthropic tokens
    assert!(backend
        .oauth_get_tokens("anthropic".into())
        .await
        .or_fail("read tokens")?);
    backend
        .oauth_clear_tokens("anthropic".into())
        .await
        .or_fail("clear tokens")?;
    // @step Then the persisted tokens are removed and subsequent reads report absent
    assert!(!path.exists());
    assert!(!backend
        .oauth_get_tokens("anthropic".into())
        .await
        .or_fail("read cleared tokens")?);
    // @step And clearing absent tokens succeeds
    backend
        .oauth_clear_tokens("anthropic".into())
        .await
        .or_fail("idempotent clear")?;
    // @step And malformed credentials return the daemon read error
    std::fs::write(&path, "not json").or_fail("malformed credentials")?;
    let error = backend
        .oauth_get_tokens("anthropic".into())
        .await
        .err()
        .or_fail("expected invalid credentials")?;
    assert!(error.to_string().starts_with("read failed:"), "{error}");
    server.abort();
    Ok(())
}

#[tokio::test]
async fn device_operations_preserve_daemon_validation_errors() -> TestResult {
    // @step Given a client connected to a real WebSocket daemon
    let (_dir, service) = common::temp_service()?;
    let (address, server) = common::start_ws_server(service).await?;
    let backend = WebSocketFspecBackend::connect(common::ws_url(address)?)
        .await
        .or_fail("connect")?;
    // @step When device start and polling request an unsupported provider
    let start = backend
        .oauth_device_start("unsupported-test-provider".into())
        .await
        .err()
        .or_fail("expected unsupported provider")?;
    let poll = backend
        .oauth_device_poll("unsupported-test-provider".into(), "handle".into(), 3)
        .await
        .err()
        .or_fail("expected unsupported provider")?;
    // @step Then both return the daemon unsupported provider error
    assert_eq!(
        start.to_string(),
        "device login unsupported for unsupported-test-provider"
    );
    assert_eq!(poll.to_string(), start.to_string());
    // @step And Copilot polling rejects a malformed device handle
    let poll = backend
        .oauth_device_poll("github-copilot".into(), "malformed".into(), 3)
        .await
        .err()
        .or_fail("expected malformed handle")?;
    assert_eq!(poll.to_string(), "malformed copilot device handle");
    // @step And Copilot start rejects a malformed enterprise URL without network access
    let start = backend
        .oauth_copilot_device_start(Some("[".into()))
        .await
        .err()
        .or_fail("expected invalid URL")?;
    assert!(
        start
            .to_string()
            .starts_with("device start failed: Failed to connect to device code endpoint:"),
        "{start}"
    );
    server.abort();
    Ok(())
}

#[tokio::test]
async fn disconnected_oauth_operations_fail_explicitly() -> TestResult {
    // @step Given a WebSocket client without an active connection
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .or_fail("reserve port")?;
    let address = listener.local_addr().or_fail("address")?;
    drop(listener);
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let backend = WebSocketFspecBackend::connect_with_supervisor(common::ws_url(address)?, sender)
        .await
        .or_fail("disconnected backend")?;
    // @step When it requests code device or token operations
    let results = [
        backend
            .oauth_headless_start("anthropic".into())
            .await
            .map(|_| ()),
        backend
            .oauth_headless_complete("anthropic".into(), "code".into(), "verifier".into())
            .await,
        backend.oauth_device_start("codex".into()).await.map(|_| ()),
        backend
            .oauth_device_poll("codex".into(), "handle".into(), 3)
            .await,
        backend.oauth_copilot_device_start(None).await.map(|_| ()),
        backend
            .oauth_get_tokens("anthropic".into())
            .await
            .map(|_| ()),
        backend.oauth_clear_tokens("anthropic".into()).await,
    ];
    // @step Then every operation returns a disconnected error
    for result in results {
        let error = result.err().or_fail("expected disconnected operation")?;
        assert!(
            matches!(
                error.downcast_ref::<BackendError>(),
                Some(BackendError::Disconnected)
            ),
            "{error}"
        );
    }
    Ok(())
}
