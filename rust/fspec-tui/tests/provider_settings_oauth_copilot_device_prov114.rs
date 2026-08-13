//! PROV-114 — GitHub Copilot OAuth device flow with the deployment-type /
//! enterprise-host preamble: view/key tests for the two new preamble modes and
//! App-level dispatch tests for the copilot device start/poll, all for one
//! feature.
//!
//! Feature: spec/features/provider-settings-oauth-copilot-device.feature
//!
//! One test file per feature (ACDD 1:1). Sections:
//!   1. VIEW/KEY — `ProviderSettingsView::handle_key` drives the new
//!      `OAuthDeploymentTypeSelect` / `OAuthEnterpriseUrlEntry` modes (no
//!      backend): selection, host typing/validation, Esc-cancel.
//!   2. DISPATCH — `App::dispatch` against the shared `MockBackend` (call
//!      counters + scripted Ok/Err, asserting the normalized enterprise host
//!      passed through), fully offline.
//!
//! No real OAuth network and no real `~/.fspec` mutation.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind;
use codelet_fspec_tui::views::{
    OAuthMethod, ProviderDisplayInfo, ProviderSettingsEvent, ProviderSettingsMode,
    ProviderSettingsView,
};
use codelet_fspec_tui::{App, FspecBackend};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::time::timeout;

mod common;
use common::MockBackend;

// ─────────────────────────── shared key helpers ───────────────────────────

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn char_key(c: char) -> KeyEvent {
    key(KeyCode::Char(c))
}

fn type_str(view: &mut ProviderSettingsView, s: &str) {
    for c in s.chars() {
        view.handle_key(char_key(c));
    }
}

// ──────────────────────────── shared fixtures ─────────────────────────────

/// A github-copilot OAuth provider whose single login row is the device-flow
/// row, NOT yet logged in (so the login row appears and there is no Logout
/// row). The label mirrors the feature wording.
fn copilot_provider() -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: "github-copilot".to_string(),
        name: "GitHub Copilot".to_string(),
        configured: false,
        credential_type: "oauth".to_string(),
        model_count: 0,
        has_oauth_tokens: false,
        is_oauth_provider: true,
        requires_api_key: false,
        env_var: None,
        profiles: Vec::new(),
        oauth_login_methods: vec![(
            OAuthMethod::Headless,
            "Login with GitHub Copilot (device flow)".to_string(),
        )],
        oauth_status_label: None,
        masked_key: None,
        source: None,
    }
}

/// Build a view with the github-copilot provider expanded and the cursor on
/// its device-flow login row.
fn view_on_copilot_login_row() -> ProviderSettingsView {
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![copilot_provider()]);
    view.toggle_expansion("github-copilot");
    let idx = view
        .nav_items
        .iter()
        .position(|i| matches!(&i.kind, NavItemKind::OAuthLogin { .. }))
        .expect("copilot login row present in fixture");
    view.selected_index = idx;
    view
}

/// A view already sitting in the deployment-type-select preamble.
fn view_in_deployment_select() -> ProviderSettingsView {
    let mut view = view_on_copilot_login_row();
    view.mode = ProviderSettingsMode::OAuthDeploymentTypeSelect {
        provider_id: "github-copilot".to_string(),
        selected_index: 0,
    };
    view
}

/// A view sitting in enterprise-url-entry with the given input + error.
fn view_in_enterprise_entry(url_input: &str, error: Option<&str>) -> ProviderSettingsView {
    let mut view = view_on_copilot_login_row();
    view.mode = ProviderSettingsMode::OAuthEnterpriseUrlEntry {
        provider_id: "github-copilot".to_string(),
        url_input: url_input.to_string(),
        validation_error: error.map(str::to_string),
    };
    view
}

/// Concatenate the visible symbols of row `y` into a String.
fn row_string(buf: &Buffer, y: u16) -> String {
    let mut s = String::new();
    for x in 0..buf.area.width {
        s.push_str(buf[(x, y)].symbol());
    }
    s
}

/// True when any row in `buf` contains `needle` as a substring.
fn screen_contains(buf: &Buffer, needle: &str) -> bool {
    (0..buf.area.height).any(|y| row_string(buf, y).contains(needle))
}

fn area() -> Rect {
    Rect {
        x: 0,
        y: 0,
        width: 100,
        height: 30,
    }
}

/// Render a view's current screen into a buffer for text assertions.
fn render_screen(view: &mut ProviderSettingsView) -> Buffer {
    let a = area();
    let mut buf = Buffer::empty(a);
    view.render(a, &mut buf);
    buf
}

// ════════════════════════ SECTION 1: VIEW / KEY ════════════════════════

// ─────────────────────────────────────────────────────────────────────────
// Scenario: GitHub.com device flow goes straight to device-waiting and
// connects (view/key half — Enter on the copilot login row enters
// deployment-type-select with GitHub.com selected at index 0, and the screen
// shows the deployment-type prompt; pressing Enter on index 0 emits the
// copilot device-start with NO enterprise host).
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn copilot_login_enters_deployment_select_and_github_dot_com_starts_device() {
    // @step Given the "github-copilot" provider is expanded
    // @step And the cursor is on the "Login with GitHub Copilot (device flow)" row
    let mut view = view_on_copilot_login_row();

    // @step When the user presses Enter
    view.handle_key(key(KeyCode::Enter));

    // @step Then the mode becomes oauth-deployment-type-select for provider "github-copilot"
    match &view.mode {
        ProviderSettingsMode::OAuthDeploymentTypeSelect {
            provider_id,
            selected_index,
        } => {
            assert_eq!(provider_id, "github-copilot");
            // @step And "GitHub.com" is selected at index 0
            assert_eq!(*selected_index, 0, "GitHub.com must be selected at index 0");
        }
        other => panic!("expected OAuthDeploymentTypeSelect, got {other:?}"),
    }

    // @step And the screen shows "GitHub Copilot Login — Select deployment type"
    let buf = render_screen(&mut view);
    assert!(screen_contains(
        &buf,
        "GitHub Copilot Login — Select deployment type"
    ));
    assert!(screen_contains(&buf, "GitHub.com"));

    // @step When the user presses Enter
    let event = view.handle_key(key(KeyCode::Enter));

    // @step Then the backend copilot device-start is called with no enterprise host
    // (the view emits the device-start action with `enterprise_host: None`)
    match event {
        ProviderSettingsEvent::Emit(Action::OAuthCopilotDeviceStart {
            enterprise_host, ..
        }) => {
            assert_eq!(
                enterprise_host, None,
                "GitHub.com must start device flow with no enterprise host"
            );
        }
        other => panic!("expected Emit(OAuthCopilotDeviceStart {{ None }}), got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: GitHub Enterprise prompts for a host, normalizes it, and polls
// against it (view/key half — Down selects index 1, Enter enters
// enterprise-url-entry, typing the URL then Enter normalizes the host and
// emits the copilot device-start carrying the normalized host).
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn copilot_enterprise_prompts_normalizes_host_and_starts_device() {
    // @step Given the "github-copilot" provider is in oauth-deployment-type-select
    let mut view = view_in_deployment_select();

    // @step When the user presses Down
    view.handle_key(key(KeyCode::Down));

    // @step Then "GitHub Enterprise" is selected at index 1
    match &view.mode {
        ProviderSettingsMode::OAuthDeploymentTypeSelect { selected_index, .. } => {
            assert_eq!(*selected_index, 1, "GitHub Enterprise must be index 1");
        }
        other => panic!("expected OAuthDeploymentTypeSelect, got {other:?}"),
    }

    // @step When the user presses Enter
    view.handle_key(key(KeyCode::Enter));

    // @step Then the mode becomes oauth-enterprise-url-entry for provider "github-copilot"
    match &view.mode {
        ProviderSettingsMode::OAuthEnterpriseUrlEntry { provider_id, .. } => {
            assert_eq!(provider_id, "github-copilot");
        }
        other => panic!("expected OAuthEnterpriseUrlEntry, got {other:?}"),
    }

    // @step When the user types "https://company.ghe.com/"
    type_str(&mut view, "https://company.ghe.com/");

    // @step And the user presses Enter
    let event = view.handle_key(key(KeyCode::Enter));

    // @step Then the host is normalized to "company.ghe.com"
    // @step And the backend copilot device-start is called with enterprise host "company.ghe.com"
    match event {
        ProviderSettingsEvent::Emit(Action::OAuthCopilotDeviceStart {
            enterprise_host, ..
        }) => {
            assert_eq!(
                enterprise_host,
                Some("company.ghe.com".to_string()),
                "the scheme + trailing slash must be stripped to the bare host"
            );
        }
        other => panic!("expected Emit(OAuthCopilotDeviceStart {{ host }}), got {other:?}"),
    }

    // @step And the mode becomes oauth-device-waiting for provider "github-copilot"
    // (the view enters the shared device-waiting screen once the start is
    // emitted; the user code + URL are filled by the dispatch start-result)
    assert!(
        matches!(
            &view.mode,
            ProviderSettingsMode::OAuthDeviceWaiting { provider_id, .. }
                if provider_id == "github-copilot"
        ),
        "submitting the enterprise host must enter device-waiting, got {:?}",
        view.mode
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Submitting an empty enterprise URL shows a validation error
// (view/key — Enter on an empty input sets the validationError and stays in
// enterprise-url-entry with no backend call; typing a char clears the error).
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn empty_enterprise_url_shows_validation_error_then_clears_on_type() {
    // @step Given the "github-copilot" provider is in oauth-enterprise-url-entry with an empty URL input
    let mut view = view_in_enterprise_entry("", None);

    // @step When the user presses Enter
    let event = view.handle_key(key(KeyCode::Enter));

    // @step Then the screen shows the validation error "URL or domain is required"
    match &view.mode {
        ProviderSettingsMode::OAuthEnterpriseUrlEntry {
            validation_error, ..
        } => {
            assert_eq!(
                validation_error.as_deref(),
                Some("URL or domain is required"),
                "an empty submit must set the validation error"
            );
        }
        other => panic!("expected OAuthEnterpriseUrlEntry, got {other:?}"),
    }
    let buf = render_screen(&mut view);
    assert!(screen_contains(&buf, "URL or domain is required"));

    // @step And the mode is still oauth-enterprise-url-entry
    assert!(matches!(
        &view.mode,
        ProviderSettingsMode::OAuthEnterpriseUrlEntry { .. }
    ));

    // @step And no backend device-start is called
    // (a pure view test: an empty submit emits no action)
    assert!(
        matches!(event, ProviderSettingsEvent::Consumed),
        "an empty submit must NOT emit a device-start action, got {event:?}"
    );

    // @step When the user types "c"
    view.handle_key(char_key('c'));

    // @step Then the validation error is cleared
    match &view.mode {
        ProviderSettingsMode::OAuthEnterpriseUrlEntry {
            url_input,
            validation_error,
            ..
        } => {
            assert_eq!(url_input, "c", "the typed char must append to the input");
            assert_eq!(
                *validation_error, None,
                "typing must clear the validation error"
            );
        }
        other => panic!("expected OAuthEnterpriseUrlEntry, got {other:?}"),
    }

    // Rule [3]: only printable ASCII (32..=126) appends to urlInput; control /
    // non-ASCII chars are dropped. Build on the "c" already in the buffer.
    let non_ascii = 'é'; // U+00E9, code point > 126 — must be ignored
    view.handle_key(char_key(non_ascii));
    match &view.mode {
        ProviderSettingsMode::OAuthEnterpriseUrlEntry { url_input, .. } => {
            assert_eq!(
                url_input, "c",
                "a non-ASCII char (é) must NOT append to url_input"
            );
        }
        other => panic!("expected OAuthEnterpriseUrlEntry, got {other:?}"),
    }

    // A control char (Tab, U+0009) is likewise dropped.
    view.handle_key(char_key('\t'));
    match &view.mode {
        ProviderSettingsMode::OAuthEnterpriseUrlEntry { url_input, .. } => {
            assert_eq!(
                url_input, "c",
                "a control char (tab) must NOT append to url_input"
            );
        }
        other => panic!("expected OAuthEnterpriseUrlEntry, got {other:?}"),
    }

    // A printable ASCII char DOES append.
    view.handle_key(char_key('o'));
    match &view.mode {
        ProviderSettingsMode::OAuthEnterpriseUrlEntry { url_input, .. } => {
            assert_eq!(
                url_input, "co",
                "a printable ASCII char must append to url_input"
            );
        }
        other => panic!("expected OAuthEnterpriseUrlEntry, got {other:?}"),
    }
}
// Esc on deployment-type-select AND on enterprise-url-entry both return to the
// list with no backend call).
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn esc_cancels_both_copilot_preamble_modes_to_list() {
    // @step Given the "github-copilot" provider is in oauth-deployment-type-select
    let mut view = view_in_deployment_select();

    // @step When the user presses Esc
    let event = view.handle_key(key(KeyCode::Esc));

    // @step Then the mode returns to list
    assert!(matches!(view.mode, ProviderSettingsMode::List));

    // @step And no backend call is made
    assert!(
        matches!(event, ProviderSettingsEvent::Consumed),
        "Esc on deployment-select emits no action, got {event:?}"
    );

    // @step Given the "github-copilot" provider is in oauth-enterprise-url-entry
    let mut view = view_in_enterprise_entry("partial", None);

    // @step When the user presses Esc
    let event = view.handle_key(key(KeyCode::Esc));

    // @step Then the mode returns to list
    assert!(matches!(view.mode, ProviderSettingsMode::List));

    // @step And no backend call is made
    assert!(
        matches!(event, ProviderSettingsEvent::Consumed),
        "Esc on enterprise-url-entry emits no action, got {event:?}"
    );
}

// ════════════════════════ SECTION 2: DISPATCH ════════════════════════

async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

async fn wait_until<F: FnMut() -> bool>(mut predicate: F, label: &str) {
    timeout(Duration::from_secs(1), async {
        loop {
            if predicate() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for: {label}"));
}

fn fresh_app(mock: Arc<MockBackend>) -> App {
    let backend: Arc<dyn FspecBackend> = mock;
    App::new(backend)
}

fn provider_settings_mode(app: &App) -> ProviderSettingsMode {
    app.navigator().provider_settings.mode.clone()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario (dispatch half): GitHub.com device flow goes straight to
// device-waiting and connects. Driving `OAuthCopilotDeviceStart { None }`
// through `App::dispatch` must call the backend copilot device-start, fold the
// device-waiting screen (code + URL), and the poll success must fold to
// oauth-success with the GitHub Copilot banner.
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn github_dot_com_device_start_waits_then_connects() {
    // @step Given the "github-copilot" provider is expanded
    // @step And the cursor is on the "Login with GitHub Copilot (device flow)" row
    let mock = Arc::new(MockBackend::new());
    mock.seed_oauth_copilot_device_start(
        "WXYZ-9876",
        "https://github.com/login/device",
        "copilot-auth-1",
        1,
    );
    let mut app = fresh_app(mock.clone());

    // @step When the user presses Enter
    // @step Then the mode becomes oauth-deployment-type-select for provider "github-copilot"
    // @step And the screen shows "GitHub Copilot Login — Select deployment type"
    // @step And "GitHub.com" is selected at index 0
    // (the deployment-select preamble is exercised by the view test; here we
    // drive the GitHub.com branch's device-start through dispatch directly)

    // @step When the user presses Enter
    let gen = app.navigator().provider_settings.oauth_generation();
    app.dispatch(Action::OAuthCopilotDeviceStart {
        enterprise_host: None,
        generation: gen,
    });
    drain_pending(&mut app).await;

    // @step Then the backend copilot device-start is called with no enterprise host
    wait_until(
        || mock.oauth_copilot_device_start_calls() >= 1,
        "copilot device-start called",
    )
    .await;
    assert_eq!(mock.oauth_copilot_device_start_calls(), 1);
    assert_eq!(
        mock.oauth_copilot_device_start_hosts(),
        vec![None],
        "GitHub.com must pass no enterprise host"
    );

    // @step And the mode becomes oauth-device-waiting for provider "github-copilot"
    // @step And the screen shows the user code and verification URL
    let gen = app.navigator().provider_settings.oauth_generation();
    app.dispatch(Action::OAuthDeviceReady {
        provider_id: "github-copilot".to_string(),
        user_code: "WXYZ-9876".to_string(),
        verification_url: "https://github.com/login/device".to_string(),
        device_auth_id: "copilot-auth-1".to_string(),
        interval: 1,
        generation: gen,
    });
    match provider_settings_mode(&app) {
        ProviderSettingsMode::OAuthDeviceWaiting {
            provider_id,
            user_code,
            verification_url,
        } => {
            assert_eq!(provider_id, "github-copilot");
            assert_eq!(user_code, "WXYZ-9876");
            assert_eq!(verification_url, "https://github.com/login/device");
        }
        other => panic!("expected OAuthDeviceWaiting, got {other:?}"),
    }
    let view = &mut app.navigator_mut().provider_settings;
    let buf = render_screen(view);
    assert!(screen_contains(&buf, "WXYZ-9876"));
    assert!(screen_contains(&buf, "https://github.com/login/device"));

    // @step When the backend device-poll resolves with a credential
    drain_pending(&mut app).await;
    wait_until(
        || {
            matches!(
                provider_settings_mode(&app),
                ProviderSettingsMode::OAuthSuccess { .. }
            )
        },
        "device poll resolves to success",
    )
    .await;

    // @step Then the mode becomes oauth-success for provider "github-copilot"
    match provider_settings_mode(&app) {
        ProviderSettingsMode::OAuthSuccess { provider_id } => {
            assert_eq!(provider_id, "github-copilot");
        }
        other => panic!("expected OAuthSuccess, got {other:?}"),
    }

    // @step And the screen shows "✓ Connected to GitHub Copilot"
    let view = &mut app.navigator_mut().provider_settings;
    let buf = render_screen(view);
    assert!(screen_contains(&buf, "✓ Connected to GitHub Copilot"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A failed copilot device-start shows the error screen and retries
// on Enter (dispatch + view — the failed start folds to oauth-error, Enter
// retries the copilot login flow, Esc returns to list).
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_copilot_device_start_shows_error_then_retries_then_cancels() {
    // @step Given the "github-copilot" provider is in oauth-deployment-type-select
    let mock = Arc::new(MockBackend::new());
    mock.set_oauth_copilot_device_start_error("the device request was refused".to_string());
    let mut app = fresh_app(mock.clone());

    // @step When the user presses Enter
    // @step And the backend copilot device-start resolves with an error
    let gen = app.navigator().provider_settings.oauth_generation();
    app.dispatch(Action::OAuthCopilotDeviceStart {
        enterprise_host: None,
        generation: gen,
    });
    drain_pending(&mut app).await;
    wait_until(
        || {
            matches!(
                provider_settings_mode(&app),
                ProviderSettingsMode::OAuthError { .. }
            )
        },
        "copilot device-start fails to oauth-error",
    )
    .await;
    assert!(mock.oauth_copilot_device_start_calls() >= 1);

    // @step Then the mode becomes oauth-error for provider "github-copilot"
    match provider_settings_mode(&app) {
        ProviderSettingsMode::OAuthError { provider_id, error } => {
            assert_eq!(provider_id, "github-copilot");
            // @step And the screen shows the error message
            assert!(
                error.contains("the device request was refused"),
                "error must carry the UI-safe message, got {error:?}"
            );
            // No RPC/method name leaks into the error text.
            assert!(
                !error.contains("oauth_copilot_device_start"),
                "error must not leak the RPC/method name, got {error:?}"
            );
        }
        other => panic!("expected OAuthError, got {other:?}"),
    }

    // @step And the screen shows "OAuth Login error"
    let view = &mut app.navigator_mut().provider_settings;
    let buf = render_screen(view);
    assert!(screen_contains(&buf, "OAuth Login error"));

    // @step When the user presses Enter
    // @step Then the copilot login flow is retried
    let event = app
        .navigator_mut()
        .provider_settings
        .handle_key(key(KeyCode::Enter));
    match event {
        ProviderSettingsEvent::Emit(Action::OAuthCopilotDeviceStart { .. }) => {}
        other => panic!("expected retry Emit(OAuthCopilotDeviceStart), got {other:?}"),
    }

    // @step When the user presses Esc
    app.navigator_mut().provider_settings.mode = ProviderSettingsMode::OAuthError {
        provider_id: "github-copilot".to_string(),
        error: "again".to_string(),
    };
    app.navigator_mut()
        .provider_settings
        .handle_key(key(KeyCode::Esc));

    // @step Then the mode returns to list
    assert!(matches!(
        provider_settings_mode(&app),
        ProviderSettingsMode::List
    ));
}
