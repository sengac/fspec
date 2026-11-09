//! Feature: spec/features/provider-settings-oauth-disconnect.feature

use super::*;
use codelet_fspec_tui::WebSocketFspecBackend;
use codelet_test_helpers::{OrFail, TestResult};

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
async fn oauth_methods_forward_on_embedded_and_websocket() -> TestResult {
    // @step Given the embedded transport is in use
    let (dir, service) = common::temp_service()?;
    let embedded =
        EmbeddedFspecBackend::new(tokio::runtime::Handle::current(), Arc::clone(&service));
    let _home = CredentialHome(std::env::var_os("FSPEC_HOME"));
    std::env::set_var("FSPEC_HOME", dir.path());
    let claude_auth = dir.path().join("claude_auth.json");
    let (address, server) = common::start_ws_server(service).await?;
    let websocket = WebSocketFspecBackend::connect(common::ws_url(address)?)
        .await
        .or_fail("connect")?;
    // @step Then the FspecBackend OAuth methods forward to the napi OAuth functions
    // @step When the websocket transport is in use
    // @step Then the FspecBackend OAuth token methods read and clear daemon credentials
    for backend in [&embedded as &dyn FspecBackend, &websocket] {
        std::fs::write(
            &claude_auth,
            r#"{"access_token":"at","refresh_token":"rt","expires":9999999999999}"#,
        )
        .or_fail("seed tokens")?;
        assert!(backend
            .oauth_get_tokens("anthropic".into())
            .await
            .or_fail("read tokens")?);
        backend
            .oauth_clear_tokens("anthropic".into())
            .await
            .or_fail("clear tokens")?;
        assert!(!claude_auth.exists());
        assert!(!backend
            .oauth_get_tokens("anthropic".into())
            .await
            .or_fail("cleared tokens")?);
        backend
            .oauth_clear_tokens("anthropic".into())
            .await
            .or_fail("idempotent clear")?;
    }
    server.abort();
    Ok(())
}
