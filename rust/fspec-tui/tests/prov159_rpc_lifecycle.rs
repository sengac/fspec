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
use codelet_test_helpers::{OrFail, OrFailErr};
use network::{bounded, RefreshServer};
use prov159_support::{Fixture, TestResult};
use serial_test::serial;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::test]
#[serial]
async fn scenario_a11_esc_permits_immediate_browser_retry_without_stale_commit() -> TestResult {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    use codelet_fspec_tui::components::Action;
    use codelet_fspec_tui::views::ProviderSettingsMode;
    use codelet_fspec_tui::App;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // @step Given Codex browser login attempt A is waiting on the local callback port
    let _fixture = Fixture::new()?;
    let issuer = RefreshServer::with_body(response()).await?;
    let address = free_address().await?;
    // BUG-686: the TUI hosts the callback listener (BUG-634); it binds the
    // configured test address. The state comes from each attempt's URL.
    let _config = configure(OAuthTestConfig {
        issuer: issuer.uri.clone(),
        callback_address: address,
        state: "unused".into(),
    });
    let project = codelet_test_helpers::setup_temp_directory()?;
    let backend = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service(project.path())?,
    ));
    let mut app = App::new(backend);
    let state_a = start_browser(&mut app, address).await?;
    let pending = app.next_pending_task().ok_or("missing login task")?;

    // @step When the user presses Esc and immediately starts browser login attempt B before delivering the old A callback
    app.navigator_mut()
        .provider_settings
        .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    // @step Then the requesting task for attempt A is cancelled rather than merely hiding its result
    let joined = bounded(pending).await?;
    assert!(
        joined
            .as_ref()
            .is_err_and(tokio::task::JoinError::is_cancelled),
        "Esc left the login request running: {joined:?}"
    );

    // @step And attempt B can bind the same callback port without waiting for attempt A to time out
    let state_b = start_browser(&mut app, address).await?;
    assert_ne!(state_a, state_b);

    // @step And the late callback for attempt A cannot persist credentials or complete attempt B
    let stale = reqwest::get(format!(
        "http://{address}/auth/callback?code=old&state={state_a}"
    ))
    .await?;
    assert_eq!(stale.status(), reqwest::StatusCode::BAD_REQUEST);
    assert!(
        issuer.requests()?.is_empty(),
        "stale callback reached token exchange"
    );
    assert!(read_codex_auth()?.and_then(|auth| auth.tokens).is_none());
    assert!(
        app.try_recv_action().is_none(),
        "stale callback completed attempt B"
    );

    // @step And only the valid callback for attempt B can commit credentials and show success
    let valid = reqwest::get(format!(
        "http://{address}/auth/callback?code=code&state={state_b}"
    ))
    .await?;
    assert_eq!(valid.status(), reqwest::StatusCode::OK);
    bounded(issuer.started.notified()).await?;
    issuer.release.add_permits(1);
    bounded(app.next_pending_task().ok_or("missing replacement login")?).await??;
    let outcome = app.try_recv_action().ok_or("attempt B sent no outcome")?;
    assert!(
        matches!(&outcome, Action::OAuthLoginSucceeded { provider_id, .. } if provider_id == "codex"),
        "attempt B did not succeed: {outcome:?}"
    );
    app.dispatch(outcome);
    assert!(matches!(
        &app.navigator().provider_settings.mode,
        ProviderSettingsMode::OAuthSuccess { provider_id } if provider_id == "codex"
    ));
    assert_eq!(
        read_codex_auth()?
            .and_then(|auth| auth.tokens)
            .ok_or("no committed login")?
            .account_id,
        "B"
    );

    // @step And the cancellation and retry execute through the real embedded backend
    let exchanges = issuer.requests()?;
    assert_eq!(exchanges.len(), 1, "one token exchange, for attempt B only");
    assert!(exchanges[0].contains("code=code"), "{}", exchanges[0]);
    Ok(())
}

/// An address no listener holds: bind an ephemeral port, then release it.
async fn free_address() -> Result<std::net::SocketAddr, std::io::Error> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    listener.local_addr()
}

/// BUG-686: start a Codex browser login the way the settings list does and
/// wait for its `OAuthBrowserReady`, which is sent once the client listener
/// is bound. Returns the authorize URL's state. A login failure (e.g. the
/// address is taken) fails the test at once.
async fn start_browser(
    app: &mut codelet_fspec_tui::App,
    address: std::net::SocketAddr,
) -> Result<String, Box<dyn std::error::Error>> {
    use codelet_fspec_tui::components::Action;
    use codelet_fspec_tui::views::{OAuthMethod, ProviderSettingsMode};
    app.navigator_mut().provider_settings.mode = ProviderSettingsMode::OAuthBrowserWaiting {
        provider_id: "codex".into(),
    };
    app.dispatch(Action::OAuthLoginStart {
        provider_id: "codex".into(),
        method: OAuthMethod::Browser,
        generation: app.navigator().provider_settings.oauth_generation(),
    });
    let ready = bounded(async {
        loop {
            if let Some(action) = app.try_recv_action() {
                return action;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    let Action::OAuthBrowserReady { authorize_url, .. } = ready else {
        return Err(format!("browser login did not become ready: {ready:?}").into());
    };
    let probe = reqwest::get(format!("http://{address}/probe")).await?;
    assert_eq!(probe.status(), reqwest::StatusCode::NOT_FOUND);
    let state = url::Url::parse(&authorize_url)?
        .query_pairs()
        .find(|(name, _)| name == "state")
        .map(|(_, value)| value.into_owned())
        .ok_or("authorize URL has no state")?;
    Ok(state)
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
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
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
    let service = service(project.path())?;
    let backend = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));
    // BUG-600: the TUI never calls the daemon-hosted browser login; the RPC
    // stays for the Paseo plugin, so it is driven through the RPC client
    // with the interactive OAuth budget (310 s), as A13 does.
    let client = EmbeddedTransport::new(tokio::runtime::Handle::current(), service).client();
    let mut context = tarpc::context::current();
    context.deadline = std::time::SystemTime::now() + std::time::Duration::from_secs(310);
    let pending =
        tokio::spawn(async move { client.oauth_browser_login(context, "codex".into()).await });
    callback(address).await?;
    bounded(issuer.started.notified()).await?;
    // TEST-042: move tokio's clock (tarpc's deadlines run on it) 11s ahead
    // instead of sleeping 11s; a 10s default deadline would expire here.
    tokio::time::pause();
    tokio::time::advance(std::time::Duration::from_secs(11)).await;
    tokio::time::resume();
    issuer.release.add_permits(1);
    bounded(pending).await????;
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
    // TEST-042: move tokio's clock (tarpc's deadlines run on it) 11s ahead
    // instead of sleeping 11s; a 10s default deadline would expire here.
    tokio::time::pause();
    tokio::time::advance(std::time::Duration::from_secs(11)).await;
    tokio::time::resume();
    device.release.add_permits(1);
    let started = bounded(pending)
        .await??
        .or_fail("the device login starts after the default deadline")?;
    assert_eq!(started.device_auth_id, "device");
    codelet_providers::codex::auth_lifecycle::cancel_login()?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a13_rpc_cancellation_cleans_up_login() -> TestResult {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
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
            // BUG-698: a deadline no real run reaches; the test moves tokio's
            // clock (tarpc's deadlines run on it) past it below instead of
            // waiting 2 s of wall-clock time.
            context.deadline = std::time::SystemTime::now() + std::time::Duration::from_secs(60);
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
            tokio::time::pause();
            tokio::time::advance(std::time::Duration::from_secs(61)).await;
            tokio::time::resume();
            assert!(bounded(pending)
                .await
                .map_err(|error| format!("deadline cancellation: {error}"))??
                .is_err());
        } else {
            pending.abort();
            assert!(pending
                .await
                .or_fail_err("an aborted task ends cancelled")?
                .is_cancelled());
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
