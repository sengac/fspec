//! TUI-124 — closing an agent (and a sub-agent): what the dialog says and
//! what closing does.
//!
//! Feature: spec/features/agent-close-and-start-dialogs-say-exactly-what-will-happen.feature

use codelet_fspec_tui::store::IsolationState;
use codelet_rpc_types::SessionStatus;
use codelet_test_helpers::TestResult;
use crossterm::event::KeyCode;

use super::{
    activate_close, assert_shows, drain, fresh_app, make_bot, open, open_ids, press, render,
    row_text, screen, sid, AGENT_BAR_ROW, CLOSE_DIALOG_ID,
};

/// Scenario: The agent bar's close button reads [ Close ] and is clicked where it is painted
#[tokio::test(start_paused = true)]
async fn the_agent_bars_close_button_reads_close_and_is_clicked_where_it_is_painted() -> TestResult
{
    // @step Given an agent is open in the agent view
    let (mut app, _mock) = fresh_app();
    open(&mut app, &["s-1"], "s-1").await?;
    // @step When the agent bar is painted
    let bar = row_text(&render(&mut app), AGENT_BAR_ROW);
    // @step Then its right-aligned buttons read "[ New Bot ]", "[ New Agent ]" and "[ Close ]"
    let bot = bar.find("[ New Bot ]");
    let agent = bar.find("[ New Agent ]");
    let close = bar.find("[ Close ]");
    assert!(bot < agent && agent < close && bot.is_some(), "bar: {bar}");
    assert!(!bar.contains("Close Agent"), "bar: {bar}");
    // @step And a left click on "[ Close ]" opens the "Close Agent?" dialog
    activate_close(&mut app).await?;
    assert!(app.compositor().contains(CLOSE_DIALOG_ID));
    assert_shows(&screen(&mut app), "Close Agent?")
}

/// Scenario: Closing an idle agent says it ends the agent and keeps its history
#[tokio::test(start_paused = true)]
async fn closing_an_idle_agent_says_it_ends_the_agent_and_keeps_its_history() -> TestResult {
    // @step Given an idle agent with an empty input is focused
    let (mut app, _mock) = fresh_app();
    open(&mut app, &["s-1"], "s-1").await?;
    // @step When the user activates "[ Close ]"
    activate_close(&mut app).await?;
    let text = screen(&mut app);
    // @step Then the dialog titled "Close Agent?" says "Closing ends this agent and returns you to the board."
    assert_shows(&text, "Close Agent?")?;
    assert_shows(
        &text,
        "Closing ends this agent and returns you to the board.",
    )?;
    // @step And it says "Its history is kept: resume it later with /resume."
    assert_shows(&text, "Its history is kept: resume it later with /resume.")?;
    // @step And its options are "Close" and "Cancel"
    assert_shows(&text, " Close    Cancel ")?;
    assert!(!text.contains("Session"), "no 'Session' wording:\n{text}");
    Ok(())
}

/// Scenario: Closing a running agent says closing stops it
#[tokio::test(start_paused = true)]
async fn closing_a_running_agent_says_closing_stops_it() -> TestResult {
    // @step Given a running agent is focused
    let (mut app, _mock) = fresh_app();
    open(&mut app, &["s-1"], "s-1").await?;
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Running);
    // @step When the user activates "[ Close ]"
    activate_close(&mut app).await?;
    let text = screen(&mut app);
    // @step Then the dialog titled "Close Agent?" says "This agent is working. Closing stops it and returns you to the board."
    assert_shows(&text, "Close Agent?")?;
    assert_shows(
        &text,
        "This agent is working. Closing stops it and returns you to the board.",
    )?;
    // @step And its options are "Stop and Close" and "Cancel"
    assert_shows(&text, " Stop and Close    Cancel ")
}

/// Scenario: Closing an isolated agent says its worktree is kept
#[tokio::test(start_paused = true)]
async fn closing_an_isolated_agent_says_its_worktree_is_kept() -> TestResult {
    // @step Given an idle isolated agent is focused
    let (mut app, _mock) = fresh_app();
    open(&mut app, &["s-1"], "s-1").await?;
    app.agent_view_store_mut().set_isolation_state(
        sid("s-1"),
        IsolationState {
            is_isolated: true,
            worktree_path: Some(".fspec/worktrees/s-1".to_string()),
            base_commit: Some("abc123".to_string()),
        },
    );
    // @step When the user activates "[ Close ]"
    activate_close(&mut app).await?;
    // @step Then the dialog says "Its worktree is kept until you prune it with /worktrees."
    assert_shows(
        &screen(&mut app),
        "Its worktree is kept until you prune it with /worktrees.",
    )
}

/// Scenario: An agent whose backend close fails keeps its tab and the failure is reported
#[tokio::test(start_paused = true)]
async fn an_agent_whose_backend_close_fails_keeps_its_tab_and_the_failure_is_reported() -> TestResult
{
    // @step Given an idle agent is focused and its backend close fails with "refused"
    let (mut app, mock) = fresh_app();
    open(&mut app, &["s-1"], "s-1").await?;
    mock.fail_destroy_session(sid("s-1"), "refused");
    // @step When the user confirms "Close"
    activate_close(&mut app).await?;
    press(&mut app, KeyCode::Enter).await?;
    // @step Then the agent keeps its tab
    assert_eq!(open_ids(&app), vec![sid("s-1")]);
    // @step And the failure "The agent was not closed: refused" is reported
    assert!(
        app.session_operation_errors()
            .iter()
            .any(|error| error == "The agent was not closed: refused"),
        "errors: {:?}",
        app.session_operation_errors()
    );
    Ok(())
}

/// Scenario: Closing a sub-agent closes only it and says so
#[tokio::test(start_paused = true)]
async fn closing_a_sub_agent_closes_only_it_and_says_so() -> TestResult {
    // @step Given a sub-agent started by a bot is focused
    let (mut app, mock) = fresh_app();
    open(&mut app, &["bot-1", "sub-1"], "sub-1").await?;
    make_bot(&mut app, "bot-1").await?;
    mock.seed_subordinates_for(sid("bot-1"), vec![sid("sub-1")]);
    mock.seed_supervisors_for(sid("sub-1"), vec![sid("bot-1")]);
    // @step When the user activates "[ Close ]"
    activate_close(&mut app).await?;
    let text = screen(&mut app);
    // @step Then the dialog titled "Close Agent?" says "Only this agent is closed; the bot that started it keeps running."
    assert_shows(&text, "Close Agent?")?;
    assert_shows(
        &text,
        "Only this agent is closed; the bot that started it keeps running.",
    )?;
    // @step And confirming closes only the sub-agent
    press(&mut app, KeyCode::Enter).await?;
    assert_eq!(mock.destroyed_sessions(), vec![sid("sub-1")]);
    assert_eq!(open_ids(&app), vec![sid("bot-1")]);
    Ok(())
}

/// Scenario: Closing an agent leaves its sub-agents running and says so
#[tokio::test(start_paused = true)]
async fn closing_an_agent_leaves_its_sub_agents_running_and_says_so() -> TestResult {
    // @step Given an agent that started two sub-agents is focused
    let (mut app, mock) = fresh_app();
    open(&mut app, &["sub-1", "sub-2", "s-1"], "s-1").await?;
    mock.seed_subordinates_for(sid("s-1"), vec![sid("sub-1"), sid("sub-2")]);
    // @step When the user activates "[ Close ]"
    activate_close(&mut app).await?;
    // @step Then the dialog says "Its 2 sub-agents keep running."
    assert_shows(&screen(&mut app), "Its 2 sub-agents keep running.")?;
    // @step And confirming closes only the agent
    press(&mut app, KeyCode::Enter).await?;
    drain(&mut app).await?;
    assert_eq!(mock.destroyed_sessions(), vec![sid("s-1")]);
    assert_eq!(open_ids(&app), vec![sid("sub-1"), sid("sub-2")]);
    Ok(())
}
