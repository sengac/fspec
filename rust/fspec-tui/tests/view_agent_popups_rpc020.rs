//! RPC-020 — Slash + File search popup behavioural tests.
//!
//! Feature: spec/features/rpc020-slash-and-file-popups.feature
//!
//! Drives the AgentView popup-overlay scenarios end-to-end: typing
//! triggers a popup, navigation chords adjust the selection, Enter
//! emits Action::SlashCommandSelected / splices a file path, etc.

use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{
    Action, AgentView, AgentViewStore, App, FspecBackend, Priority, RenderedChunk, SessionContext,
};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::text::Line;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

mod common;
use common::MockBackend;

fn fresh_view() -> (AgentView, UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    // BUG-552: the composer belongs to a session, as App::sync_composer_draft sets it.
    let mut view = AgentView::new(tx);
    view.set_composer_session(Some(codelet_rpc_types::SessionId::new("s-1")));
    (view, rx)
}

fn fresh_app() -> App {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    App::new(backend)
}

/// Seed `app` with a single open SessionContext so AgentView callers
/// have a current session to mutate (RPC-024 moved scrollback off
/// AgentView and onto SessionContext).
fn seed_session(app: &mut App, id: &str) {
    app.dispatch(Action::SessionCreated(SessionId::new(id)));
}

fn push_chunk(app: &mut App, body: &str) -> Result<(), codelet_test_helpers::TestError> {
    let ctx = app
        .agent_view_store_mut()
        .current_session_context_mut()
        .or_fail("current ctx")?;
    ctx.scrollback.push(RenderedChunk {
        row_ends: Vec::new(),
        seq: 0,
        lines: vec![Line::from(body.to_string())],
        source: None,
    });
    Ok(())
}

#[allow(dead_code)]
fn scrollback_text(app: &App) -> String {
    let chunks = app
        .agent_view_store()
        .current_session_context()
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default();
    chunks
        .iter()
        .flat_map(|c| {
            c.lines.iter().map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

fn type_chars(view: &mut AgentView, s: &str) {
    for ch in s.chars() {
        view.handle_event(&key(KeyCode::Char(ch), KeyModifiers::NONE));
    }
}

fn drain(rx: &mut UnboundedReceiver<Action>) -> Vec<Action> {
    let mut out = Vec::new();
    while let Ok(a) = rx.try_recv() {
        out.push(a);
    }
    out
}

/// Scenario: Typing '/' on an empty input opens the slash command palette
#[test]
fn typing_slash_on_empty_input_opens_slash_command_palette() -> TestResult {
    // @step Given an AgentView with an empty MultiLineInput and no popup visible
    let (mut view, _rx) = fresh_view();
    assert!(view.slash_popup.is_none());

    // @step When the user types "/"
    type_chars(&mut view, "/");

    // @step Then AgentView.slash_popup is Some
    let popup = view
        .slash_popup
        .as_ref()
        .or_fail("slash popup should be open")?;
    // @step And the slash popup's filter is ""
    assert_eq!(popup.filter(), "");
    // @step And the slash popup's match count equals the SLASH_COMMANDS registry length
    assert_eq!(popup.match_count(), codelet_fspec_tui::SLASH_COMMANDS.len());
    // @step And the slash popup's selected index is 0
    assert_eq!(popup.selected_index(), 0);
    Ok(())
}

/// Scenario: Typing characters after '/' filters the slash command list
#[test]
fn typing_chars_after_slash_filters_the_slash_command_list() -> TestResult {
    // @step Given an AgentView whose MultiLineInput contains "/"
    let (mut view, _rx) = fresh_view();
    type_chars(&mut view, "/");

    // @step When the user types "he"
    type_chars(&mut view, "he");

    // @step Then the slash popup's filter is "he"
    let popup = view.slash_popup.as_ref().or_fail("slash popup")?;
    assert_eq!(popup.filter(), "he");
    // @step And the slash popup's first match is the command named "help"
    assert_eq!(popup.matches()[0].name(), "help");
    Ok(())
}

/// Scenario: Pressing Enter on /quit emits Action::Quit
#[test]
fn pressing_enter_on_quit_emits_slash_command_selected_quit() -> TestResult {
    // @step Given an AgentView whose slash popup is open with "/quit" highlighted
    let (mut view, mut rx) = fresh_view();
    type_chars(&mut view, "/quit");
    let _ = drain(&mut rx);

    // @step When the user presses Enter
    view.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));

    // @step Then AgentView emits Action::SlashCommandSelected(SlashCommandAction::Quit)
    let actions = drain(&mut rx);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::SlashCommandSelected(_, SlashCommandAction::Quit))),
        "expected SlashCommandSelected(Quit) on bus, got {actions:?}"
    );
    // @step And the slash popup is closed
    assert!(view.slash_popup.is_none());
    // @step And dispatching that action sets App.should_quit to true
    // (App::dispatch is exercised in app_with_mock_backend_repl tests;
    // here we assert only the Action escape — the dispatch behaviour
    // is covered by the rpc020-app-dispatch test file.)
    Ok(())
}

/// Scenario: Pressing Enter on /clear resets the AgentView scrollback and input
#[test]
fn pressing_enter_on_clear_emits_clear_action() -> TestResult {
    // @step Given an AgentView whose scrollback has 5 chunks
    let (mut view, mut rx) = fresh_view();
    let mut store = AgentViewStore::default();
    store.append_session(SessionContext::new(SessionId::new("s-1")));
    {
        let ctx = store.current_session_context_mut().or_fail("ctx")?;
        for _ in 0..5 {
            ctx.scrollback.push(RenderedChunk {
                row_ends: Vec::new(),
                seq: 0,
                lines: vec![Line::from("x")],
                source: None,
            });
        }
    }
    assert_eq!(view.chunk_count(&store), 5);
    // @step And the slash popup is open with "/clear" highlighted
    type_chars(&mut view, "/clear");
    let _ = drain(&mut rx);

    // @step When the user presses Enter
    view.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));

    // @step Then AgentView emits Action::SlashCommandSelected(SlashCommandAction::Clear)
    let actions = drain(&mut rx);
    assert!(
        actions.iter().any(|a| matches!(
            a,
            Action::SlashCommandSelected(_, SlashCommandAction::Clear)
        )),
        "expected SlashCommandSelected(Clear), got {actions:?}"
    );
    // @step And dispatching that action makes the AgentView's chunk_count equal 0
    let mut app = fresh_app();
    seed_session(&mut app, "s-1");
    for _ in 0..5 {
        push_chunk(&mut app, "x")?;
    }
    app.navigator_mut().agent.input.set_value("/clear");
    assert_eq!(app.navigator().agent.chunk_count(app.agent_view_store()), 5);
    app.dispatch(Action::SlashCommandSelected(
        app.current_session(),
        SlashCommandAction::Clear,
    ));
    assert_eq!(app.navigator().agent.chunk_count(app.agent_view_store()), 0);
    // @step And the MultiLineInput's buffer is empty
    assert!(view.input.is_empty());
    assert!(app.navigator().agent.input.is_empty());
    // @step And the slash popup is closed
    assert!(view.slash_popup.is_none());
    Ok(())
}

/// Scenario: Pressing Enter on /help pushes the HelpDialog onto the Compositor at Priority::Critical
#[test]
fn pressing_enter_on_help_emits_help_action() -> TestResult {
    // @step Given an App with an AgentView whose slash popup is open with "/help" highlighted
    let (mut view, mut rx) = fresh_view();
    type_chars(&mut view, "/help");
    let _ = drain(&mut rx);

    // @step When the user presses Enter
    view.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));

    // @step Then AgentView emits Action::SlashCommandSelected(SlashCommandAction::Help)
    let actions = drain(&mut rx);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::SlashCommandSelected(_, SlashCommandAction::Help))),
        "expected SlashCommandSelected(Help), got {actions:?}"
    );
    // @step And dispatching that action pushes a HelpDialog Component with id "help-dialog" onto the Compositor
    let mut app = fresh_app();
    assert!(!app.compositor().contains("help-dialog"));
    app.dispatch(Action::SlashCommandSelected(
        app.current_session(),
        SlashCommandAction::Help,
    ));
    assert!(app.compositor().contains("help-dialog"));
    // @step And the topmost compositor layer reports priority Priority::Critical
    assert_eq!(
        app.compositor().topmost_priority(),
        Some(Priority::Critical)
    );
    // @step And the slash popup is closed
    assert!(view.slash_popup.is_none());
    Ok(())
}

/// Scenario: Pressing Enter on /isolation opens the CreateSessionDialog
///
/// RPC-060 (superseded by WT-009): /isolation probes
/// `list_session_worktrees` and, for a non-isolated session with no live
/// worktree, dispatches `Action::OpenCreateSessionDialog` with
/// `preselect=Some(Isolated)` — the catch-all `[notice] /<name> not yet
/// implemented` fallback was removed because every SlashCommandAction
/// variant has a handler. The probe needs a runtime, so the dispatch
/// half is an async test.
#[tokio::test]
async fn pressing_enter_on_unimplemented_command_emits_notice() -> TestResult {
    use codelet_fspec_tui::CreateSessionOption;
    // @step Given an AgentView whose slash popup is open with "/isolation" highlighted
    let (mut view, mut rx) = fresh_view();
    type_chars(&mut view, "/isolation");
    let _ = drain(&mut rx);

    // @step When the user presses Enter
    view.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));

    // @step Then AgentView emits Action::SlashCommandSelected(SlashCommandAction::Isolation)
    let actions = drain(&mut rx);
    assert!(
        actions.iter().any(|a| matches!(
            a,
            Action::SlashCommandSelected(_, SlashCommandAction::Isolation)
        )),
        "expected SlashCommandSelected(Isolation), got {actions:?}"
    );
    // @step And App::dispatch routes that action into Action::OpenCreateSessionDialog{preselect:Some(Isolated)}
    let mut app = fresh_app();
    seed_session(&mut app, "s-1");
    assert_eq!(app.navigator().agent.chunk_count(app.agent_view_store()), 0);
    app.dispatch(Action::SlashCommandSelected(
        app.current_session(),
        SlashCommandAction::Isolation,
    ));
    // (SessionCreated queued a ModelInfoLoaded probe first — drain it.)
    // TEST-042: wait for the action itself under the deadlock guard.
    let action = codelet_test_helpers::deadlock_guard("OpenCreateSessionDialog", async {
        loop {
            while let Some(a) = app.try_recv_action() {
                if matches!(a, Action::OpenCreateSessionDialog { .. }) {
                    return a;
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    match action {
        Action::OpenCreateSessionDialog { preselect } => {
            assert_eq!(preselect, Some(CreateSessionOption::Isolated));
        }
        other => return Err(format!("expected OpenCreateSessionDialog, got {other:?}").into()),
    }
    // @step And NO scrollback notice chunk is pushed for /isolation
    assert_eq!(
        app.navigator().agent.chunk_count(app.agent_view_store()),
        0,
        "no [notice] chunk should be pushed for /isolation"
    );
    // @step And the slash popup is closed
    assert!(view.slash_popup.is_none());
    Ok(())
}

/// Scenario: Pressing Tab on a selected slash command fills the input without executing
#[test]
fn pressing_tab_on_selected_slash_command_fills_input_without_executing() -> TestResult {
    // @step Given an AgentView whose MultiLineInput contains "/c"
    let (mut view, mut rx) = fresh_view();
    type_chars(&mut view, "/c");
    let _ = drain(&mut rx);

    // @step And the slash popup is open with "/clear" highlighted
    assert_eq!(
        view.slash_popup.as_ref().or_fail("popup")?.matches()[0].name(),
        "clear"
    );

    // @step When the user presses Tab
    view.handle_event(&key(KeyCode::Tab, KeyModifiers::NONE));

    // @step Then the MultiLineInput's buffer is exactly "/clear"
    assert_eq!(view.input.value(), "/clear");
    // @step And the slash popup is closed
    assert!(view.slash_popup.is_none());
    // @step And no Action::SlashCommandSelected variant was emitted
    let actions = drain(&mut rx);
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::SlashCommandSelected(..))),
        "no SlashCommandSelected expected, got {actions:?}"
    );
    Ok(())
}

/// Scenario: Pressing Esc on the slash popup dismisses it without clearing the input
#[test]
fn pressing_esc_on_slash_popup_dismisses_without_clearing_input() -> TestResult {
    // @step Given an AgentView whose MultiLineInput contains "/"
    let (mut view, mut rx) = fresh_view();
    type_chars(&mut view, "/");
    let _ = drain(&mut rx);
    // @step And the slash popup is open
    assert!(view.slash_popup.is_some());

    // @step When the user presses Esc
    view.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));

    // @step Then the slash popup is closed
    assert!(view.slash_popup.is_none());
    // @step And the MultiLineInput's buffer is still exactly "/"
    assert_eq!(view.input.value(), "/");
    // @step And no Action::BackToBoard was emitted
    let actions = drain(&mut rx);
    assert!(
        !actions.iter().any(|a| matches!(a, Action::BackToBoard)),
        "popup ESC must not propagate to BackToBoard, got {actions:?}"
    );
    Ok(())
}

/// Scenario: Typing '@' inside any input opens the file search popup at the '@' anchor
#[test]
fn typing_at_opens_file_search_popup_at_anchor() -> TestResult {
    // @step Given an AgentView whose MultiLineInput contains "hello "
    let (mut view, mut rx) = fresh_view();
    type_chars(&mut view, "hello ");
    let _ = drain(&mut rx);

    // @step When the user types "@rea"
    type_chars(&mut view, "@rea");

    // @step Then AgentView.file_popup is Some
    let popup = view.file_popup.as_ref().or_fail("file popup")?;
    // @step And the file popup's filter is "rea"
    assert_eq!(popup.filter(), "rea");
    // @step And the file popup's anchor_offset equals the byte offset of '@' in the joined buffer
    let buf = view.input.value();
    let expected_anchor = buf.rfind('@').or_fail("@ in buffer")?;
    assert_eq!(popup.anchor_offset(), expected_anchor);
    // @step And AgentView emits Action::SearchFiles("rea")
    let actions = drain(&mut rx);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::SearchFiles(s) if s == "rea")),
        "expected SearchFiles(\"rea\"), got {actions:?}"
    );
    Ok(())
}

/// Scenario: Pressing Enter on a file search result splices the chosen path with a trailing space
#[test]
fn pressing_enter_on_file_match_splices_path_with_trailing_space() -> TestResult {
    // @step Given an AgentView whose MultiLineInput contains "hello @rea"
    let (mut view, _rx) = fresh_view();
    type_chars(&mut view, "hello @rea");
    // @step And the file popup is open with matches ["README.md", "src/reader.ts"] and selected index 0
    view.set_file_search_results(vec!["README.md".to_string(), "src/reader.ts".to_string()]);

    // @step When the user presses Enter
    view.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));

    // @step Then the MultiLineInput's buffer is exactly "hello @README.md "
    assert_eq!(view.input.value(), "hello @README.md ");
    // @step And the file popup is closed
    assert!(view.file_popup.is_none());
    Ok(())
}

/// Scenario: Pressing Tab on a file search result splices the chosen path without a trailing space
#[test]
fn pressing_tab_on_file_match_splices_path_without_trailing_space() -> TestResult {
    // @step Given an AgentView whose MultiLineInput contains "hello @rea"
    let (mut view, _rx) = fresh_view();
    type_chars(&mut view, "hello @rea");
    // @step And the file popup is open with matches ["README.md"] and selected index 0
    view.set_file_search_results(vec!["README.md".to_string()]);

    // @step When the user presses Tab
    view.handle_event(&key(KeyCode::Tab, KeyModifiers::NONE));

    // @step Then the MultiLineInput's buffer is exactly "hello @README.md"
    assert_eq!(view.input.value(), "hello @README.md");
    // @step And the file popup is closed
    assert!(view.file_popup.is_none());
    Ok(())
}

/// Scenario: Typing a space after '@' auto-dismisses the file search popup
#[test]
fn space_after_at_auto_dismisses_file_search_popup() -> TestResult {
    // @step Given an AgentView whose MultiLineInput contains "hello @"
    let (mut view, _rx) = fresh_view();
    type_chars(&mut view, "hello @");
    // @step And the file popup is open with filter ""
    assert!(view.file_popup.is_some());

    // @step When the user types " "
    type_chars(&mut view, " ");

    // @step Then the file popup is closed
    assert!(view.file_popup.is_none());
    // @step And the MultiLineInput's buffer is exactly "hello @ "
    assert_eq!(view.input.value(), "hello @ ");
    Ok(())
}

/// Scenario: Pressing Esc on the file search popup dismisses it without modifying the input
#[test]
fn pressing_esc_on_file_search_popup_dismisses_without_modifying_input() -> TestResult {
    // @step Given an AgentView whose MultiLineInput contains "hello @rea"
    let (mut view, _rx) = fresh_view();
    type_chars(&mut view, "hello @rea");
    // @step And the file popup is open with matches ["README.md"]
    view.set_file_search_results(vec!["README.md".to_string()]);
    assert!(view.file_popup.is_some());

    // @step When the user presses Esc
    view.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));

    // @step Then the file popup is closed
    assert!(view.file_popup.is_none());
    // @step And the MultiLineInput's buffer is still exactly "hello @rea"
    assert_eq!(view.input.value(), "hello @rea");
    Ok(())
}

/// Scenario: While the slash popup is open, plain Enter is intercepted (not submitted to the chat)
#[test]
fn slash_popup_intercepts_plain_enter_so_chat_submit_does_not_fire() -> TestResult {
    // @step Given an AgentView whose slash popup is open with "/help" highlighted
    let (mut view, mut rx) = fresh_view();
    type_chars(&mut view, "/help");
    let _ = drain(&mut rx);

    // @step When the user presses plain Enter
    view.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));

    // @step Then AgentView emits Action::SlashCommandSelected(SlashCommandAction::Help)
    let actions = drain(&mut rx);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::SlashCommandSelected(_, SlashCommandAction::Help))),
        "got {actions:?}"
    );
    // @step And NO Action::InputSubmitted is emitted
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::InputSubmitted(..))),
        "InputSubmitted should not fire while popup is open"
    );
    Ok(())
}

/// Scenario: While the slash popup is open, 'q' is treated as a typed character not as Quit
#[test]
fn slash_popup_does_not_consume_q_as_quit() -> TestResult {
    // @step Given an AgentView whose MultiLineInput contains "/"
    let (mut view, mut rx) = fresh_view();
    type_chars(&mut view, "/");
    // @step And the slash popup is open
    assert!(view.slash_popup.is_some());
    let _ = drain(&mut rx);

    // @step When the user types "q"
    type_chars(&mut view, "q");

    // @step Then the MultiLineInput's buffer is exactly "/q"
    assert_eq!(view.input.value(), "/q");
    // @step And the slash popup is still open
    let popup = view.slash_popup.as_ref().or_fail("popup still open")?;
    // @step And the slash popup's filter is "q"
    assert_eq!(popup.filter(), "q");
    // @step And no Action::Quit was emitted
    let actions = drain(&mut rx);
    assert!(
        !actions.iter().any(|a| matches!(a, Action::Quit)),
        "Quit must not fire from typing 'q' into the popup filter"
    );
    Ok(())
}
