//! Feature: spec/features/prov159-refresh-completion.feature

#[path = "prov159_support/network.rs"]
mod network;
mod prov159_support;

use codelet_providers::codex::codex_auth::{delete_codex_auth, write_codex_auth};
use codelet_providers::codex::refreshing_client::RefreshingCodexClient;
use codelet_providers::codex::websocket::CodexWebSocketHandle;
use network::{bounded, client, send, RefreshServer};
use prov159_support::{auth, Fixture, TestResult};
use rig::http_client::HttpClientExt;
use serial_test::serial;
use std::time::Duration;
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn stream(client: RefreshingCodexClient, url: String) -> Result<(), rig::http_client::Error> {
    let request = http::Request::builder()
        .method("POST")
        .uri(url)
        .body(b"{\"input\":[]}".to_vec())
        .map_err(rig::http_client::Error::Protocol)?;
    client.send_streaming(request).await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_deleted_credentials_cannot_construct_an_unfenced_client() -> TestResult {
    // @step Given account A credentials were captured before provider deletion
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    delete_codex_auth()?;
    let issuer = MockServer::start().await;
    let api = MockServer::start().await;
    // @step When stale clients are constructed after deletion and a subsequent login
    let deleted = client(&issuer.uri(), 3600);
    write_codex_auth(&auth("B"))?;
    let replaced = client(&issuer.uri(), 3600);
    // @step Then HTTP streaming and socket requests fail without contacting the issuer or API
    for stale in [deleted, replaced] {
        assert!(bounded(send(&stale, &api.uri())).await?.is_err());
        assert!(bounded(stream(stale.clone(), api.uri())).await?.is_err());
        assert!(bounded(stream(
            stale.with_websocket(CodexWebSocketHandle::default()),
            api.uri(),
        ))
        .await?
        .is_err());
    }
    assert!(api
        .received_requests()
        .await
        .ok_or("API capture unavailable")?
        .is_empty());
    assert!(issuer
        .received_requests()
        .await
        .ok_or("issuer capture unavailable")?
        .is_empty());
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_refresh_persistence_respects_streaming_credential_deadlines() -> TestResult {
    for socket in [false, true] {
        // @step Given an expired client receives refreshed credentials while another writer holds the credential lock
        let fixture = Fixture::new()?;
        fixture.seed("A")?;
        let issuer = RefreshServer::start().await?;
        let api = MockServer::start().await;
        Mock::given(wiremock::matchers::method("POST"))
            .respond_with(
                ResponseTemplate::new(200).insert_header("content-type", "text/event-stream"),
            )
            .mount(&api)
            .await;
        let mut cached = client(&issuer.uri, 0).with_idle_timeout(Duration::from_millis(250));
        if socket {
            cached = cached.with_websocket(CodexWebSocketHandle::default());
        }
        let before = std::fs::read(fixture.directory.path().join("auth.json"))?;
        let request = tokio::spawn(stream(cached, api.uri()));
        bounded(issuer.started.notified()).await?;
        let home = fixture.directory.path().to_path_buf();
        let (ready, held) = tokio::sync::oneshot::channel();
        let (release, released) = std::sync::mpsc::channel();
        let writer = std::thread::spawn(move || -> Result<(), std::io::Error> {
            let guard = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(home.join(".fspec-auth.lock"))?;
            guard.lock()?;
            let _ = ready.send(());
            let _ = released.recv_timeout(Duration::from_secs(1));
            Ok(())
        });
        bounded(held).await??;
        issuer.release.add_permits(1);
        // @step When the streaming or socket credential deadline expires before the lock is released
        let started = std::time::Instant::now();
        let result = bounded(request).await?;
        let elapsed = started.elapsed();
        let _ = release.send(());
        writer
            .join()
            .map_err(|_| "credential writer thread failed")??;
        // @step Then the request fails promptly without blocking the runtime
        assert!(
            elapsed < Duration::from_millis(750),
            "runtime blocked for {elapsed:?}"
        );
        assert!(
            result?.is_err(),
            "credential deadline did not cancel refresh"
        );
        // @step And releasing the lock does not persist the cancelled refresh
        assert_eq!(
            std::fs::read(fixture.directory.path().join("auth.json"))?,
            before
        );
        assert!(api
            .received_requests()
            .await
            .ok_or("API capture unavailable")?
            .is_empty());
    }
    Ok(())
}
