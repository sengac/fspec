//! Feature: spec/features/provider-settings-oauth-login.feature
//! Primary feature: spec/features/prov159-rpc-lifecycle.feature

#[path = "../../providers/tests/prov159_support/network.rs"]
pub mod network;
#[path = "../../providers/tests/prov159_support/mod.rs"]
pub mod prov159_support;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend};
use codelet_providers::codex::codex_auth::read_codex_auth;
use codelet_providers::codex::oauth_test_support::{configure, OAuthTestConfig};
use codelet_rpc::SharedFspecService;
use codelet_rpc_embedded::EmbeddedTransport;
use network::{bounded, RefreshServer};
use prov159_support::{Fixture, TestResult};
use serial_test::serial;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::test]
#[serial]
async fn scenario_a11_esc_permits_immediate_browser_retry_without_stale_commit() -> TestResult {
    // @step Given Codex browser login attempt A is waiting on the local callback port
    // @step When the user presses Esc and immediately starts browser login attempt B before delivering the old A callback
    // @step Then the requesting task for attempt A is cancelled rather than merely hiding its result
    // @step And attempt B can bind the same callback port without waiting for attempt A to time out
    // @step And the late callback for attempt A cannot persist credentials or complete attempt B
    // @step And only the valid callback for attempt B can commit credentials and show success
    // @step And the cancellation and retry execute through the real embedded backend
    use codelet_fspec_tui::components::Action;
    use codelet_fspec_tui::views::{OAuthMethod, ProviderSettingsMode};
    use codelet_fspec_tui::App;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let _fixture = Fixture::new()?;
    let issuer = RefreshServer::with_body(response()).await?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    drop(listener);
    let config = configure(OAuthTestConfig {
        issuer: issuer.uri.clone(),
        callback_address: address,
        state: "old".into(),
    });
    let project = codelet_test_helpers::setup_temp_directory()?;
    let backend = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service(project.path())?,
    ));
    let mut app = App::new(backend);
    app.navigator_mut().provider_settings.mode = ProviderSettingsMode::OAuthBrowserWaiting {
        provider_id: "codex".into(),
    };
    app.dispatch(Action::OAuthLoginStart {
        provider_id: "codex".into(),
        method: OAuthMethod::Browser,
        generation: 0,
    });
    probe(address).await?;
    let pending = app.next_pending_task().ok_or("missing login task")?;
    app.navigator_mut()
        .provider_settings
        .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(
        bounded(pending).await?.is_err(),
        "Esc left the login request running"
    );
    drop(config);
    let _config = configure(OAuthTestConfig {
        issuer: issuer.uri.clone(),
        callback_address: address,
        state: "rpc".into(),
    });
    app.dispatch(Action::OAuthLoginStart {
        provider_id: "codex".into(),
        method: OAuthMethod::Browser,
        generation: 1,
    });
    probe(address).await?;
    let stale = reqwest::get(format!("http://{address}/auth/callback?code=old&state=old")).await?;
    assert_eq!(stale.status(), reqwest::StatusCode::BAD_REQUEST);
    assert!(
        issuer.requests()?.is_empty(),
        "stale callback reached token exchange"
    );
    assert!(read_codex_auth()?.and_then(|auth| auth.tokens).is_none());
    callback(address).await?;
    bounded(issuer.started.notified()).await?;
    issuer.release.add_permits(1);
    bounded(app.next_pending_task().ok_or("missing replacement login")?).await??;
    assert_eq!(
        read_codex_auth()?
            .and_then(|auth| auth.tokens)
            .ok_or("no committed login")?
            .account_id,
        "B"
    );
    Ok(())
}

async fn probe(address: std::net::SocketAddr) -> TestResult {
    bounded(async {
        loop {
            if reqwest::get(format!("http://{address}/probe"))
                .await
                .is_ok()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

fn service(path: &std::path::Path) -> Result<Arc<SharedFspecService>, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(path.join("spec"))?;
    std::fs::write(path.join("spec/work-units.json"), r#"{"workUnits":{}}"#)?;
    Ok(Arc::new(SharedFspecService::new(Arc::new(
        WorkUnitsWatcher::new(path)?,
    ))))
}

fn response() -> String {
    use base64::Engine;
    let payload =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"chatgpt_account_id":"B"}"#);
    serde_json::json!({"id_token": format!("e30.{payload}.sig"), "access_token": "access-B",
        "refresh_token": "refresh-B", "expires_in": 3600})
    .to_string()
}

async fn callback(address: std::net::SocketAddr) -> TestResult {
    bounded(async {
        loop {
            if reqwest::get(format!(
                "http://{address}/auth/callback?code=code&state=rpc"
            ))
            .await
            .is_ok()
            {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a12_interactive_oauth_outlives_default_rpc_deadline() -> TestResult {
    // @step Given Codex browser authorization or device polling runs through the real embedded tarpc transport
    // @step When authorization completes after more than 10 seconds but within the supported 300-second OAuth budget
    // @step Then the login succeeds and the committed credentials are available to the settings UI and providers
    // @step And unrelated RPC calls retain their existing short default deadlines
    // @step And the operation budget includes callback waiting, polling and token exchange
    let _fixture = Fixture::new()?;
    let issuer = RefreshServer::with_body(response()).await?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    drop(listener);
    let _config = configure(OAuthTestConfig {
        issuer: issuer.uri.clone(),
        callback_address: address,
        state: "rpc".into(),
    });
    let project = codelet_test_helpers::setup_temp_directory()?;
    let backend = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service(project.path())?,
    ));
    let pending = tokio::spawn({
        let backend = backend.clone();
        async move { backend.oauth_browser_login("codex".into()).await }
    });
    callback(address).await?;
    bounded(issuer.started.notified()).await?;
    tokio::time::sleep(std::time::Duration::from_secs(11)).await;
    issuer.release.add_permits(1);
    bounded(pending).await???;
    assert!(backend.oauth_get_tokens("codex".into()).await?);
    assert!(
        tarpc::context::current()
            .deadline
            .duration_since(std::time::SystemTime::now())?
            .as_secs()
            <= 10
    );
    let device = RefreshServer::with_body(
        serde_json::json!({
            "device_auth_id": "device", "user_code": "ABCD", "interval": 1
        })
        .to_string(),
    )
    .await?;
    drop(_config);
    let _device_config = configure(OAuthTestConfig {
        issuer: device.uri.clone(),
        callback_address: address,
        state: "unused".into(),
    });
    let pending = tokio::spawn({
        let backend = backend.clone();
        async move { backend.oauth_device_start("codex".into()).await }
    });
    bounded(device.started.notified()).await?;
    tokio::time::sleep(std::time::Duration::from_secs(11)).await;
    device.release.add_permits(1);
    let started = bounded(pending).await???;
    assert_eq!(started.device_auth_id, "device");
    codelet_providers::codex::auth_lifecycle::cancel_login();
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a13_rpc_cancellation_cleans_up_login() -> TestResult {
    // @step Given a Codex browser or device login is pending through the real embedded tarpc transport
    // @step When its operation deadline expires or the requesting client disconnects before authorization completes
    // @step Then the operation has a clear timeout or cancellation outcome and reports no successful login
    // @step And the callback listener, polling tasks and exchange work are cleaned up
    // @step And delayed authorization cannot persist credentials after cleanup
    // @step And a subsequent login can immediately reuse the callback port
    for deadline in [false, true] {
        let _fixture = Fixture::new()?;
        let issuer = RefreshServer::with_body(response()).await?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        drop(listener);
        let _config = configure(OAuthTestConfig {
            issuer: issuer.uri.clone(),
            callback_address: address,
            state: "rpc".into(),
        });
        let project = codelet_test_helpers::setup_temp_directory()?;
        let transport =
            EmbeddedTransport::new(tokio::runtime::Handle::current(), service(project.path())?);
        let client = transport.client();
        let mut context = tarpc::context::current();
        if deadline {
            context.deadline = std::time::SystemTime::now() + std::time::Duration::from_secs(2);
        }
        let pending =
            tokio::spawn(async move { client.oauth_browser_login(context, "codex".into()).await });
        callback(address)
            .await
            .map_err(|error| format!("callback (deadline={deadline}): {error}"))?;
        bounded(issuer.started.notified())
            .await
            .map_err(|error| format!("exchange start (deadline={deadline}): {error}"))?;
        if deadline {
            assert!(bounded(pending)
                .await
                .map_err(|error| format!("deadline cancellation: {error}"))??
                .is_err());
        } else {
            pending.abort();
            let _ = pending.await;
        }
        bounded(async {
            loop {
                if TcpListener::bind(address).await.is_ok() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await?;
        issuer.release.add_permits(1);
        tokio::task::yield_now().await;
        assert!(read_codex_auth()?.and_then(|auth| auth.tokens).is_none());
    }
    Ok(())
}
