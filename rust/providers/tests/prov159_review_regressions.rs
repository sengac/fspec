//! Feature: spec/features/prov159-review-regressions.feature

#[path = "prov159_support/network.rs"]
pub mod network;
pub mod prov159_support;
#[path = "prov159_support/socket.rs"]
pub mod socket;

use codelet_providers::codex::codex_auth::{clear_codex_oauth, read_codex_auth, write_codex_auth};
use network::{bounded, client, send};
use prov159_support::{auth, Fixture, TestResult};
use serial_test::serial;
use wiremock::{matchers::header, Mock, MockServer, ResponseTemplate};

#[cfg(target_os = "macos")]
#[test]
#[serial]
fn scenario_null_preferred_api_keys_preserve_fallback_keys() -> TestResult {
    use keyring::credential::CredentialApi;

    for operation in ["logout", "login"] {
        // @step Given preferred OAuth credentials contain a null API key and the fallback has an independent key
        let fixture = Fixture::new()?;
        fixture.seed("A")?;
        let mut preferred = serde_json::to_value(auth("A"))?;
        preferred["OPENAI_API_KEY"] = serde_json::Value::Null;
        fixture.keychain()?.set_password(&preferred.to_string())?;
        // @step When OAuth logout or a replacement login is persisted
        if operation == "logout" {
            clear_codex_oauth()?;
        } else {
            write_codex_auth(&auth("B"))?;
        }
        // @step Then the independent API key and unrelated fields remain available
        let stored = read_codex_auth()?.ok_or("credentials lost")?;
        assert_eq!(stored.openai_api_key.as_deref(), Some("sk-independent"));
        let document: serde_json::Value =
            serde_json::from_slice(&std::fs::read(fixture.directory.path().join("auth.json"))?)?;
        assert_eq!(document["unrelated"]["keep"], true);
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_external_rotation_without_private_metadata() -> TestResult {
    // @step Given an existing client and an upstream same-account rotation without private metadata
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    let issuer = MockServer::start().await;
    let cached = client(&issuer.uri(), 0);
    let mut replacement = auth("A");
    let tokens = replacement
        .tokens
        .as_mut()
        .ok_or("fixture tokens missing")?;
    tokens.access_token = "access-A-external".into();
    tokens.refresh_token = "refresh-A-external".into();
    std::fs::write(
        fixture.directory.path().join("auth.json"),
        serde_json::to_vec(&replacement)?,
    )?;
    let api = MockServer::start().await;
    Mock::given(header("authorization", "Bearer access-A-external"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&api)
        .await;
    // @step When the existing client prepares another request
    bounded(send(&cached, &api.uri())).await??;
    // @step Then the new credentials are reused without refreshing the stale token
    assert!(issuer
        .received_requests()
        .await
        .ok_or("issuer capture unavailable")?
        .is_empty());
    api.verify().await;
    // @step And a completed local logout and same-account login still revoke the old client
    clear_codex_oauth()?;
    write_codex_auth(&replacement)?;
    std::fs::write(
        fixture.directory.path().join("auth.json"),
        serde_json::to_vec(&replacement)?,
    )?;
    assert!(bounded(send(&cached, &api.uri())).await?.is_err());
    assert_eq!(
        api.received_requests()
            .await
            .ok_or("API capture unavailable")?
            .len(),
        1
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_identical_relogin_resets_socket_and_routing() -> TestResult {
    // @step Given a completed socket response for an authenticated account
    let _fixture = Fixture::new()?;
    write_codex_auth(&auth("A"))?;
    let socket = socket::SocketProbe::start().await?;
    bounded(socket.send()).await??;
    // @step When the account logs out and logs back in with identical token strings
    clear_codex_oauth()?;
    write_codex_auth(&auth("A"))?;
    bounded(socket.send_reauthenticated()).await??;
    // @step Then a newly bound client opens a new socket without prior continuation state
    {
        let headers = socket
            .headers
            .lock()
            .map_err(|_| "header capture poisoned")?;
        assert_eq!(headers.len(), 2, "re-login reused the previous connection");
        let messages = socket
            .messages
            .lock()
            .map_err(|_| "message capture poisoned")?;
        assert!(messages[1].get("previous_response_id").is_none());
        // @step And account routing affinity is renewed without an intervening revoked request
        assert_ne!(headers[0]["session_id"], headers[1]["session_id"]);
        assert!(headers[1].get("x-codex-turn-state").is_none());
        assert_ne!(
            messages[0]["prompt_cache_key"],
            messages[1]["prompt_cache_key"]
        );
    }

    let socket = socket::SocketProbe::start().await?;
    bounded(socket.send_unrouted(false)).await??;
    clear_codex_oauth()?;
    write_codex_auth(&auth("A"))?;
    bounded(socket.send_unrouted(true)).await??;
    assert_eq!(
        socket
            .headers
            .lock()
            .map_err(|_| "header capture poisoned")?
            .len(),
        2,
        "identical headers must not permit generation-crossing socket reuse"
    );
    assert!(socket
        .messages
        .lock()
        .map_err(|_| "message capture poisoned")?[1]
        .get("previous_response_id")
        .is_none());
    Ok(())
}
