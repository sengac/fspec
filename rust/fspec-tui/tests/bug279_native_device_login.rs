//! Feature: spec/features/native-codex-device-code-settings.feature
#[path = "../../providers/tests/prov159_support/mod.rs"]
pub mod auth;
#[path = "../../providers/tests/device_protocol/support.rs"]
pub mod support;

use auth::{Fixture, TestResult};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::provider_settings::nav_item::{NavItemKind, OAuthMethod};
use codelet_fspec_tui::views::{ProviderSettingsEvent, ProviderSettingsMode};
use codelet_fspec_tui::{App, EmbeddedFspecBackend};
use codelet_providers::codex::codex_auth::read_codex_auth;
use codelet_providers::codex::oauth_test_support::{configure, OAuthTestConfig};
use codelet_rpc::SharedFspecService;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serial_test::serial;
use std::sync::Arc;
use support::*;
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

async fn application(
    server: &MockServer,
) -> Result<
    (
        tempfile::TempDir,
        App,
        codelet_providers::codex::oauth_test_support::OAuthTestGuard,
    ),
    Box<dyn std::error::Error>,
> {
    let config = configure(OAuthTestConfig {
        issuer: server.uri(),
        callback_address: "127.0.0.1:0".parse()?,
        state: "SYNTHETIC-state".into(),
    });
    let project = codelet_test_helpers::setup_temp_directory()?;
    std::fs::create_dir_all(project.path().join("spec"))?;
    std::fs::write(
        project.path().join("spec/work-units.json"),
        r#"{"workUnits":{}}"#,
    )?;
    let backend = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        Arc::new(SharedFspecService::new(Arc::new(WorkUnitsWatcher::new(
            project.path(),
        )?))),
    ));
    let mut app = App::new(backend);
    let providers = codelet_fspec_tui::views::provider_settings::projection::project_display_infos(
        &[codelet_rpc_types::ProviderCredentialInfo {
            provider_id: "codex".into(),
            display_name: "Codex (ChatGPT)".into(),
            configured: false,
            credential_type: "oauth".into(),
            model_count: 0,
            masked_key: None,
            source: None,
        }],
        &[],
    );
    app.navigator_mut()
        .provider_settings
        .set_provider_display_infos(providers);
    app.navigator_mut()
        .provider_settings
        .toggle_expansion("codex");
    let view = &mut app.navigator_mut().provider_settings;
    view.selected_index = view.nav_items.iter().position(|item| item.provider_id == "codex" && matches!(&item.kind,
        NavItemKind::OAuthLogin { method: OAuthMethod::Headless, label } if label == "Sign in with device code")).ok_or("native projected device row missing")?;
    Ok((project, app, config))
}

fn enter(app: &mut App) -> TestResult {
    let event = app
        .navigator_mut()
        .provider_settings
        .handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let ProviderSettingsEvent::Emit(action) = event else {
        return Err("device Enter emitted no action".into());
    };
    app.dispatch(*action);
    Ok(())
}

async fn result_action(app: &mut App) -> Result<Action, Box<dyn std::error::Error>> {
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        app.next_pending_task().ok_or("missing native OAuth task")?,
    )
    .await??;
    app.try_recv_action()
        .ok_or_else(|| "missing native OAuth result action".into())
}

async fn complete(app: &mut App) -> TestResult {
    let ready = result_action(app).await?;
    assert!(
        matches!(&ready, Action::OAuthDeviceReady { user_code, .. } if user_code == USER_CODE),
        "native start failed: {ready:?}"
    );
    app.dispatch(ready);
    assert!(matches!(
        app.navigator().provider_settings.mode,
        ProviderSettingsMode::OAuthDeviceWaiting { .. }
    ));
    let success = result_action(app).await?;
    assert!(
        matches!(success, Action::OAuthLoginSucceeded { .. }),
        "native completion failed: {success:?}"
    );
    app.dispatch(success);
    assert!(matches!(
        app.navigator().provider_settings.mode,
        ProviderSettingsMode::OAuthSuccess { .. }
    ));
    assert_eq!(
        read_codex_auth()?
            .and_then(|auth| auth.tokens)
            .ok_or("no authorized credentials")?
            .account_id,
        "SYNTHETIC-account"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_native_settings_completes_official_device_authorization() -> TestResult {
    // @step Given a native Codex settings login backed by isolated loopback authorization
    let _fixture = Fixture::new()?;
    let server = MockServer::start().await;
    official_start(&server, usercode()).await;
    official_poll(&server).await;
    official_exchange(&server).await;
    let (_project, mut app, _config) = application(&server).await?;
    // @step When the user selects device login and completes the official protocol
    enter(&mut app)?;
    // @step Then settings shows success and only the authorized synthetic credentials are persisted
    complete(&mut app).await?;
    assert_eq!(count(&server, "/api/accounts/deviceauth/usercode").await, 1);
    assert_eq!(count(&server, "/api/accounts/deviceauth/token").await, 1);
    assert_eq!(count(&server, "/oauth/token").await, 1);
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_native_settings_reports_redacted_errors_and_permits_retry() -> TestResult {
    // @step Given a device authorization server returning a secret bearing start failure
    let fixture = Fixture::new()?;
    let server = MockServer::start().await;
    Mock::given(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(404).set_body_string(SECRET))
        .mount(&server)
        .await;
    let (_project, mut app, _config) = application(&server).await?;
    // @step When native settings starts device login and the user retries after failure
    enter(&mut app)?;
    let failure = result_action(&mut app).await?;
    app.dispatch(failure);
    let ProviderSettingsMode::OAuthError { error, .. } = &app.navigator().provider_settings.mode
    else {
        return Err("missing user-visible native error".into());
    };
    // @step Then settings displays a useful redacted error and a fresh authorized retry can succeed
    assert!(error.contains("404"), "{error}");
    assert!(!error.contains(SECRET), "server secret reached settings");
    let area = ratatui::layout::Rect::new(0, 0, 100, 30);
    let mut buffer = ratatui::buffer::Buffer::empty(area);
    app.navigator_mut()
        .provider_settings
        .render(area, &mut buffer);
    let screen: String = buffer
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(screen.contains("404"));
    assert!(!screen.contains(SECRET));
    assert!(!fixture.directory.path().join("auth.json").exists());
    assert_eq!(count(&server, "/oauth/token").await, 0);
    server.reset().await;
    official_start(&server, usercode()).await;
    official_poll(&server).await;
    official_exchange(&server).await;
    enter(&mut app)?;
    complete(&mut app).await?;
    Ok(())
}
