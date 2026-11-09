//! TUI-124 — the start dialogs (New Agent, a work unit, New Bot) and the
//! /resume delete dialog say exactly what will happen, in the words of the
//! session's kind.
//!
//! Feature: spec/features/agent-close-and-start-dialogs-say-exactly-what-will-happen.feature

use codelet_fspec_tui::views::agent::ResumeSessionView;
use codelet_fspec_tui::{Action, ViewMode};
use codelet_rpc_types::{SessionInfo, WorkUnitInfo};
use codelet_sessions::bot::{is_bot_role, ORCHESTRATOR_ROLE};
use codelet_test_helpers::{OrFail, TestResult};
use crossterm::event::{KeyCode, KeyModifiers};

use super::{assert_shows, click_label, drain, fresh_app, open, press, screen};

fn work_unit(id: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.into(),
        title: "Agent close and start dialogs".into(),
        status: "specifying".into(),
        work_type: "story".into(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

fn listed(id: &str, role: Option<&str>) -> SessionInfo {
    SessionInfo {
        id: id.to_string(),
        name: id.to_string(),
        status: "idle".to_string(),
        project: String::new(),
        message_count: 3,
        provider_id: None,
        model_id: None,
        is_isolated: false,
        worktree_path: None,
        role: role.map(str::to_string),
        updated_at_ms: None,
        work_unit_focus: Default::default(),
    }
}

/// Scenario: The New Agent dialog says what will be started
#[tokio::test(start_paused = true)]
async fn the_new_agent_dialog_says_what_will_be_started() -> TestResult {
    // @step Given the board is shown
    let (mut app, _mock) = fresh_app();
    app.navigator_mut().active_view = ViewMode::Board;
    // @step When the user starts a new agent
    press(&mut app, KeyCode::Char('.')).await?;
    let text = screen(&mut app);
    // @step Then the dialog titled "Start New Agent?" says "Starts a new agent, not linked to any work unit."
    assert_shows(&text, "Start New Agent?")?;
    assert_shows(&text, "Starts a new agent, not linked to any work unit.")?;
    // @step And it says "Start in Worktree gives it its own new git worktree."
    assert_shows(
        &text,
        "Start in Worktree gives it its own new git worktree.",
    )?;
    // @step And its options are "Start", "Start in Worktree" and "Cancel"
    assert_shows(&text, " Start    Start in Worktree    Cancel ")?;
    assert!(!text.contains("AI "), "no 'AI session' wording:\n{text}");
    Ok(())
}

/// Scenario: The dialog for a work unit says the agent is linked to it
#[tokio::test(start_paused = true)]
async fn the_dialog_for_a_work_unit_says_the_agent_is_linked_to_it() -> TestResult {
    // @step Given work unit TUI-124 has no agent
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::WorkUnitsLoaded(vec![work_unit("TUI-124")]));
    app.navigator_mut().active_view = ViewMode::Board;
    // @step When the user presses Enter on TUI-124
    app.dispatch(Action::EnterWorkUnit("TUI-124".into()));
    drain(&mut app).await?;
    let text = screen(&mut app);
    // @step Then the dialog titled "Start Agent for TUI-124?" says "Starts a new agent linked to work unit TUI-124."
    assert_shows(&text, "Start Agent for TUI-124?")?;
    assert_shows(&text, "Starts a new agent linked to work unit TUI-124.")
}

/// Scenario: The New Bot dialog says a bot is started
#[tokio::test(start_paused = true)]
async fn the_new_bot_dialog_says_a_bot_is_started() -> TestResult {
    // @step Given an agent is open in the agent view
    let (mut app, mock) = fresh_app();
    open(&mut app, &["s-1"], "s-1").await?;
    // @step When the user activates "[ New Bot ]"
    click_label(&mut app, "[ New Bot ]").await?;
    let text = screen(&mut app);
    // @step Then the dialog titled "Start New Bot?" says "Starts a new bot that hands your tasks to sub-agents."
    assert_shows(&text, "Start New Bot?")?;
    assert_shows(
        &text,
        "Starts a new bot that hands your tasks to sub-agents.",
    )?;
    // @step And a left click on "Start in Worktree" starts an isolated bot
    // The button row, not the description line that names the option.
    click_label(&mut app, " Start in Worktree    Cancel ").await?;
    assert_eq!(mock.create_isolated_session_calls(), 1);
    let roles = mock.created_isolated_session_roles();
    assert!(
        roles
            .first()
            .and_then(Option::as_deref)
            .is_some_and(is_bot_role),
        "roles: {roles:?}"
    );
    Ok(())
}

/// Scenario: Deleting from /resume names the kind and says the history is deleted
#[test]
fn deleting_from_resume_names_the_kind_and_says_the_history_is_deleted() -> TestResult {
    // @step Given the /resume list shows a bot and an agent
    let mut view = ResumeSessionView::new();
    view.set_sessions(vec![
        listed("bot-1", Some(ORCHESTRATOR_ROLE)),
        listed("s-1", None),
    ]);
    // @step When the user presses D on the bot
    let _ = view.handle_key(KeyCode::Char('d'), KeyModifiers::NONE, 20);
    let dialog = view.delete_confirm().or_fail("the delete dialog opens")?;
    // @step Then the dialog titled "Delete Bot?" says "Delete bot" with the bot's id
    assert_eq!(dialog.title(), "Delete Bot?");
    assert!(
        dialog.body().contains("Delete bot bot-1?"),
        "{}",
        dialog.body()
    );
    // @step And it says "Its saved history is deleted; it can no longer be resumed."
    assert!(
        dialog
            .body()
            .contains("Its saved history is deleted; it can no longer be resumed."),
        "{}",
        dialog.body()
    );
    // @step When the user presses D on the agent
    let mut view = ResumeSessionView::new();
    view.set_sessions(vec![
        listed("bot-1", Some(ORCHESTRATOR_ROLE)),
        listed("s-1", None),
    ]);
    let _ = view.handle_key(KeyCode::Down, KeyModifiers::NONE, 20);
    let _ = view.handle_key(KeyCode::Char('d'), KeyModifiers::NONE, 20);
    let dialog = view.delete_confirm().or_fail("the delete dialog opens")?;
    // @step Then the dialog is titled "Delete Agent?"
    assert_eq!(dialog.title(), "Delete Agent?");
    assert!(
        dialog.body().contains("Delete agent s-1?"),
        "{}",
        dialog.body()
    );
    Ok(())
}
