//! TUI-132: the live paths that shared a removed variant's helper still
//! work, and the Provider Settings List footer offers only keys that act
//! (PS-15).
//!
//! Feature: spec/features/every-tui-action-is-reachable-from-the-ui.feature

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestResult};
use std::collections::HashSet;
use std::sync::Arc;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::views::provider_settings::nav_item::{NavItemKind, ProviderDisplayInfo};
use codelet_fspec_tui::views::ProviderSettingsView;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, SessionInfo};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

mod common;
use common::MockBackend;

fn session(id: &str) -> SessionInfo {
    SessionInfo {
        id: id.to_string(),
        name: id.to_string(),
        status: "idle".to_string(),
        project: String::new(),
        message_count: 0,
        provider_id: None,
        model_id: None,
        is_isolated: false,
        worktree_path: None,
        role: None,
        updated_at_ms: None,
        work_unit_focus: Default::default(),
    }
}

fn drain(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

fn press(app: &mut App, code: KeyCode) {
    let _ = app
        .navigator_mut()
        .agent
        .handle_event(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
}

/// Scenario: The resume view still deletes a session after its confirm
#[tokio::test]
async fn the_resume_view_still_deletes_a_session_after_its_confirm() -> TestResult {
    // @step Given the resume view lists the session "s-old"
    let mock = Arc::new(MockBackend::new());
    mock.seed_sessions(vec![session("s-old")]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SlashCommandSelected(
        app.current_session(),
        SlashCommandAction::Resume,
    ));
    let listing = app.next_pending_task().or_fail("the list_sessions task")?;
    listing.await.completed_or_cancelled()?;
    drain(&mut app);

    // @step When the user presses d on "s-old"
    press(&mut app, KeyCode::Char('d'));
    drain(&mut app);

    // @step Then the delete confirm opens
    let view = app
        .navigator()
        .agent
        .resume_view
        .as_ref()
        .or_fail("resume view")?;
    assert!(
        view.delete_confirm().is_some(),
        "d must open the delete confirm"
    );
    assert_eq!(mock.delete_session_calls(), 0);

    // @step When the user confirms the delete
    press(&mut app, KeyCode::Enter);
    drain(&mut app);
    let delete = app.next_pending_task().or_fail("the delete task")?;
    delete.await.completed_or_cancelled()?;

    // @step Then persistence_delete_session is called once for "s-old"
    assert_eq!(mock.delete_session_calls(), 1);
    assert_eq!(mock.last_deleted_session(), Some(SessionId::new("s-old")));
    Ok(())
}

fn anthropic(configured: bool) -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: "anthropic".to_string(),
        name: "Anthropic".to_string(),
        configured,
        credential_type: "api_key".to_string(),
        model_count: 8,
        has_oauth_tokens: false,
        is_oauth_provider: false,
        requires_api_key: true,
        env_var: Some("ANTHROPIC_API_KEY".to_string()),
        profiles: Vec::new(),
        oauth_login_methods: Vec::new(),
        oauth_status_label: None,
        masked_key: None,
        source: None,
    }
}

fn list(configured: bool, expanded: bool) -> ProviderSettingsView {
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![anthropic(configured)]);
    let mut ids = HashSet::new();
    if expanded {
        ids.insert("anthropic".to_string());
    }
    view.expanded = ids;
    view.rebuild_nav_items();
    view.selected_index = 0;
    view
}

fn focus_api_key(view: &mut ProviderSettingsView) -> TestResult {
    view.selected_index = view
        .nav_items
        .iter()
        .position(|n| matches!(n.kind, NavItemKind::ApiKey))
        .or_fail("an API key row")?;
    Ok(())
}

/// Scenario: The API key row offers delete only when the provider is configured
#[test]
fn the_api_key_row_offers_delete_only_when_the_provider_is_configured() -> TestResult {
    // @step Given Provider Settings lists the provider "anthropic" expanded with its API key row focused
    let mut unconfigured = list(false, true);
    focus_api_key(&mut unconfigured)?;
    let mut configured = list(true, true);
    focus_api_key(&mut configured)?;

    // @step When "anthropic" is not configured
    // @step Then the footer reads "Enter: edit · / filter · Tab: Switch to models · Esc: close"
    assert_eq!(
        unconfigured.footer_hint(),
        "Enter: edit · / filter · Tab: Switch to models · Esc: close"
    );

    // @step When "anthropic" is configured
    // @step Then the footer reads "Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close"
    assert_eq!(
        configured.footer_hint(),
        "Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close"
    );
    Ok(())
}

/// Scenario: A provider row advertises its arrow keys and delete
#[test]
fn a_provider_row_advertises_its_arrow_keys_and_delete() -> TestResult {
    // @step Given Provider Settings lists the configured provider "anthropic" collapsed and focused
    let mut view = list(true, false);

    // @step Then the footer reads "Enter/→: expand · d: delete · / filter · Tab: Switch to models · Esc: close"
    assert_eq!(
        view.footer_hint(),
        "Enter/→: expand · d: delete · / filter · Tab: Switch to models · Esc: close"
    );

    // @step When the user presses →
    let _ = view.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));

    // @step Then the footer reads "Enter/←: collapse · d: delete · / filter · Tab: Switch to models · Esc: close"
    assert_eq!(
        view.footer_hint(),
        "Enter/←: collapse · d: delete · / filter · Tab: Switch to models · Esc: close"
    );

    // @step And for an unconfigured collapsed provider row the footer reads "Enter/→: expand · / filter · Tab: Switch to models · Esc: close"
    assert_eq!(
        list(false, false).footer_hint(),
        "Enter/→: expand · / filter · Tab: Switch to models · Esc: close"
    );
    Ok(())
}
