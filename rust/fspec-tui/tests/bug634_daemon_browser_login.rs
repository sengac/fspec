//! Feature: spec/features/daemon-anthropic-browser-sign-in.feature

use std::sync::Arc;

use codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind;
use codelet_fspec_tui::views::provider_settings::projection::project_display_infos;
use codelet_fspec_tui::views::{OAuthMethod, ProviderSettingsMode};
use codelet_fspec_tui::{Action, App, WebSocketFspecBackend};
use codelet_rpc_types::ProviderCredentialInfo;
use codelet_test_helpers::{deadlock_guard, OrFail, TestResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;
use tokio::task::JoinHandle;

mod common;
use common::clipboard::{osc52, Clipboard, Launcher};
use common::TaskOutcome;

struct Screen {
    app: App,
    launcher: Launcher,
    clipboard: Clipboard,
    _directory: TempDir,
    server: JoinHandle<()>,
}

/// BUG-698: dispatch the app's actions under the deadlock guard until the
/// browser form is ready (a failed start fails at once with its error), then
/// await the browser launch it spawned; no polling for the launch and no
/// fixed deadline.
async fn wait_for_browser(screen: &mut Screen) -> TestResult {
    let app = &mut screen.app;
    deadlock_guard("the browser form becomes ready", async {
        loop {
            let Some(action) = app.try_recv_action() else {
                tokio::task::yield_now().await;
                continue;
            };
            if let Action::OAuthLoginFailed { error, .. } = &action {
                return Err(format!("browser login failed to start: {error}"));
            }
            let ready = matches!(action, Action::OAuthBrowserReady { .. });
            app.dispatch(action);
            if ready {
                return Ok(());
            }
        }
    })
    .await??;
    deadlock_guard(
        "the browser launch",
        screen
            .app
            .next_pending_task()
            .or_fail("browser launch task")?,
    )
    .await?
    .completed_or_cancelled()?;
    screen
        .launcher
        .opened()?
        .first()
        .or_fail("client browser opened")?;
    Ok(())
}

impl Drop for Screen {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn drain(app: &mut App) -> TestResult {
    for _ in 0..50 {
        let mut progressed = false;
        while let Some(handle) = app.next_pending_task() {
            deadlock_guard("a pending OAuth task", handle)
                .await?
                .completed_or_cancelled()?;
            progressed = true;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
            progressed = true;
        }
        if !progressed {
            return Ok(());
        }
    }
    Err("OAuth actions did not settle".into())
}

async fn settings() -> Result<Screen, codelet_test_helpers::TestError> {
    let (directory, service) = common::temp_service_with_stub()?;
    let (address, server) = common::start_ws_server(service).await?;
    let backend = WebSocketFspecBackend::connect(common::ws_url(address)?).await?;
    let mut app = App::new(Arc::new(backend));
    let launcher = Launcher::attach(&mut app);
    let clipboard = Clipboard::attach(&mut app);
    app.dispatch(Action::OpenProviderSettingsView);
    drain(&mut app).await?;
    // BUG-664: seed the signed-out OAuth rows straight into the view.
    let infos: Vec<ProviderCredentialInfo> = ["anthropic", "codex"]
        .into_iter()
        .map(|provider| ProviderCredentialInfo {
            provider_id: provider.to_owned(),
            display_name: provider.to_owned(),
            configured: false,
            credential_type: "oauth".to_owned(),
            model_count: 0,
            masked_key: None,
            source: None,
        })
        .collect();
    let view = &mut app.navigator_mut().provider_settings;
    view.set_provider_display_infos(project_display_infos(&infos, &[]));
    view.set_providers(infos);
    app.navigator_mut()
        .provider_settings
        .toggle_expansion("anthropic");
    app.navigator_mut()
        .provider_settings
        .toggle_expansion("codex");
    Ok(Screen {
        app,
        launcher,
        clipboard,
        _directory: directory,
        server,
    })
}

fn press(app: &mut App, key: KeyCode) {
    app.handle_event(&Event::Key(KeyEvent::new(key, KeyModifiers::NONE)));
}

fn select(app: &mut App, method: OAuthMethod) -> TestResult {
    let view = &mut app.navigator_mut().provider_settings;
    let index = view
        .nav_items
        .iter()
        .position(|item| {
            matches!(
                &item.kind,
                NavItemKind::OAuthLogin { method: candidate, .. }
                    if item.provider_id == "anthropic" && *candidate == method
            )
        })
        .or_fail("Anthropic login row must be present")?;
    view.selected_index = index;
    press(app, KeyCode::Enter);
    Ok(())
}

fn entry(app: &App) -> Result<(String, String), codelet_test_helpers::TestError> {
    match &app.navigator().provider_settings.mode {
        ProviderSettingsMode::OAuthHeadlessCodeEntry {
            authorize_url,
            code_input,
            ..
        } => Ok((authorize_url.clone(), code_input.clone())),
        other => Err(format!("Expected code entry, got {other:?}").into()),
    }
}

#[tokio::test]
async fn browser_opens_on_client_and_submits_to_daemon() -> TestResult {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    // @step Given Provider Settings is connected to a daemon
    let mut screen = settings().await?;
    // @step When I select Anthropic Sign in with browser
    select(&mut screen.app, OAuthMethod::Browser)?;
    wait_for_browser(&mut screen).await?;
    // @step Then my browser opens the original authorization link and code paste form
    let opened = screen.launcher.opened()?;
    let url = opened.first().or_fail("browser form URL")?;
    assert!(url.starts_with("http://127.0.0.1:"));
    let client = reqwest::Client::new();
    let form = client.get(url).send().await?.text().await?;
    assert!(form.contains("https://claude.ai/oauth/authorize?"));
    assert!(form.contains("Submit Code"));
    assert!(matches!(
        screen.app.navigator().provider_settings.mode,
        ProviderSettingsMode::OAuthBrowserWaiting { .. }
    ));
    // @step When I submit a code with the wrong state through the browser form
    let response = client
        .post(format!("{url}submit"))
        .form(&[("code", "code#wrong-state")])
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    drain(&mut screen.app).await?;
    // @step Then the login reports the state mismatch rather than unsupported login
    assert!(matches!(&screen.app.navigator().provider_settings.mode,
        ProviderSettingsMode::OAuthError { error, .. }
            if error.contains("CSRF validation failed") && error.contains("state mismatch")));
    Ok(())
}

#[tokio::test]
async fn code_login_does_not_open_browser() -> TestResult {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    // @step Given Provider Settings is connected to a daemon
    let mut screen = settings().await?;
    // @step When I select Anthropic Sign in with code
    select(&mut screen.app, OAuthMethod::Headless)?;
    drain(&mut screen.app).await?;
    // @step Then the authorization URL and code field appear without opening a browser
    assert!(entry(&screen.app)?
        .0
        .starts_with("https://claude.ai/oauth/authorize?"));
    assert!(screen.launcher.opened()?.is_empty());
    // @step And Codex also offers browser sign-in
    assert!(screen
        .app
        .navigator()
        .provider_settings
        .nav_items
        .iter()
        .any(
            |item| matches!(&item.kind, NavItemKind::OAuthLogin { method: OAuthMethod::Browser, .. }
            if item.provider_id == "codex")
        ));
    Ok(())
}

#[tokio::test]
async fn launch_failure_leaves_usable_code_entry() -> TestResult {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    // @step Given Provider Settings is connected to a daemon and my browser cannot open
    let mut screen = settings().await?;
    screen.launcher.fail_with("browser unavailable")?;
    // @step When I select Anthropic Sign in with browser
    select(&mut screen.app, OAuthMethod::Browser)?;
    wait_for_browser(&mut screen).await?;
    // The finished launch already sent its failure status.
    while let Some(action) = screen.app.try_recv_action() {
        screen.app.dispatch(action);
    }
    // @step Then the launch error explains that I can copy the URL
    let status = &screen.app.navigator().provider_settings.status;
    assert!(status.contains("browser unavailable"));
    assert!(status.contains("Press c to copy the URL"));
    let opened = screen.launcher.opened()?;
    let url = opened.first().or_fail("browser form URL")?;
    // @step When I copy the URL and visit it manually
    assert!(url.starts_with("http://127.0.0.1:"));
    press(&mut screen.app, KeyCode::Char('c'));
    while let Some(action) = screen.app.try_recv_action() {
        screen.app.dispatch(action);
    }
    let form = reqwest::get(url).await?.text().await?;
    // @step Then the clipboard contains the URL and the browser paste form is usable
    assert!(screen.clipboard.bytes()?.ends_with(&osc52(url)));
    assert!(form.contains("Submit Code"));
    press(&mut screen.app, KeyCode::Esc);
    drain(&mut screen.app).await?;
    Ok(())
}

#[tokio::test]
async fn cancelling_ignores_late_authorization_url() -> TestResult {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    // @step Given Provider Settings is connected to a daemon
    let mut screen = settings().await?;
    let generation = screen.app.navigator().provider_settings.oauth_generation();
    // @step When I start Anthropic browser sign-in and cancel before it is ready
    select(&mut screen.app, OAuthMethod::Browser)?;
    press(&mut screen.app, KeyCode::Esc);
    assert_ne!(
        screen.app.navigator().provider_settings.oauth_generation(),
        generation
    );
    // @step And the cancelled authorization URL arrives
    screen.app.dispatch(Action::OAuthBrowserReady {
        provider_id: "anthropic".to_owned(),
        authorize_url: "https://claude.ai/oauth/authorize?cancelled".to_owned(),
        generation,
    });
    drain(&mut screen.app).await?;
    // @step Then no browser opens and Provider Settings stays on the list
    assert!(screen.launcher.opened()?.is_empty());
    assert!(matches!(
        screen.app.navigator().provider_settings.mode,
        ProviderSettingsMode::List
    ));
    Ok(())
}
