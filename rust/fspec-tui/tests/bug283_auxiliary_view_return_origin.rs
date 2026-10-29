//! Feature: spec/features/auxiliary-view-return-origin.feature

mod common;
#[path = "bug283_return_origin/context.rs"]
mod context;
#[path = "bug283_return_origin/fixture.rs"]
mod fixture;
#[path = "bug283_return_origin/local_escape.rs"]
mod local_escape;

use codelet_fspec_tui::{Action, ViewMode};
use crossterm::event::KeyCode;
use fixture::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn board_without_sessions_returns_from_providers() -> TestResult {
    for menu in [false, true] {
        // @step Given Board has a selected card and no sessions
        let mut fixture = Fixture::new()?;
        assert!(fixture.app.agent_view_store().current_session().is_none());
        let before = fixture.effects();
        // @step When I open Providers using the board key or menu and press Escape
        context::open_board_providers(&mut fixture.app, menu).await;
        key(&mut fixture.app, KeyCode::Esc).await;
        // @step Then the same Board selection returns without session or model changes
        assert_eq!(fixture.app.active_view(), ViewMode::Board);
        assert_eq!(fixture.app.board_store().focused_column(), "implementing");
        assert_eq!(
            fixture.app.board_store().selected_index_for("implementing"),
            1
        );
        assert!(screen(&mut fixture.app).contains("IMPLEMENTING"));
        assert_eq!(fixture.effects(), before);
        assert!(fixture.app.agent_view_store().open_sessions().is_empty());
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn latest_caller_replaces_a_previous_agent_visit() -> TestResult {
    // @step Given I previously visited Providers from Agent A and returned to Board
    let mut fixture = Fixture::new()?;
    fixture.caller(ViewMode::Agent).await;
    open(
        &mut fixture.app,
        Action::OpenProviderSettingsView,
        ViewMode::ProviderSettings,
    )
    .await;
    key(&mut fixture.app, KeyCode::Esc).await;
    assert_eq!(fixture.app.active_view(), ViewMode::Agent);
    fixture.app.dispatch(Action::BackToBoard);
    // @step When I open Providers again and press Escape
    key(&mut fixture.app, KeyCode::Char('P')).await;
    key(&mut fixture.app, KeyCode::Esc).await;
    // @step Then Board returns instead of the retained Agent A
    assert_eq!(fixture.app.active_view(), ViewMode::Board);
    assert_eq!(
        fixture.app.agent_view_store().current_session(),
        Some(&sid())
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn agent_dismissal_preserves_identity_draft_assignment_and_scroll() -> TestResult {
    // @step Given Agent A has an assignment draft and scrolled transcript
    let mut fixture = Fixture::new()?;
    fixture.caller(ViewMode::Agent).await;
    context::seed_agent(&mut fixture.app)?;
    let before = fixture.effects();
    for (action, _, mode) in auxiliary_actions().into_iter().take(3) {
        // @step When I open and dismiss Providers Models and Blocklist
        open(&mut fixture.app, action, mode).await;
        key(&mut fixture.app, KeyCode::Esc).await;
        // @step Then Agent A retains all context without implicit backend writes
        context::assert_agent(&fixture.app)?;
        assert_eq!(fixture.effects(), before);
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn mux_dismissal_preserves_layout_panes_and_focus() -> TestResult {
    // @step Given Mux has multiple panes a custom split and a focused pane
    let mut fixture = Fixture::new()?;
    fixture.caller(ViewMode::Mux).await;
    fixture.app.navigator_mut().mux.set_focus(2);
    let before = context::MuxSnapshot::capture(&fixture.app);
    let effects = fixture.effects();
    // @step When I open Providers from the mux menu and press Escape
    context::open_board_providers(&mut fixture.app, true).await;
    key(&mut fixture.app, KeyCode::Esc).await;
    // @step Then the same Mux layout panes window and focus return
    assert_eq!(fixture.app.active_view(), ViewMode::Mux);
    assert_eq!(context::MuxSnapshot::capture(&fixture.app), before);
    assert_eq!(fixture.effects(), effects);
    Ok(())
}

#[tokio::test]
#[serial]
async fn repeated_providers_and_models_switches_retain_the_excursion_caller() -> TestResult {
    for caller in [ViewMode::Board, ViewMode::Agent, ViewMode::Mux] {
        for end_in_models in [false, true] {
            // @step Given Providers is opened from Board Agent or Mux
            let mut fixture = Fixture::new()?;
            fixture.caller(caller).await;
            let before = fixture.effects();
            open(
                &mut fixture.app,
                Action::OpenProviderSettingsView,
                ViewMode::ProviderSettings,
            )
            .await;
            // @step When I Tab between Providers and Models repeatedly and Escape from either screen
            context::switch_siblings(&mut fixture.app, end_in_models).await;
            key(&mut fixture.app, KeyCode::Esc).await;
            // @step Then the original caller returns without changing sessions or models
            assert_eq!(fixture.app.active_view(), caller);
            assert_eq!(fixture.effects(), before);
        }
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn model_commitment_restores_the_caller_and_preserves_explicit_model_semantics() -> TestResult
{
    for caller in [ViewMode::Board, ViewMode::Agent, ViewMode::Mux] {
        // @step Given Models was reached through Providers from Board Agent or Mux with Agent A open
        let mut fixture = Fixture::new()?;
        fixture.caller(caller).await;
        let mut expected = fixture.effects();
        open(
            &mut fixture.app,
            Action::OpenProviderSettingsView,
            ViewMode::ProviderSettings,
        )
        .await;
        key(&mut fixture.app, KeyCode::Tab).await;
        assert_eq!(fixture.app.active_view(), ViewMode::ModelSelector);
        // @step When I commit a model for Agent A
        key(&mut fixture.app, KeyCode::Right).await;
        assert_eq!(fixture.app.active_view(), ViewMode::ModelSelector);
        key(&mut fixture.app, KeyCode::Down).await;
        assert_eq!(fixture.mock.set_session_model_calls(), expected[3]);
        key(&mut fixture.app, KeyCode::Enter).await;
        drain(&mut fixture.app).await;
        // @step Then the original caller returns and only the targeted session model is updated
        assert_eq!(fixture.app.active_view(), caller);
        assert_eq!(
            fixture.mock.last_set_session_model(),
            Some((sid(), "openai".into(), "chosen-model".into()))
        );
        expected[3] += 1;
        assert_eq!(fixture.effects(), expected);
        assert_eq!(
            fixture.app.agent_view_store().current_session(),
            Some(&sid())
        );
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn files_checkpoints_and_blocklist_restore_their_full_screen_caller() -> TestResult {
    for caller in [ViewMode::Board, ViewMode::Agent, ViewMode::Mux] {
        for (action, _, mode) in auxiliary_actions().into_iter().skip(2) {
            // @step Given Board Agent or Mux is the active caller
            let mut fixture = Fixture::new()?;
            fixture.caller(caller).await;
            let before = fixture.effects();
            // @step When I open and Escape from full-screen Files Checkpoints and Blocklist
            open(&mut fixture.app, action, mode).await;
            key(&mut fixture.app, KeyCode::Esc).await;
            // @step Then each dismissal restores that caller without session side effects
            assert_eq!(fixture.app.active_view(), caller, "closing {mode:?}");
            assert_eq!(fixture.effects(), before);
        }
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn local_escape_hierarchy_does_not_end_the_excursion_early() -> TestResult {
    // @step Given Providers or Models has an active filter detail form or confirmation
    let mut fixture = Fixture::new()?;
    open(
        &mut fixture.app,
        Action::OpenProviderSettingsView,
        ViewMode::ProviderSettings,
    )
    .await;
    // @step When I press Escape through the local hierarchy and finally dismiss the root screen
    local_escape::cancel_local_modes(&mut fixture.app).await;
    key(&mut fixture.app, KeyCode::Esc).await;
    // @step Then local cancellation stays in the screen and final dismissal restores Board
    assert_eq!(fixture.app.active_view(), ViewMode::Board);
    assert_eq!(fixture.mock.delete_provider_credentials_calls(), 0);
    assert_eq!(fixture.mock.set_provider_credentials_calls(), 0);
    assert_eq!(fixture.effects(), [0; 5]);
    Ok(())
}

#[tokio::test]
#[serial]
async fn mux_pane_escape_remains_local_after_an_auxiliary_excursion() -> TestResult {
    // @step Given Mux has loaded Files and Checkpoints panes after visiting Providers
    let mut fixture = Fixture::new()?;
    fixture.caller(ViewMode::Mux).await;
    let config = fixture.app.navigator().mux.config().clone();
    open(
        &mut fixture.app,
        Action::OpenProviderSettingsView,
        ViewMode::ProviderSettings,
    )
    .await;
    key(&mut fixture.app, KeyCode::Esc).await;
    assert_eq!(fixture.app.active_view(), ViewMode::Mux);
    // @step When I press Escape in each focused tool pane
    local_escape::close_mux_panes(&mut fixture.app).await?;
    // @step Then only the focused pane closes and the saved layout remains unchanged
    assert_eq!(fixture.app.navigator().mux.config(), &config);
    assert_eq!(fixture.app.navigator().mux.effective_panes().len(), 2);
    Ok(())
}

#[tokio::test]
#[serial]
async fn repeated_opens_and_inactive_closes_do_not_corrupt_the_origin() -> TestResult {
    for (action, close, mode) in auxiliary_actions() {
        // @step Given Board opened an auxiliary screen
        let mut fixture = Fixture::new()?;
        open(&mut fixture.app, action.clone(), mode).await;
        let before = fixture.effects();
        // @step When the same screen is reopened and mismatched or duplicate closes arrive
        fixture.app.dispatch(action.clone());
        fixture.app.dispatch(action);
        drain(&mut fixture.app).await;
        for (_, mismatched, other) in auxiliary_actions() {
            if other != mode {
                fixture.app.dispatch(mismatched);
                assert_eq!(fixture.app.active_view(), mode, "inactive {other:?} close");
            }
        }
        fixture.app.dispatch(close.clone());
        // @step Then inactive closes do nothing and the matching close returns to Board once
        assert_eq!(fixture.app.active_view(), ViewMode::Board);
        fixture.app.dispatch(close);
        assert_eq!(fixture.app.active_view(), ViewMode::Board);
        drain(&mut fixture.app).await;
        assert_eq!(fixture.effects(), before);
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn missing_origin_and_explicit_navigation_use_a_safe_fresh_caller() -> TestResult {
    for (_, close, mode) in auxiliary_actions() {
        // @step Given an auxiliary screen has no recorded caller or an earlier excursion was left explicitly
        let mut fixture = Fixture::new()?;
        fixture.app.navigator_mut().active_view = mode;
        fixture.app.navigator_mut().mux.config_mut().enabled = true;
        // @step When I dismiss the screen or open and dismiss another excursion with a saved mux preference
        fixture.app.dispatch(close);
        // @step Then missing origin lands on Board and new excursions restore only their live caller
        assert_eq!(fixture.app.active_view(), ViewMode::Board);
    }
    context::assert_explicit_navigation().await?;
    Ok(())
}
