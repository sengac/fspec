//! Feature: spec/features/client-codex-browser-sign-in.feature

use codelet_fspec_tui::views::{OAuthMethod, ProviderSettingsMode};
use codelet_fspec_tui::{Action, App, EmbeddedFspecBackend, WebSocketFspecBackend};
use codelet_providers::codex::oauth_test_support::{configure, OAuthTestConfig, OAuthTestGuard};
use codelet_test_helpers::{deadlock_guard, OrFail, TestError, TestResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{buffer::Buffer, layout::Rect};
use std::sync::Arc;

mod common;
#[path = "../../providers/tests/fixtures/mod.rs"]
mod oauth_fixtures;
use common::clipboard::{osc52, Clipboard, Launcher};
use common::{MockBackend, TaskOutcome};

const URL: &str = "https://auth.openai.com/oauth/authorize?state=abcdefghijklmnopqrstuvwxyz&code_challenge=012345678901234567890123456789END";

fn setup() -> (App, Launcher, Clipboard) {
    let mut app = App::new(Arc::new(MockBackend::new()));
    app.dispatch(Action::OpenProviderSettingsView);
    let launcher = Launcher::attach(&mut app);
    let clipboard = Clipboard::attach(&mut app);
    app.navigator_mut().provider_settings.mode = ProviderSettingsMode::OAuthBrowserWaiting {
        provider_id: "codex".into(),
    };
    (app, launcher, clipboard)
}

fn ready(app: &mut App, generation: u64) {
    app.dispatch(Action::OAuthBrowserReady {
        provider_id: "codex".into(),
        authorize_url: URL.into(),
        generation,
    });
}

async fn drain(app: &mut App) -> TestResult {
    loop {
        let mut progressed = false;
        while let Some(task) = app.next_pending_task() {
            task.await.completed_or_cancelled()?;
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
}

#[tokio::test]
async fn browser_url_can_be_copied_and_reopened() -> TestResult {
    // @step Given Codex browser authorization is ready
    let (mut app, launcher, clipboard) = setup();
    ready_now(&mut app);
    // @step When I press c and Ctrl+C and o
    for modifiers in [KeyModifiers::NONE, KeyModifiers::CONTROL] {
        app.handle_event(&Event::Key(KeyEvent::new(KeyCode::Char('c'), modifiers)));
    }
    app.handle_event(&Event::Key(KeyEvent::new(
        KeyCode::Char('o'),
        KeyModifiers::NONE,
    )));
    drain(&mut app).await?;
    // @step Then the complete URL is copied twice and reopened
    assert_eq!(clipboard.bytes()?, [osc52(URL), osc52(URL)].concat());
    assert_eq!(launcher.opened()?, vec![URL, URL]);
    Ok(())
}

#[tokio::test]
async fn browser_url_and_launch_errors_wrap() -> TestResult {
    // @step Given Codex browser authorization is ready but the browser cannot open
    let (mut app, launcher, _) = setup();
    launcher.fail_with("browser unavailable")?;
    ready_now(&mut app);
    drain(&mut app).await?;
    // @step When I view the waiting screen in a narrow terminal
    let area = Rect::new(0, 0, 45, 35);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    let compact: String = buffer
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>()
        .split_whitespace()
        .collect();
    // @step Then the complete URL and launch failure remain visible across lines
    assert!(compact.contains(URL));
    assert!(compact.contains("browserunavailable"));
    assert!(compact.contains("c:copyURL"));
    drain(&mut app).await?;
    Ok(())
}

#[tokio::test]
#[serial_test::serial(codex_callback)]
async fn cancellation_releases_callback_and_ignores_late_url() -> TestResult {
    // @step Given Codex browser authorization is waiting for a local callback
    let (_credentials, _home) = oauth_fixtures::setup_codex_home()?;
    let (address, _callback) = callback_address().await?;
    let (_directory, service) = common::temp_service_with_stub()?;
    let backend = EmbeddedFspecBackend::new(tokio::runtime::Handle::current(), service);
    let mut app = App::new(Arc::new(backend));
    app.dispatch(Action::OpenProviderSettingsView);
    let launcher = Launcher::attach(&mut app);
    drain(&mut app).await?;
    let started = start_browser(&mut app, &launcher).await?;
    assert!(
        tokio::net::TcpListener::bind(address).await.is_err(),
        "the callback listener does not hold its configured address"
    );
    // @step When I cancel sign-in and a late ready event arrives
    app.handle_event(&Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
    drain(&mut app).await?;
    ready(&mut app, started);
    // @step Then the callback port is released and no further browser opens
    assert_eq!(launcher.opened()?.len(), 1);
    assert!(matches!(
        app.navigator().provider_settings.mode,
        ProviderSettingsMode::List
    ));
    let listener = tokio::net::TcpListener::bind(address).await?;
    drop(listener);
    Ok(())
}

/// BUG-685: an ephemeral callback address for the client listener, so the
/// tests never need (or collide on) the real port 1455. The issuer is never
/// reached: no test here exchanges a code.
async fn callback_address() -> Result<(std::net::SocketAddr, OAuthTestGuard), TestError> {
    let address = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await?
        .local_addr()?;
    let guard = configure(OAuthTestConfig {
        issuer: "http://127.0.0.1:9".into(),
        callback_address: address,
        state: "unused".into(),
    });
    Ok((address, guard))
}

/// BUG-685: start browser sign-in and wait for its first action. A failed
/// start (e.g. the callback address is taken) fails at once with its error;
/// a ready one is dispatched and its browser launch awaited.
async fn start_browser(app: &mut App, launcher: &Launcher) -> Result<u64, TestError> {
    app.dispatch(Action::OAuthLoginStart {
        provider_id: "codex".into(),
        method: OAuthMethod::Browser,
        generation: app.navigator().provider_settings.oauth_generation(),
    });
    let first = deadlock_guard("the browser login's first action", async {
        loop {
            if let Some(action) = app.try_recv_action() {
                return action;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    if let Action::OAuthLoginFailed { error, .. } = &first {
        return Err(format!("browser login failed to start: {error}").into());
    }
    app.dispatch(first);
    app.next_pending_task()
        .or_fail("browser launch task")?
        .await
        .completed_or_cancelled()?;
    launcher
        .opened()?
        .first()
        .or_fail("client browser opened")?;
    Ok(app.navigator().provider_settings.oauth_generation())
}

#[tokio::test]
#[serial_test::serial(codex_callback)]
async fn daemon_browser_receives_automatic_callback_error() -> TestResult {
    // @step Given Codex browser sign-in is connected to a WebSocket daemon
    let (_credentials, _home) = oauth_fixtures::setup_codex_home()?;
    let (address, _callback) = callback_address().await?;
    let (_directory, service) = common::temp_service_with_stub()?;
    let (daemon, server) = common::start_ws_server(service).await?;
    let backend = WebSocketFspecBackend::connect(common::ws_url(daemon)?).await?;
    let mut app = App::new(Arc::new(backend));
    app.dispatch(Action::OpenProviderSettingsView);
    drain(&mut app).await?;
    let launcher = Launcher::attach(&mut app);
    start_browser(&mut app, &launcher).await?;
    let urls = launcher.opened()?;
    let authorize_url = url::Url::parse(urls.first().or_fail("client browser opened")?)?;
    let state = authorize_url
        .query_pairs()
        .find(|(name, _)| name == "state")
        .or_fail("authorization state")?
        .1
        .into_owned();
    // @step When the browser redirects to the local callback with an authorization error
    let response = reqwest::get(format!(
        "http://{address}/auth/callback?error=access_denied&state={state}"
    ))
    .await?;
    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    assert!(response.text().await?.contains("access_denied"));
    drain(&mut app).await?;
    // @step Then the TUI reports the authorization error without asking for a pasted code
    assert!(matches!(
        &app.navigator().provider_settings.mode,
        ProviderSettingsMode::OAuthError { error, .. } if error.contains("access_denied")
    ));
    server.abort();
    Ok(())
}

/// BUG-575: the authorization arrives for the login the view is waiting on.
fn ready_now(app: &mut App) {
    let generation = app.navigator().provider_settings.oauth_generation();
    ready(app, generation);
}
