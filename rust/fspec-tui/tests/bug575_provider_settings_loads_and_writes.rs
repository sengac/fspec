//! BUG-575: Provider Settings loads and writes never corrupt the view.
//!
//! Feature: spec/features/provider-settings-loads-and-writes-never-corrupt-the-view.feature
//!
//! The real `App` dispatch runs against `MockBackend` on a paused tokio
//! clock. MockBackend's call gates (`tests/common/call_gates.rs`) hold the
//! credential and settings reads, so a test decides when, and in which
//! order, each read answers. A gated read answers with the backend state at
//! the moment it was called, so reseeding the mock between two calls gives
//! each refresh its own snapshot. `pump_until` dispatches the actions that
//! have arrived until the asserted state holds (no sleeps; TEST-042).

use std::sync::Arc;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind;
use codelet_fspec_tui::views::provider_settings::ProviderSettingsMode;
use codelet_fspec_tui::views::ProviderSettingsView;
use codelet_fspec_tui::{App, FspecBackend};
use codelet_rpc_types::{
    ProfileDefinition, ProviderCredentialInfo, ProviderProfileConfig, ProviderSettingsConfig,
};
use codelet_test_helpers::{deadlock_guard, OrFail, TestError, TestResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

mod common;
use common::{MockBackend, TaskOutcome};

const CREDENTIALS: &str = "list_provider_credentials";
const SETTINGS: &str = "list_provider_settings";

fn credential(id: &str, credential_type: &str, masked_key: Option<&str>) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured: true,
        credential_type: credential_type.to_string(),
        model_count: 0,
        masked_key: masked_key.map(str::to_string),
        source: masked_key.map(|_| "file".to_string()),
    }
}

fn credentials() -> Vec<ProviderCredentialInfo> {
    vec![
        credential("anthropic", "api_key", Some("sk-…ant")),
        credential("codex", "oauth", None),
        credential("openai", "api_key", Some("sk-…oai")),
    ]
}

fn stored_settings() -> ProfileDefinition {
    ProfileDefinition {
        context_window: Some(200_000),
        max_output_tokens: Some(32_000),
        streaming: Some(true),
        auto_continue: Some(3),
        max_images: Some(5),
        loop_detection_enabled: Some(true),
        loop_detection_window: Some(12),
        ..ProfileDefinition::default()
    }
}

fn profile(name: &str) -> ProviderProfileConfig {
    ProviderProfileConfig {
        name: name.to_string(),
        definition: ProfileDefinition {
            base_url: "http://127.0.0.1:8080/v1".to_string(),
            context_window: Some(64_000),
            max_output_tokens: Some(8_000),
            ..ProfileDefinition::default()
        },
    }
}

/// anthropic with stored settings, openai with `profiles`.
fn settings(profiles: &[&str]) -> Vec<ProviderSettingsConfig> {
    vec![
        ProviderSettingsConfig {
            provider_id: "anthropic".to_string(),
            settings: stored_settings(),
            profiles: Vec::new(),
        },
        ProviderSettingsConfig {
            provider_id: "openai".to_string(),
            settings: ProfileDefinition::default(),
            profiles: profiles.iter().map(|name| profile(name)).collect(),
        },
    ]
}

fn mock(profiles: &[&str]) -> Result<Arc<MockBackend>, TestError> {
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(credentials());
    mock.seed_provider_settings(settings(profiles))
        .or_fail("seed settings")?;
    Ok(mock)
}

fn app(mock: &Arc<MockBackend>) -> App {
    let backend: Arc<dyn FspecBackend> = mock.clone();
    App::new(backend)
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

/// Press `code` and dispatch what it emits.
fn press(app: &mut App, code: KeyCode) {
    app.handle_event(&key(code));
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

/// Dispatch arriving actions until `ready` holds.
async fn pump_until(
    app: &mut App,
    what: &str,
    mut ready: impl FnMut(&App) -> bool,
) -> Result<(), TestError> {
    deadlock_guard(what, async {
        loop {
            while let Some(action) = app.try_recv_action() {
                app.dispatch(action);
            }
            if ready(app) {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

/// Finish every task and dispatch everything (no gate may be holding).
async fn drain(app: &mut App) -> Result<(), TestError> {
    deadlock_guard("every task to finish", async {
        loop {
            while let Some(handle) = app.next_pending_task() {
                handle.await.completed_or_cancelled()?;
            }
            let Some(action) = app.try_recv_action() else {
                return Ok::<(), TestError>(());
            };
            app.dispatch(action);
        }
    })
    .await??;
    Ok(())
}

fn view(app: &App) -> &ProviderSettingsView {
    &app.navigator().provider_settings
}

/// Open /provider and let its first refresh answer.
async fn open(app: &mut App) -> Result<(), TestError> {
    app.dispatch(Action::OpenProviderSettingsView);
    drain(app).await
}

/// Move the cursor onto the row `matches` picks.
fn focus(
    app: &mut App,
    matches: impl Fn(&codelet_fspec_tui::views::NavItem) -> bool,
) -> TestResult {
    let index = view(app)
        .nav_items
        .iter()
        .position(matches)
        .or_fail("the row to focus")?;
    app.navigator_mut().provider_settings.selected_index = index;
    Ok(())
}

fn expand(app: &mut App, provider: &str) -> TestResult {
    focus(app, |item| {
        item.provider_id == provider && matches!(item.kind, NavItemKind::Provider { .. })
    })?;
    press(app, KeyCode::Right);
    Ok(())
}

fn profile_names(view: &ProviderSettingsView) -> Vec<String> {
    view.nav_items
        .iter()
        .filter_map(|item| match &item.kind {
            NavItemKind::Profile { profile_name } => {
                profile_name.split(" → ").next().map(str::to_string)
            }
            _ => None,
        })
        .collect()
}

fn focused(view: &ProviderSettingsView) -> Option<(String, NavItemKind)> {
    view.focused_nav_item()
        .map(|item| (item.provider_id.clone(), item.kind.clone()))
}

fn screen(app: &mut App) -> (String, Buffer) {
    let area = Rect::new(0, 0, 100, 30);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    let text = (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    (text, buffer)
}

/// The foreground colour of the first cell of `needle` on screen.
fn colour_of(app: &mut App, needle: &str) -> Result<Color, TestError> {
    let (text, buffer) = screen(app);
    let (y, line) = text
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains(needle))
        .or_fail(&format!("{needle} on screen"))?;
    let column = line
        .char_indices()
        .position(|(at, _)| line[at..].starts_with(needle))
        .or_fail("column")?;
    let x = u16::try_from(column)?;
    let y = u16::try_from(y)?;
    Ok(buffer[(x, y)].fg)
}

#[tokio::test(start_paused = true)]
async fn the_settings_form_does_not_open_before_the_settings_have_loaded() -> TestResult {
    // @step Given anthropic has stored settings and the settings read has not answered yet
    let mock = mock(&[])?;
    let mut app = app(&mock);
    mock.gates.gate(SETTINGS);

    // @step When I open /provider, expand anthropic and press Enter on Settings
    app.dispatch(Action::OpenProviderSettingsView);
    pump_until(&mut app, "the settings read to be held", |_| {
        mock.gates.held(SETTINGS) == 1
    })
    .await?;
    let rows_shown = !view(&app).nav_items.is_empty();
    if rows_shown {
        expand(&mut app, "anthropic")?;
        focus(&mut app, |item| {
            item.provider_id == "anthropic" && item.kind == NavItemKind::Settings
        })?;
        press(&mut app, KeyCode::Enter);
    }

    // @step Then no settings form opens and the status says the settings are still loading
    assert!(
        !matches!(view(&app).mode, ProviderSettingsMode::EditSettings { .. }),
        "a settings form opened before the settings loaded"
    );
    let (text, _) = screen(&mut app);
    assert!(text.contains("Loading"), "{text}");

    // @step And once the settings have loaded, Enter opens the form with the stored values
    mock.gates.open(SETTINGS);
    pump_until(&mut app, "the tree with Settings rows", |app| {
        view(app)
            .nav_items
            .iter()
            .any(|item| item.provider_id == "anthropic")
    })
    .await?;
    drain(&mut app).await?;
    expand(&mut app, "anthropic")?;
    focus(&mut app, |item| {
        item.provider_id == "anthropic" && item.kind == NavItemKind::Settings
    })?;
    press(&mut app, KeyCode::Enter);
    assert!(matches!(
        view(&app).mode,
        ProviderSettingsMode::EditSettings { .. }
    ));

    // @step And saving one changed field keeps every other stored setting
    press(&mut app, KeyCode::Char('0'));
    press(&mut app, KeyCode::Enter);
    drain(&mut app).await?;
    let saved = mock
        .provider_settings_now()
        .into_iter()
        .find(|config| config.provider_id == "anthropic")
        .or_fail("anthropic settings")?
        .settings;
    let stored = stored_settings();
    assert_eq!(saved.context_window, Some(2_000_000));
    assert_eq!(saved.max_output_tokens, stored.max_output_tokens);
    assert_eq!(saved.streaming, stored.streaming);
    assert_eq!(saved.auto_continue, stored.auto_continue);
    assert_eq!(saved.max_images, stored.max_images);
    assert_eq!(saved.loop_detection_enabled, stored.loop_detection_enabled);
    assert_eq!(saved.loop_detection_window, stored.loop_detection_window);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn settings_that_could_not_be_read_cannot_be_edited() -> TestResult {
    // @step Given the settings read fails
    let mock = mock(&[])?;
    mock.fail_list_provider_settings("settings file unreadable");
    let mut app = app(&mock);

    // @step When I open /provider, expand anthropic and press Enter on Settings
    open(&mut app).await?;
    expand(&mut app, "anthropic")?;
    focus(&mut app, |item| {
        item.provider_id == "anthropic" && item.kind == NavItemKind::Settings
    })?;
    press(&mut app, KeyCode::Enter);

    // @step Then no settings form opens and the status says the settings could not be loaded
    assert!(!matches!(
        view(&app).mode,
        ProviderSettingsMode::EditSettings { .. }
    ));
    let status = &view(&app).status;
    assert!(status.contains("could not be loaded"), "{status}");
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn refreshes_that_finish_in_reverse_order_leave_the_newest_state() -> TestResult {
    // @step Given profiles p1 and p2 are listed under openai
    let mock = mock(&["p1", "p2"])?;
    let mut app = app(&mock);
    open(&mut app).await?;
    expand(&mut app, "openai")?;
    assert_eq!(profile_names(view(&app)), vec!["p1", "p2"]);
    mock.gates.gate(SETTINGS);

    // @step When profile p1 is saved and profile p2 is deleted, and the delete's refresh answers before the save's
    app.dispatch(Action::SaveProfile {
        provider_id: "openai".to_string(),
        profile_name: "p1".to_string(),
        old_profile_name: Some("p1".to_string()),
        definition: profile("p1").definition,
    });
    pump_until(&mut app, "the save's settings read", |_| {
        mock.gates.held(SETTINGS) == 1
    })
    .await?;
    mock.seed_provider_settings(settings(&["p1"]))
        .or_fail("seed")?;
    app.dispatch(Action::ConfirmDeleteProfile {
        provider_id: "openai".to_string(),
        profile_name: "p2".to_string(),
    });
    pump_until(&mut app, "the delete's settings read", |_| {
        mock.gates.held(SETTINGS) == 2
    })
    .await?;
    mock.gates.release(SETTINGS, 1);
    pump_until(&mut app, "the delete's refresh", |app| {
        profile_names(view(app)) == vec!["p1"]
    })
    .await?;
    mock.gates.open(SETTINGS);
    drain(&mut app).await?;

    // @step Then p2 is not listed and p1 is listed
    assert_eq!(profile_names(view(&app)), vec!["p1"]);
    Ok(())
}

fn on_codex_logout(view: &ProviderSettingsView) -> bool {
    matches!(focused(view), Some((provider, NavItemKind::OAuthStatus { .. })) if provider == "codex")
}

fn on_codex_provider(view: &ProviderSettingsView) -> bool {
    matches!(focused(view), Some((provider, NavItemKind::Provider { .. })) if provider == "codex")
}

fn anthropic_key(view: &ProviderSettingsView) -> Option<String> {
    view.providers
        .iter()
        .find(|provider| provider.provider_id == "anthropic")
        .and_then(|provider| provider.masked_key.clone())
}

#[tokio::test(start_paused = true)]
async fn an_earlier_refresh_does_not_take_the_return_to_the_provider_row() -> TestResult {
    // @step Given I am on the Codex logout row while a refresh from saving a key is still running
    let mock = mock(&[])?;
    let mut app = app(&mock);
    open(&mut app).await?;
    expand(&mut app, "codex")?;
    focus(&mut app, |item| {
        item.provider_id == "codex" && matches!(item.kind, NavItemKind::OAuthStatus { .. })
    })?;
    // The save's refresh reads the new key, so its arrival is visible.
    let mut saved = credentials();
    saved[0].masked_key = Some("sk-…new".to_string());
    mock.set_post_save_provider_credentials(saved);
    mock.gates.gate(CREDENTIALS);
    app.dispatch(Action::SaveProviderCredentials {
        provider_id: "anthropic".to_string(),
        api_key: "sk-ant-new".to_string(),
    });
    pump_until(&mut app, "the save's credential read", |_| {
        mock.gates.held(CREDENTIALS) == 1
    })
    .await?;

    // @step When I log out of Codex and the earlier refresh answers first
    app.dispatch(Action::OAuthDisconnect {
        provider_id: "codex".to_string(),
    });
    pump_until(&mut app, "the logout's credential read", |_| {
        mock.gates.held(CREDENTIALS) == 2
    })
    .await?;
    mock.gates.release(CREDENTIALS, 0);
    pump_until(&mut app, "the save's refresh to be shown", |app| {
        anthropic_key(view(app)).as_deref() == Some("sk-…new")
    })
    .await?;

    // @step Then the earlier refresh leaves the cursor where it was
    assert!(on_codex_logout(view(&app)), "{:?}", focused(view(&app)));

    // @step And after the logout refresh the cursor is on the Codex provider row
    mock.gates.open(CREDENTIALS);
    drain(&mut app).await?;
    assert!(on_codex_provider(view(&app)), "{:?}", focused(view(&app)));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn moving_the_cursor_during_a_logout_keeps_it_where_i_put_it() -> TestResult {
    // @step Given I log out of Codex
    let mock = mock(&[])?;
    let mut app = app(&mock);
    open(&mut app).await?;
    expand(&mut app, "codex")?;
    focus(&mut app, |item| {
        item.provider_id == "codex" && matches!(item.kind, NavItemKind::OAuthStatus { .. })
    })?;
    mock.gates.gate(CREDENTIALS);
    app.dispatch(Action::OAuthDisconnect {
        provider_id: "codex".to_string(),
    });
    pump_until(&mut app, "the logout's credential read", |_| {
        mock.gates.held(CREDENTIALS) == 1
    })
    .await?;

    // @step When I move the cursor before the logout refresh answers
    focus(&mut app, |item| {
        item.provider_id == "openai" && matches!(item.kind, NavItemKind::Provider { .. })
    })?;
    press(&mut app, KeyCode::Up);
    let moved_to = focused(view(&app));
    mock.gates.open(CREDENTIALS);
    drain(&mut app).await?;

    // @step Then the cursor stays on the row I moved to
    assert_eq!(focused(view(&app)), moved_to);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn renaming_a_profile_keeps_the_cursor_on_it() -> TestResult {
    // @step Given profiles a, local and m are listed under openai and the cursor is on local
    let mock = mock(&["a", "local", "m"])?;
    let mut app = app(&mock);
    open(&mut app).await?;
    expand(&mut app, "openai")?;
    focus(
        &mut app,
        |item| matches!(&item.kind, NavItemKind::Profile { profile_name } if profile_name.starts_with("local ")),
    )?;

    // @step When local is renamed to zlocal and the refresh answers
    mock.seed_provider_settings(settings(&["a", "m", "zlocal"]))
        .or_fail("seed")?;
    app.dispatch(Action::SaveProfile {
        provider_id: "openai".to_string(),
        profile_name: "zlocal".to_string(),
        old_profile_name: Some("local".to_string()),
        definition: profile("zlocal").definition,
    });
    drain(&mut app).await?;

    // @step Then the cursor is on zlocal
    let on = focused(view(&app));
    assert!(
        matches!(&on, Some((_, NavItemKind::Profile { profile_name })) if profile_name.starts_with("zlocal ")),
        "{on:?}"
    );
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_reload_that_shrinks_the_tree_keeps_the_cursor_inside_it() -> TestResult {
    // @step Given the cursor is on the last of many rows
    let names: Vec<String> = (0..30).map(|n| format!("p{n:02}")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let mock = mock(&refs)?;
    let mut app = app(&mock);
    open(&mut app).await?;
    expand(&mut app, "openai")?;
    press(&mut app, KeyCode::End);
    let last = view(&app).nav_items.len() - 1;
    assert_eq!(view(&app).selected_index, last);

    // @step When a refresh answers with a much shorter tree
    mock.seed_provider_settings(settings(&[])).or_fail("seed")?;
    app.dispatch(Action::SaveProviderCredentials {
        provider_id: "anthropic".to_string(),
        api_key: "sk-ant-new".to_string(),
    });
    drain(&mut app).await?;

    // @step Then the cursor is on the last row and the view draws without error
    let view = view(&app);
    assert_eq!(view.selected_index, view.nav_items.len() - 1);
    assert!(view.focused_nav_item().is_some());
    let _ = screen(&mut app);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn an_oauth_answer_from_an_earlier_visit_is_ignored() -> TestResult {
    // @step Given I started a GitHub Copilot login and pressed Esc twice
    let mock = mock(&[])?;
    let mut app = app(&mock);
    open(&mut app).await?;
    // The device start is in flight on the waiting screen with this number.
    let stale = view(&app).oauth_generation();
    app.navigator_mut().provider_settings.mode = ProviderSettingsMode::OAuthDeviceWaiting {
        provider_id: "github-copilot".to_string(),
        user_code: String::new(),
        verification_url: String::new(),
    };
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Esc);

    // @step When I open /provider again and the old device start answers
    open(&mut app).await?;
    app.dispatch(Action::OAuthDeviceReady {
        provider_id: "github-copilot".to_string(),
        user_code: "OLD-CODE".to_string(),
        verification_url: "https://github.com/login/device".to_string(),
        device_auth_id: "old".to_string(),
        interval: 5,
        generation: stale,
    });
    drain(&mut app).await?;

    // @step Then the view stays in the provider list and no device poll starts
    assert!(matches!(view(&app).mode, ProviderSettingsMode::List));
    assert_eq!(mock.oauth_device_poll_calls(), 0);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_saved_message_is_not_shown_in_red() -> TestResult {
    // @step Given profile b's form is open
    let mock = mock(&["a", "b"])?;
    let mut app = app(&mock);
    open(&mut app).await?;
    expand(&mut app, "openai")?;
    focus(
        &mut app,
        |item| matches!(&item.kind, NavItemKind::Profile { profile_name } if profile_name.starts_with("b ")),
    )?;
    press(&mut app, KeyCode::Enter);
    assert!(matches!(
        view(&app).mode,
        ProviderSettingsMode::EditProfile { .. }
    ));

    // @step When the status "✓ openai: a profile saved" arrives
    app.dispatch(Action::ProviderSettingsStatus(
        "✓ openai: a profile saved".to_string(),
    ));

    // @step Then the status is drawn in green, not red
    assert_eq!(
        colour_of(&mut app, "✓ openai: a profile saved")?,
        Color::Green
    );
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn leaving_a_form_clears_its_hint() -> TestResult {
    // @step Given the profile form shows a hint about a bad Max Images value
    let mock = mock(&["b"])?;
    let mut app = app(&mock);
    open(&mut app).await?;
    expand(&mut app, "openai")?;
    focus(&mut app, |item| {
        matches!(item.kind, NavItemKind::Profile { .. })
    })?;
    press(&mut app, KeyCode::Enter);
    app.dispatch(Action::ProviderSettingsStatus(
        "Max Images must be a whole number".to_string(),
    ));

    // @step When I press Esc
    press(&mut app, KeyCode::Esc);

    // @step Then the provider list shows no status
    assert!(matches!(view(&app).mode, ProviderSettingsMode::List));
    assert_eq!(view(&app).status, "");
    let (text, _) = screen(&mut app);
    assert!(!text.contains("Max Images"), "{text}");
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn loading_and_failed_loads_are_not_shown_as_no_providers() -> TestResult {
    // @step Given the first refresh has not answered
    let mock = mock(&[])?;
    let mut app = app(&mock);
    mock.gates.gate(CREDENTIALS);
    app.dispatch(Action::OpenProviderSettingsView);
    pump_until(&mut app, "the credential read", |_| {
        mock.gates.held(CREDENTIALS) == 1
    })
    .await?;

    // @step When the view is drawn
    let (text, _) = screen(&mut app);

    // @step Then it says "Loading providers…"
    assert!(text.contains("Loading providers…"), "{text}");
    assert!(!text.contains("no providers configured"), "{text}");

    // @step And when the refresh fails it says the providers could not be loaded and why
    mock.set_list_provider_credentials_error("keyring locked".to_string());
    app.dispatch(Action::CloseProviderSettingsView);
    mock.gates.open(CREDENTIALS);
    drain(&mut app).await?;
    open(&mut app).await?;
    let (text, _) = screen(&mut app);
    assert!(text.contains("could not be loaded"), "{text}");
    assert!(text.contains("keyring locked"), "{text}");
    assert!(!text.contains("no providers configured"), "{text}");
    Ok(())
}
