//! TUI-124 — closing a bot closes its sub-agents (product owner rule,
//! 2026-11-07): what the dialog says first, the close order, and the
//! all-or-nothing outcome for each session.
//!
//! Feature: spec/features/agent-close-and-start-dialogs-say-exactly-what-will-happen.feature

use codelet_fspec_tui::views::multiplex::MuxPaneKind;
use codelet_fspec_tui::{Action, ViewMode};
use codelet_rpc_types::SessionStatus;
use codelet_test_helpers::TestResult;
use crossterm::event::KeyCode;

use super::{
    activate_close, assert_shows, drain, fresh_app, make_bot, open, open_ids, press, screen, sid,
    CLOSE_DIALOG_ID,
};

/// Scenario: Esc on an idle bot opens the Close Bot dialog
#[tokio::test(start_paused = true)]
async fn esc_on_an_idle_bot_opens_the_close_bot_dialog() -> TestResult {
    // @step Given an idle bot with an empty input and no sub-agents is focused
    let (mut app, _mock) = fresh_app();
    open(&mut app, &["bot-1"], "bot-1").await?;
    make_bot(&mut app, "bot-1").await?;
    // @step When the user presses Esc
    press(&mut app, KeyCode::Esc).await?;
    let text = screen(&mut app);
    // @step Then the dialog titled "Close Bot?" says "Closing ends this bot and returns you to the board."
    assert_shows(&text, "Close Bot?")?;
    assert_shows(&text, "Closing ends this bot and returns you to the board.")?;
    // @step And it says "It has no sub-agents to close."
    assert_shows(&text, "It has no sub-agents to close.")
}

/// Scenario: Closing a bot closes its sub-agents and counts the running ones first
#[tokio::test(start_paused = true)]
async fn closing_a_bot_closes_its_sub_agents_and_counts_the_running_ones_first() -> TestResult {
    // @step Given a bot has three sub-agents, one running, one idle and one not open in this TUI
    let (mut app, mock) = fresh_app();
    open(&mut app, &["sub-1", "sub-2", "bot-1"], "bot-1").await?;
    make_bot(&mut app, "bot-1").await?;
    app.agent_view_store_mut()
        .set_session_status(sid("sub-1"), SessionStatus::Running);
    mock.seed_subordinates_for(sid("bot-1"), vec![sid("sub-1"), sid("sub-2"), sid("sub-3")]);
    app.board_store_mut()
        .attach_session("AUTH-001", sid("bot-1"));
    app.board_store_mut()
        .attach_session("AUTH-002", sid("sub-1"));
    // @step When the user activates "[ Close ]"
    activate_close(&mut app).await?;
    let text = screen(&mut app);
    // @step Then the dialog titled "Close Bot?" says "Its 3 sub-agents will be closed too (1 running, 1 status unknown)."
    assert_shows(&text, "Close Bot?")?;
    assert_shows(
        &text,
        "Its 3 sub-agents will be closed too (1 running, 1 status unknown).",
    )?;
    // @step And it says "All their history is kept: resume any of them later with /resume."
    assert_shows(
        &text,
        "All their history is kept: resume any of them later with /resume.",
    )?;
    // @step And its options are "Stop and Close All" and "Cancel"
    assert_shows(&text, " Stop and Close All    Cancel ")?;
    assert!(mock.destroyed_sessions().is_empty(), "nothing closes first");
    // @step When the user confirms "Stop and Close All"
    press(&mut app, KeyCode::Enter).await?;
    // @step Then the three sub-agents are closed before the bot
    assert_eq!(
        mock.destroyed_sessions(),
        vec![sid("sub-1"), sid("sub-2"), sid("sub-3"), sid("bot-1")]
    );
    // @step And the bot and its open sub-agents have no tabs and no work-unit links
    assert!(open_ids(&app).is_empty(), "open: {:?}", open_ids(&app));
    assert_eq!(app.board_store().session_for("AUTH-001"), None);
    assert_eq!(app.board_store().session_for("AUTH-002"), None);
    assert!(!app.compositor().contains(CLOSE_DIALOG_ID));
    Ok(())
}

/// Scenario: A sub-agent that is already closed is neither counted nor closed
#[tokio::test(start_paused = true)]
async fn a_sub_agent_that_is_already_closed_is_neither_counted_nor_closed() -> TestResult {
    // @step Given a bot has two sub-agents and the first one was already closed
    let (mut app, mock) = fresh_app();
    open(&mut app, &["bot-1", "sub-2", "sub-1"], "sub-1").await?;
    make_bot(&mut app, "bot-1").await?;
    mock.seed_subordinates_for(sid("bot-1"), vec![sid("sub-1"), sid("sub-2")]);
    mock.seed_supervisors_for(sid("sub-1"), vec![sid("bot-1")]);
    activate_close(&mut app).await?;
    press(&mut app, KeyCode::Enter).await?;
    assert_eq!(mock.destroyed_sessions(), vec![sid("sub-1")]);
    // @step When the user activates "[ Close ]" on the bot
    app.dispatch(Action::OpenAgentView(Some(sid("bot-1"))));
    drain(&mut app).await?;
    activate_close(&mut app).await?;
    // @step Then the dialog says "Its 1 sub-agent will be closed too (none running)."
    assert_shows(
        &screen(&mut app),
        "Its 1 sub-agent will be closed too (none running).",
    )?;
    // @step And confirming closes only the second sub-agent and then the bot
    press(&mut app, KeyCode::Enter).await?;
    assert_eq!(
        mock.destroyed_sessions(),
        vec![sid("sub-1"), sid("sub-2"), sid("bot-1")]
    );
    Ok(())
}

/// Scenario: A sub-agent open in a mux pane closes with its bot
#[tokio::test(start_paused = true)]
async fn a_sub_agent_open_in_a_mux_pane_closes_with_its_bot() -> TestResult {
    // @step Given the mux shows a bot and its sub-agent in two agent panes
    let (mut app, mock) = fresh_app();
    open(&mut app, &["bot-1", "sub-1"], "bot-1").await?;
    make_bot(&mut app, "bot-1").await?;
    mock.seed_subordinates_for(sid("bot-1"), vec![sid("sub-1")]);
    app.dispatch(Action::InputSubmitted(sid("bot-1"), "/mux on".to_string()));
    drain(&mut app).await?;
    assert_eq!(app.active_view(), ViewMode::Mux);
    let panes = app.navigator().mux.pane_rects().len();
    app.navigator_mut().mux.set_focus(panes - 1);
    app.navigator_mut()
        .mux
        .focus_agent_pane_for_session(&sid("bot-1"));
    // @step When the bot is closed with "Close All"
    press(&mut app, KeyCode::Esc).await?;
    let text = screen(&mut app);
    assert_shows(&text, "Close Bot?")?;
    assert_shows(&text, " Close All    Cancel ")?;
    press(&mut app, KeyCode::Enter).await?;
    // @step Then both sessions are closed and the mux stays on with the board pane focused
    assert_eq!(mock.destroyed_sessions(), vec![sid("sub-1"), sid("bot-1")]);
    assert!(open_ids(&app).is_empty(), "open: {:?}", open_ids(&app));
    assert_eq!(app.active_view(), ViewMode::Mux);
    assert!(app.navigator().mux.config().enabled);
    let focused = app.navigator().mux.effective_panes()[app.navigator().mux.focus()];
    assert_eq!(focused, MuxPaneKind::Board);
    Ok(())
}

/// Scenario: A sub-agent that cannot be closed stops the close and is reported
#[tokio::test(start_paused = true)]
async fn a_sub_agent_that_cannot_be_closed_stops_the_close_and_is_reported() -> TestResult {
    // @step Given a bot has two sub-agents and the second one fails to close
    let (mut app, mock) = fresh_app();
    open(&mut app, &["sub-1", "sub-2", "bot-1"], "bot-1").await?;
    make_bot(&mut app, "bot-1").await?;
    mock.seed_subordinates_for(sid("bot-1"), vec![sid("sub-1"), sid("sub-2")]);
    mock.fail_destroy_session(sid("sub-2"), "refused");
    // @step When the user confirms "Close All"
    activate_close(&mut app).await?;
    assert_shows(&screen(&mut app), " Close All    Cancel ")?;
    press(&mut app, KeyCode::Enter).await?;
    // @step Then the first sub-agent is closed
    assert_eq!(mock.destroyed_sessions(), vec![sid("sub-1")]);
    // @step And the second sub-agent and the bot keep their tabs
    assert_eq!(open_ids(&app), vec![sid("sub-2"), sid("bot-1")]);
    // @step And the failure "The bot was not closed: sub-agent sub-2 could not be closed: refused. 1 of 2 sub-agents were closed." is reported
    let expected = "The bot was not closed: sub-agent sub-2 could not be closed: refused. 1 of 2 sub-agents were closed.";
    assert!(
        app.session_operation_errors()
            .iter()
            .any(|error| error == expected),
        "errors: {:?}",
        app.session_operation_errors()
    );
    Ok(())
}

/// Scenario: If the sub-agents cannot be listed nothing is closed and the failure is reported
#[tokio::test(start_paused = true)]
async fn if_the_sub_agents_cannot_be_listed_nothing_is_closed_and_the_failure_is_reported(
) -> TestResult {
    // @step Given listing the focused bot's sub-agents fails with "offline"
    let (mut app, mock) = fresh_app();
    open(&mut app, &["bot-1"], "bot-1").await?;
    make_bot(&mut app, "bot-1").await?;
    mock.fail_get_subordinates("offline");
    // @step When the user activates "[ Close ]"
    activate_close(&mut app).await?;
    // @step Then no close dialog opens
    assert!(!app.compositor().contains(CLOSE_DIALOG_ID));
    assert!(mock.destroyed_sessions().is_empty());
    // @step And the failure "The bot was not closed: its sub-agents could not be listed: offline" is reported
    let expected = "The bot was not closed: its sub-agents could not be listed: offline";
    assert!(
        app.session_operation_errors()
            .iter()
            .any(|error| error == expected),
        "errors: {:?}",
        app.session_operation_errors()
    );
    Ok(())
}
