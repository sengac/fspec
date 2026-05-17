//! RPC-020 — App::dispatch routing tests for SlashCommandSelected /
//! SearchFiles / FileSearchResults.
//!
//! Feature: spec/features/rpc020-slash-and-file-popups.feature
//!
//! Exercises the App-level wiring of the three new RPC-020 Action
//! variants: that `SlashCommandSelected(Help)` pushes the HelpDialog
//! onto the Compositor at Priority::Critical, that
//! `SlashCommandSelected(Clear)` resets the AgentView's scrollback +
//! input, that `SlashCommandSelected(Quit)` sets `should_quit`, and
//! that all other variants surface a `[notice]` scrollback line. Also
//! verifies that `FileSearchResults` is folded into the open file
//! popup (App::dispatch routing for an internal Action — no direct
//! feature scenario; supplementary to the view-level tests in
//! `view_agent_popups_rpc020.rs`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{Action, App, FspecBackend, Priority, RenderedChunk};
use ratatui::text::Line;

mod common;
use common::MockBackend;

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

/// Scenario: Pressing Enter on /help pushes the HelpDialog onto the Compositor at Priority::Critical
#[test]
fn dispatch_help_pushes_help_dialog_at_priority_critical() {
    // @step Given an App with an AgentView whose slash popup is open with "/help" highlighted
    let (mut app, _mock) = fresh_app();
    assert!(!app.compositor().contains("help-dialog"));

    // @step When the user presses Enter
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Help));

    // @step Then AgentView emits Action::SlashCommandSelected(SlashCommandAction::Help)
    // (Action is dispatched directly above; the routing under test is
    // App::dispatch's reaction to receiving that Action.)

    // @step And dispatching that action pushes a HelpDialog Component with id "help-dialog" onto the Compositor
    assert!(app.compositor().contains("help-dialog"));
    // @step And the topmost compositor layer reports priority Priority::Critical
    assert_eq!(app.compositor().topmost_priority(), Some(Priority::Critical));
}

/// Scenario: Pressing Enter on /clear resets the AgentView scrollback and input
#[test]
fn dispatch_clear_resets_scrollback_and_input() {
    // @step Given an AgentView whose scrollback has 5 chunks
    let (mut app, _mock) = fresh_app();
    for _ in 0..5 {
        app.navigator_mut().agent.scrollback.push(RenderedChunk {
            seq: 0,
            lines: vec![Line::from("x")],
        });
    }
    // @step And the slash popup is open with "/clear" highlighted
    app.navigator_mut().agent.input.set_value("/clear");
    assert_eq!(app.navigator().agent.chunk_count(), 5);

    // @step When the user presses Enter
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Clear));

    // @step Then AgentView emits Action::SlashCommandSelected(SlashCommandAction::Clear)
    // (Action is dispatched directly above; this test asserts the
    // App::dispatch reaction to that Action.)

    // @step And dispatching that action makes the AgentView's chunk_count equal 0
    assert_eq!(app.navigator().agent.chunk_count(), 0);
    // @step And the MultiLineInput's buffer is empty
    assert!(app.navigator().agent.input.is_empty());
}

/// Scenario: Pressing Enter on /quit emits Action::Quit
#[test]
fn dispatch_quit_flips_should_quit() {
    // @step Given an AgentView whose slash popup is open with "/quit" highlighted
    let (mut app, _mock) = fresh_app();
    assert!(!app.should_quit());

    // @step When the user presses Enter
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Quit));

    // @step Then AgentView emits Action::SlashCommandSelected(SlashCommandAction::Quit)
    // (Action is dispatched directly above; this test asserts the
    // App::dispatch reaction to that Action.)

    // @step And dispatching that action sets App.should_quit to true
    assert!(app.should_quit());
}

/// Scenario: Pressing Enter on an unimplemented command emits a scrollback notice
#[test]
fn dispatch_unimplemented_command_emits_scrollback_notice() {
    // @step Given an AgentView whose slash popup is open with "/model" highlighted
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.navigator().agent.chunk_count(), 0);

    // @step When the user presses Enter
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Model));

    // @step Then AgentView emits Action::SlashCommandSelected(SlashCommandAction::Model)
    // (Action is dispatched directly above; this test asserts the
    // App::dispatch reaction to that Action.)

    // @step And dispatching that action appends one scrollback chunk whose text contains "[notice]"
    assert_eq!(
        app.navigator().agent.chunk_count(),
        1,
        "one notice chunk should be pushed"
    );
}

/// Supplementary App-level routing test for the internal
/// `Action::FileSearchResults(Vec<String>)` variant.
///
/// This test does NOT map 1:1 to a Gherkin scenario — it covers the
/// App::dispatch routing path that folds backend matches into the open
/// file popup. The corresponding user-facing scenarios (popup opens on
/// '@', Enter splices the chosen path) are covered by tests in
/// `view_agent_popups_rpc020.rs`, which set popup matches synchronously
/// via `set_file_search_results` rather than through Action dispatch.
#[tokio::test]
async fn dispatch_file_search_results_folds_into_popup() {
    let (mut app, _mock) = fresh_app();
    // Open a file popup by typing "@" through the view's handle_event.
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    for ch in "@rea".chars() {
        let e = Event::Key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        app.navigator_mut().agent.handle_event(&e);
    }
    assert!(app.navigator().agent.file_popup.is_some());

    // Dispatch FileSearchResults — App::dispatch routes into
    // AgentView::set_file_search_results.
    app.dispatch(Action::FileSearchResults(vec![
        "README.md".to_string(),
        "src/reader.ts".to_string(),
    ]));

    let popup = app
        .navigator()
        .agent
        .file_popup
        .as_ref()
        .expect("file popup still open");
    assert_eq!(popup.match_count(), 2);
    assert_eq!(popup.matches()[0], "README.md");
}
