//! Feature: spec/features/blocklist-toggle-rule-turns-the-rule-off-for-the-session.feature
//!
//! BUG-601: the blocklist view's Toggle Rule flipped a set kept only in the
//! TUI, so the rule stayed enforced. The view now asks the daemon to turn the
//! rule off or on for the session it was opened from, and shows only what the
//! daemon answers. The daemon is a MockBackend that keeps each session's
//! turned-off rules and records every call.

use codelet_test_helpers::{OrFail, TestError, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{BlocklistRuleInfo, SessionId};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

mod common;
use common::{buffer_to_rows, render_one_frame, MockBackend, TaskOutcome};

fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

fn rule(id: &str, pattern: &str) -> BlocklistRuleInfo {
    BlocklistRuleInfo {
        id: id.to_string(),
        pattern: pattern.to_string(),
        action: "block".to_string(),
        reason: format!("reason for {id}"),
        guidance: None,
        source: "system".to_string(),
    }
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn space() -> Event {
    key(KeyCode::Char(' '))
}

async fn drain(app: &mut App) -> TestResult {
    loop {
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
        let Some(action) = app.try_recv_action() else {
            return Ok(());
        };
        app.dispatch(action);
    }
}

/// A daemon listing git-checkout-block then cat-block.
fn daemon() -> Arc<MockBackend> {
    let mock = Arc::new(MockBackend::new());
    mock.seed_blocklist_rules(vec![
        rule("git-checkout-block", "^git\\s+checkout\\b"),
        rule("cat-block", "^cat\\s+"),
    ]);
    mock
}

/// An App with session s-1 focused (or none) whose `/blocklist` was opened.
async fn open_blocklist(mock: &Arc<MockBackend>, session: Option<&str>) -> Result<App, TestError> {
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    if let Some(session) = session {
        app.dispatch(Action::SessionCreated(sid(session)));
        app.dispatch(Action::OpenAgentView(Some(sid(session))));
        drain(&mut app).await?;
    }
    app.dispatch(Action::SlashCommandSelected(
        app.current_session(),
        SlashCommandAction::Blocklist,
    ));
    drain(&mut app).await?;
    assert_eq!(app.navigator().active_view, ViewMode::Blocklist);
    Ok(app)
}

fn screen(app: &mut App) -> Result<String, TestError> {
    let mut terminal = Terminal::new(TestBackend::new(120, 24)).or_fail("terminal")?;
    let buffer = render_one_frame(&mut terminal, app)?;
    Ok(buffer_to_rows(&buffer).join("\n"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn opening_the_blocklist_view_shows_the_rules_the_daemon_has_turned_off_for_the_session(
) -> TestResult {
    // @step Given the daemon lists rules "git-checkout-block" and "cat-block" and has turned off "cat-block" for session "s-1"
    let mock = daemon();
    mock.seed_blocklist_disabled(&sid("s-1"), &["cat-block"]);

    // @step When the user opens "/blocklist" from "s-1"
    let mut app = open_blocklist(&mock, Some("s-1")).await?;

    // @step Then the TUI asks the daemon for the turned-off rules of "s-1"
    assert_eq!(mock.blocklist_disabled_requests(), vec!["s-1".to_string()]);

    // @step And the view shows "○ cat-block" with "(disabled)" and "● git-checkout-block"
    let text = screen(&mut app)?;
    assert!(text.contains("○ cat-block"), "{text}");
    assert!(text.contains("(disabled)"), "{text}");
    assert!(text.contains("● git-checkout-block"), "{text}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn toggle_rule_asks_the_daemon_and_shows_its_answer() -> TestResult {
    // @step Given the blocklist view opened from "s-1" shows "git-checkout-block" on
    let mock = daemon();
    let mut app = open_blocklist(&mock, Some("s-1")).await?;
    assert!(screen(&mut app)?.contains("● git-checkout-block"));

    // @step When the user presses Space on "git-checkout-block"
    let _ = app.handle_event(&space());
    drain(&mut app).await?;

    // @step Then the TUI asks the daemon to turn "git-checkout-block" off for "s-1"
    assert_eq!(
        mock.blocklist_toggle_requests(),
        vec![("s-1".to_string(), "git-checkout-block".to_string(), false)]
    );

    // @step And the view shows "○ git-checkout-block"
    assert!(screen(&mut app)?.contains("○ git-checkout-block"));

    // @step When the user presses Space on "git-checkout-block" again
    let _ = app.handle_event(&space());
    drain(&mut app).await?;

    // @step Then the TUI asks the daemon to turn "git-checkout-block" on for "s-1"
    assert_eq!(
        mock.blocklist_toggle_requests().last(),
        Some(&("s-1".to_string(), "git-checkout-block".to_string(), true))
    );

    // @step And the view shows "● git-checkout-block"
    assert!(screen(&mut app)?.contains("● git-checkout-block"));
    assert!(app.session_operation_errors().is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refused_toggle_shows_the_daemons_error_and_keeps_the_rules_state() -> TestResult {
    // @step Given the blocklist view opened from "s-1" shows "git-checkout-block" on, and the daemon refuses toggles with 'the blocklist has no rule "git-checkout-block"'
    let mock = daemon();
    mock.fail_blocklist_toggles("the blocklist has no rule \"git-checkout-block\"");
    let mut app = open_blocklist(&mock, Some("s-1")).await?;

    // @step When the user presses Space on "git-checkout-block"
    let _ = app.handle_event(&space());
    drain(&mut app).await?;

    // @step Then the TUI shows the error 'Toggle Rule failed: the blocklist has no rule "git-checkout-block"'
    assert_eq!(
        app.session_operation_errors(),
        ["Toggle Rule failed: the blocklist has no rule \"git-checkout-block\"".to_string()]
    );

    // @step And the view shows "● git-checkout-block"
    // (once the user dismisses the error dialog over it)
    let _ = app.handle_event(&key(KeyCode::Esc));
    drain(&mut app).await?;
    let text = screen(&mut app)?;
    assert!(text.contains("● git-checkout-block"), "{text}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_read_shows_the_error_instead_of_an_empty_or_all_on_view() -> TestResult {
    // @step Given the daemon refuses to list the blocklist rules with "cannot load the project blocklist" and to read the turned-off rules of "s-1" with "Session not found: s-1"
    let mock = daemon();
    mock.set_blocklist_list_error("cannot load the project blocklist".to_string());
    mock.fail_blocklist_disabled_reads("Session not found: s-1");

    // @step When the user opens "/blocklist" from "s-1"
    let app = open_blocklist(&mock, Some("s-1")).await?;

    // @step Then the TUI shows the errors "Blocklist rules could not be loaded: cannot load the project blocklist" and "Turned-off blocklist rules could not be loaded: Session not found: s-1"
    let mut errors = app.session_operation_errors().to_vec();
    errors.sort();
    assert_eq!(
        errors,
        [
            "Blocklist rules could not be loaded: cannot load the project blocklist".to_string(),
            "Turned-off blocklist rules could not be loaded: Session not found: s-1".to_string(),
        ]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn toggle_rule_without_a_session_shows_an_error() -> TestResult {
    // @step Given the blocklist view is open while no session is open
    let mock = daemon();
    let mut app = open_blocklist(&mock, None).await?;
    assert!(mock.blocklist_disabled_requests().is_empty());

    // @step When the user presses Space on "git-checkout-block"
    let _ = app.handle_event(&space());
    drain(&mut app).await?;

    // @step Then the TUI shows the error "Toggle Rule failed: open /blocklist from a session to turn its rules off or on"
    assert_eq!(
        app.session_operation_errors(),
        [
            "Toggle Rule failed: open /blocklist from a session to turn its rules off or on"
                .to_string()
        ]
    );

    // @step And the daemon is not asked to change any rule
    assert!(mock.blocklist_toggle_requests().is_empty());
    Ok(())
}
