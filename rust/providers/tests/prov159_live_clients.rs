//! Feature: spec/features/codex-refreshing-client.feature
//! Primary feature: spec/features/prov159-live-clients.feature

#[path = "prov159_support/network.rs"]
mod network;
mod prov159_support;
#[path = "prov159_support/socket.rs"]
mod socket;

use codelet_providers::codex::codex_auth::delete_codex_auth;
use codelet_providers::codex::refreshing_client::RefreshingCodexClient;
use codelet_providers::codex::CodexProvider;
use futures::StreamExt;
use network::{bounded, client, send, RefreshServer};
use prov159_support::{Fixture, TestResult};
use rig::http_client::{HttpClientExt, LazyBody, MultipartForm};
use serial_test::serial;
use wiremock::matchers::{header, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn send_http(client: &RefreshingCodexClient, url: &str, transport: &str) -> TestResult {
    let request = http::Request::builder().method("POST").uri(url);
    match transport {
        "multipart" => {
            let _: http::Response<LazyBody<Vec<u8>>> = client
                .send_multipart(request.body(MultipartForm::new().text("input", "payload-A"))?)
                .await?;
        }
        "sse" => {
            let mut response = client
                .send_streaming(request.body(b"payload-A".to_vec())?)
                .await?;
            while let Some(chunk) = response.body_mut().next().await {
                chunk?;
            }
        }
        _ => {
            let _: http::Response<LazyBody<Vec<u8>>> =
                client.send(request.body(b"payload-A".to_vec())?).await?;
        }
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a4_existing_clients_cannot_retain_logged_out_account() -> TestResult {
    // @step Given existing Codex HTTP and WebSocket clients have authenticated requests and continuation state for account A
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    let api = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer access-A"))
        .and(header("chatgpt-account-id", "A"))
        .and(header("originator", "rust"))
        .respond_with(ResponseTemplate::new(200))
        .expect(3)
        .mount(&api)
        .await;
    let cached = client(&api.uri(), 3600);
    let socket = socket::SocketProbe::start().await?;
    for transport in ["http", "multipart", "sse"] {
        bounded(send_http(&cached, &api.uri(), transport)).await??;
    }
    let requests = api
        .received_requests()
        .await
        .ok_or("HTTP capture unavailable")?;
    assert_eq!(requests.len(), 3);
    for request in requests {
        assert!(String::from_utf8_lossy(&request.body).contains("payload-A"));
    }
    bounded(socket.send()).await??;
    {
        let headers = socket
            .headers
            .lock()
            .expect("header capture lock is available");
        assert_eq!(headers[0]["authorization"], "Bearer access-A");
        assert_eq!(headers[0]["chatgpt-account-id"], "A");
        assert_eq!(headers[0]["originator"], "rust");
        let messages = socket
            .messages
            .lock()
            .expect("message capture lock is available");
        assert_eq!(messages[0]["type"], "response.create");
        assert!(messages[0].get("previous_response_id").is_none());
    }
    // @step When the user logs out and completes login as account B before another request
    delete_codex_auth()?;
    // @step And requests between logout and the new login fail without sending account A credentials
    for transport in ["http", "multipart", "sse"] {
        assert!(bounded(send_http(&cached, &api.uri(), transport))
            .await?
            .is_err());
    }
    assert!(bounded(socket.send()).await?.is_err());
    fixture.seed("B")?;
    // @step Then no subsequent HTTP request or WebSocket message uses account A credentials
    // @step And each existing client deliberately rebinds to account B or returns a clear reauthentication error
    for transport in ["http", "multipart", "sse"] {
        let error = bounded(send_http(&cached, &api.uri(), transport))
            .await?
            .expect_err("an existing client must not retain account A after account B logged in");
        assert!(error.to_string().contains("sign in again"));
    }
    assert!(bounded(socket.send()).await?.is_err());
    assert_eq!(
        socket.requests(),
        1,
        "account A socket received a request after logout"
    );
    // @step And account A connections, continuation identifiers and routing state are not reused for account B
    fixture.seed("A")?;
    bounded(socket.send_reauthenticated()).await??;
    {
        let headers = socket
            .headers
            .lock()
            .expect("header capture lock is available");
        assert_eq!(headers.len(), 2, "reauthentication reused a revoked socket");
        assert_ne!(headers[0]["session_id"], headers[1]["session_id"]);
        assert!(headers[1].get("x-codex-turn-state").is_none());
        let messages = socket
            .messages
            .lock()
            .expect("message capture lock is available");
        assert!(
            messages[1].get("previous_response_id").is_none(),
            "reauthentication reused the revoked baseline"
        );
        assert_ne!(
            messages[0]["prompt_cache_key"],
            messages[1]["prompt_cache_key"]
        );
    }
    api.verify().await;

    let socket = socket::SocketProbe::with_handshake(true).await?;
    let (sent, transitioned) = tokio::join!(bounded(socket.send()), async {
        bounded(socket.started.notified()).await?;
        delete_codex_auth()?;
        fixture.seed("B")?;
        socket.release.add_permits(1);
        Ok::<_, Box<dyn std::error::Error>>(())
    });
    transitioned?;
    assert!(
        sent?.is_err(),
        "a prepared account A request crossed the handshake transition"
    );
    assert_eq!(
        socket.requests(),
        0,
        "revoked account A sent a WebSocket message"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a14_disk_credentials_follow_buffered_freshness() -> TestResult {
    use base64::Engine;
    let fixture = Fixture::new()?;
    let previous = std::env::var_os("CODEX_MODEL");
    std::env::set_var("CODEX_MODEL", "gpt-5.1-codex");
    let result = async {
        // @step Given shared Codex credentials have fresh, near-expiry, expired, missing or malformed expiry metadata
        for (seconds, expected_expired) in [(Some(3600), false), (Some(15), true), (Some(-15), true), (None, true)] {
            fixture.seed("A")?;
            let path = fixture.directory.path().join("auth.json");
            let mut document: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
            let claims = seconds.map(|seconds| serde_json::json!({"exp": chrono::Utc::now().timestamp() + seconds}))
                .unwrap_or_else(|| serde_json::json!({"exp": "malformed"}));
            let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims.to_string());
            document["tokens"]["access_token"] = format!("e30.{payload}.signature").into();
            document["last_refresh"] = "malformed".into();
            std::fs::write(&path, document.to_string())?;
            // @step When independent providers load those credentials and prepare requests
            for _ in 0..2 {
                let provider = CodexProvider::new()?;
                // @step Then fresh credentials outside the 30-second safety window are reused without contacting the issuer
                // @step And missing or malformed expiry follows a documented conservative freshness fallback
                // @step And unverified token claims are not used as proof of authorization
                assert_eq!(provider.client().http_client().is_token_expired().await, expected_expired,
                    "disk-loaded expiry must honor JWT exp with a 30-second buffer; malformed metadata must refresh conservatively");
            }
        }
        fixture.seed("A")?;
        let issuer = RefreshServer::start().await?;
        let api = MockServer::start().await;
        Mock::given(method("POST")).respond_with(ResponseTemplate::new(200)).mount(&api).await;
        let first = client(&issuer.uri, 0);
        let url = api.uri();
        // @step And near-expiry and expired credentials are refreshed through the shared coordination policy
        let (sent, released) = tokio::join!(send(&first, &url), async {
            bounded(issuer.started.notified()).await?;
            issuer.release.add_permits(1);
            Ok::<_, tokio::time::error::Elapsed>(())
        });
        sent?;
        released?;
        // @step And provider construction does not cause independent clients to refresh the same credentials repeatedly
        for _ in 0..2 {
            let provider = CodexProvider::new()?;
            assert!(!provider.client().http_client().is_token_expired().await,
                "a fresh persisted refresh must survive provider reconstruction even for opaque access tokens");
        }
        assert_eq!(issuer.requests().len(), 1);
        Ok::<_, Box<dyn std::error::Error>>(())
    }.await;
    match previous {
        Some(previous) => std::env::set_var("CODEX_MODEL", previous),
        None => std::env::remove_var("CODEX_MODEL"),
    }
    result
}
