//! TUI-129 menu scenarios: row keys in the board and mux dropdowns and the
//! menu-bar help dialog, the Help row's text, and the mux Daemons and
//! Workspaces rows.
//!
//! Feature: spec/features/advertised-keys-do-what-their-hints-say.feature

use codelet_fspec_tui::{Action, MuxPaneKind, ViewMode};
use codelet_test_helpers::TestResult;
use crossterm::event::KeyCode;

use super::fixture::*;

const MENU_BAR_HELP_ID: &str = "board-actions-dialog";
/// MENU-008: Kanban, Tools, Settings, Help.
const TOOLS: usize = 1;
const SETTINGS: usize = 2;

/// Scenario: A row's key runs that row while a board dropdown is open
#[tokio::test(start_paused = true)]
async fn a_rows_key_runs_that_row_while_a_board_dropdown_is_open() -> TestResult {
    // @step Given the board's Tools dropdown is open
    let (mut app, _mock) = fresh_app();
    select_card(&mut app, unit("TUI-001", &[])).await?;
    app.dispatch(Action::MenuOpenDropdown(TOOLS));
    drain(&mut app).await?;
    assert!(app.board_store().open_menu().is_some());

    // @step When the user presses "F"
    type_char(&mut app, 'F').await?;

    // @step Then the Changed Files view is shown
    assert_eq!(app.navigator().active_view, ViewMode::ChangedFiles);

    // @step And no dropdown is open
    assert_eq!(app.board_store().open_menu(), None);
    Ok(())
}

/// Scenario: A row's key runs that row while a mux bar dropdown is open
#[tokio::test(start_paused = true)]
async fn a_rows_key_runs_that_row_while_a_mux_bar_dropdown_is_open() -> TestResult {
    // @step Given the mux is shown and its Settings dropdown is open
    let (mut app, _mock) = fresh_app();
    open_sessions(&mut app, 1).await?;
    enable_mux(&mut app, vec![MuxPaneKind::Board, MuxPaneKind::Agent]);
    app.dispatch(Action::MenuOpenDropdown(SETTINGS));
    drain(&mut app).await?;
    assert!(app.navigator().mux.open_menu().is_some());

    // @step When the user presses "p"
    type_char(&mut app, 'p').await?;

    // @step Then the Provider Settings view is shown
    assert_eq!(app.navigator().active_view, ViewMode::ProviderSettings);
    Ok(())
}

/// Scenario: A row's key runs that row in the menu-bar help dialog
#[tokio::test(start_paused = true)]
async fn a_rows_key_runs_that_row_in_the_menu_bar_help_dialog() -> TestResult {
    // @step Given the menu-bar help dialog is open over the board
    let (mut app, _mock) = fresh_app();
    select_card(&mut app, unit("TUI-001", &[])).await?;
    type_char(&mut app, 'u').await?;
    assert!(app.compositor().contains(MENU_BAR_HELP_ID));

    // @step When the user presses "C"
    type_char(&mut app, 'C').await?;

    // @step Then the menu-bar help dialog is closed
    assert!(!app.compositor().contains(MENU_BAR_HELP_ID));

    // @step And the Checkpoints view is shown
    assert_eq!(app.navigator().active_view, ViewMode::Checkpoints);
    Ok(())
}

/// Scenario: The Help row says it shows the board keybindings
#[tokio::test(start_paused = true)]
async fn the_help_row_says_it_shows_the_board_keybindings() -> TestResult {
    let (mut app, _mock) = fresh_app();
    type_char(&mut app, 'u').await?;

    // @step When the menu-bar help dialog is rendered
    let rows = screen(&mut app, 120, 40)?;

    // @step Then it shows "? Help - Show the board keybindings"
    assert!(
        shows(&rows, "? Help - Show the board keybindings"),
        "{}",
        rows.join("\n")
    );
    Ok(())
}

/// Scenario: Mux Daemons and Workspaces rows show their dropdown on the Board pane
#[tokio::test(start_paused = true)]
async fn mux_daemons_and_workspaces_rows_show_their_dropdown_on_the_board_pane() -> TestResult {
    // @step Given the mux shows Board and Agent with the Agent pane focused
    let (mut app, _mock) = fresh_app();
    open_sessions(&mut app, 1).await?;
    enable_mux(&mut app, vec![MuxPaneKind::Board, MuxPaneKind::Agent]);
    app.navigator_mut().mux.set_focus(1);

    // @step When the user runs Settings > Workspaces
    app.dispatch(Action::MenuOpenDropdown(SETTINGS));
    drain(&mut app).await?;
    for _ in 0..3 {
        press(&mut app, KeyCode::Down).await?;
    }
    press(&mut app, KeyCode::Enter).await?;

    // @step Then the Board pane is focused and the workspace dropdown is open
    assert_eq!(app.navigator().mux.focus(), 0, "the Board pane is pane 0");
    assert!(app.board_store().workspace_dropdown().is_some());

    // @step And Esc closes the workspace dropdown
    press(&mut app, KeyCode::Esc).await?;
    assert_eq!(app.board_store().workspace_dropdown(), None);

    // @step Given the mux shows two Agent panes and no Board pane
    let (mut app, _mock) = fresh_app();
    open_sessions(&mut app, 2).await?;
    enable_mux(&mut app, vec![MuxPaneKind::Agent, MuxPaneKind::Agent]);
    assert!(!app
        .navigator()
        .mux
        .effective_panes()
        .contains(&MuxPaneKind::Board));

    // @step When the user runs Settings > Daemons
    app.dispatch(Action::MenuOpenDropdown(SETTINGS));
    drain(&mut app).await?;
    for _ in 0..2 {
        press(&mut app, KeyCode::Down).await?;
    }
    press(&mut app, KeyCode::Enter).await?;

    // @step Then a notice says the Board pane is needed
    assert!(app.compositor().contains(NOTICE_DIALOG_ID));
    let rows = screen(&mut app, 120, 30)?;
    assert!(shows(&rows, "Board pane"), "{}", rows.join("\n"));

    // @step And no daemon switcher is open
    assert_eq!(app.board_store().daemon_switcher(), None);
    Ok(())
}
