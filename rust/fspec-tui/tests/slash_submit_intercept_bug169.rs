//! BUG-169 — slash-command submit interception is registry-driven.
//!
//! Feature: spec/features/slash-submit-intercept-registry.feature
//!
//! Root cause: `parse_slash_command` (the submit-time interceptor in
//! `App::handle_input_submitted`) only recognized 11 of the 21 registered
//! slash commands. Every other command that reached submit with the popup
//! closed — via Tab-fill (`PopupOutcome::Filled`), Esc-dismiss
//! (`PopupOutcome::Dismiss`), or plain typing — fell through to
//! `backend.send_input` and was sent to the LLM verbatim.
//!
//! These tests drive BOTH entry points of the regression:
//!   1. `App::dispatch(Action::InputSubmitted(..))` — the typed-submit path
//!      (`handle_input_submitted` → `parse_slash_command` → the new
//!      `BareCommand` arm → `handle_slash_command`).
//!   2. Real key events through the in-App `AgentView` (`handle_event`) —
//!      the end-to-end Tab-fill / Esc-dismiss repro from the bug report,
//!      where the view's `InputSubmitted` emission loops back onto the
//!      App's action bus.
//!
//! All assertions share one invariant: an intercepted bare command never
//! calls `backend.send_input` (and never `persistence_add_history`), and
//! lands the SAME observable effect a popup pick would produce.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{parse_slash_command, Action, App, FspecBackend, SlashCommandParse, ViewMode};
use codelet_rpc_types::{SessionId, ThinkingLevel, WorkUnitContext};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// Drain pending tasks AND any follow-up actions emitted onto the action
/// bus (the established RPC-022 / RPC-046 / RPC-054 idiom).
async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

/// Given: one open session s-1 in AgentView (the precondition shared by
/// every scenario in the feature file).
async fn given_app_in_agent_view(mock: &Arc<MockBackend>) -> (App, Arc<MockBackend>) {
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    // Flip to Agent view so the scenario's precondition holds (RPC-054
    // pattern — SessionCreated does not move the navigator).
    app.navigator_mut().active_view = ViewMode::Agent;
    drain_pending(&mut app).await;
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
    (app, mock.clone())
}

/// Type `s` char-by-char through the in-App AgentView (the real
/// MultiLineInput + popup overlay path).
fn type_chars(app: &mut App, s: &str) {
    for ch in s.chars() {
        app.navigator_mut()
            .agent
            .handle_event(&Event::Key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE)));
    }
}

fn press(app: &mut App, code: KeyCode) {
    app.navigator_mut()
        .agent
        .handle_event(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
}

/// The observable side effect of a popup-pick of the given bare command —
/// the same effect a typed submit must produce (R3).
fn assert_popup_pick_effect(app: &mut App, mock: &MockBackend, cmd: &str) {
    match cmd {
        "/help" => assert!(
            app.compositor().contains("help-dialog"),
            "/help must push the HelpDialog onto the Compositor"
        ),
        "/clear" => {
            assert_eq!(
                app.navigator().agent.chunk_count(app.agent_view_store()),
                0,
                "/clear must reset the focused session's scrollback"
            );
            assert_eq!(
                mock.clear_history_calls(),
                1,
                "/clear must call backend.clear_history"
            );
            assert_eq!(mock.last_clear_history_session(), Some(sid("s-1")));
        }
        "/quit" => assert!(
            app.should_quit(),
            "/quit must flip App.should_quit"
        ),
        "/resume" => assert!(
            app.navigator().agent.resume_view.is_some(),
            "/resume must open the resume mode view"
        ),
        "/search" => assert!(
            app.navigator().agent.search_view.is_some(),
            "/search must open the search mode view"
        ),
        "/provider" => assert_eq!(
            app.navigator().active_view,
            ViewMode::ProviderSettings,
            "/provider must flip the Navigator to ProviderSettings"
        ),
        "/debug" => {
            assert_eq!(
                mock.toggle_debug_calls(),
                1,
                "/debug must call backend.toggle_debug"
            );
            assert!(
                app.agent_view_store()
                    .current_session_context()
                    .expect("s-1 present")
                    .scrollback
                    .visible_window(1024)
                    .iter()
                    .flat_map(|c| c.lines.iter())
                    .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
                    .any(|t| t.contains("[debug] capture toggled")),
                "/debug must land a `[debug] capture toggled` scrollback notice"
            );
        }
        "/compact" => assert_eq!(
            mock.compact_session_calls(),
            1,
            "/compact must call backend.compact_session"
        ),
        "/isolation" => {
            // handle_slash_command(Isolation) emits OpenCreateSessionDialog
            // on the bus; drain it into the Compositor (idempotent push).
            if let Some(a) = app.try_recv_action() {
                match a {
                    Action::OpenCreateSessionDialog { preselect } => {
                        assert_eq!(preselect, Some(codelet_fspec_tui::CreateSessionOption::Isolated));
                        app.dispatch(a);
                    }
                    other => panic!("expected OpenCreateSessionDialog on bus, got {other:?}"),
                }
            }
            assert!(
                app.compositor().contains("create-session-dialog"),
                "/isolation must push the CreateSessionDialog (preselect Isolated)"
            );
        }
        "/blocklist" => assert_eq!(
            app.navigator().active_view,
            ViewMode::Blocklist,
            "/blocklist must flip the Navigator to Blocklist"
        ),
        "/detach" => {
            assert_eq!(
                mock.set_work_unit_context_calls(),
                1,
                "/detach must call backend.set_work_unit_context(s-1, None)"
            );
            assert_eq!(
                mock.last_set_work_unit_context(),
                Some((sid("s-1"), None)),
            );
            assert!(
                app.agent_view_store()
                    .work_unit_context_for(&sid("s-1"))
                    .is_none(),
                "/detach must clear the session's work-unit binding"
            );
        }
        "/merge-worktree" => assert_eq!(
            mock.inspect_session_changes_calls(),
            1,
            "/merge-worktree must call backend.inspect_session_changes"
        ),
        _ => panic!("unhandled command {cmd}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario Outline: Submitting a bare registered command routes to the
// popup-pick handler and never sends text to the LLM
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn submitting_bare_registered_commands_routes_to_the_popup_pick_handler() {
    for (cmd, _) in [
        ("/help", ""),
        ("/clear", ""),
        ("/quit", ""),
        ("/resume", ""),
        ("/search", ""),
        ("/provider", ""),
        ("/debug", ""),
        ("/compact", ""),
        ("/isolation", ""),
        ("/blocklist", ""),
        ("/detach", ""),
        ("/merge-worktree", ""),
    ] {
        let mock = Arc::new(MockBackend::new());
        let (mut app, _mock) = given_app_in_agent_view(&mock).await;
        // /detach needs a bound work unit so the backend path (not the
        // "no work unit attached" notice path) is exercised.
        if cmd == "/detach" {
            app.agent_view_store_mut().set_work_unit_context(
                sid("s-1"),
                WorkUnitContext {
                    id: "BUG-169".to_string(),
                    title: "BUG-169".to_string(),
                    status: "testing".to_string(),
                },
            );
        }
        let prior_send = mock.send_input_calls();
        // @step Given an App with one open session SessionId("s-1") in AgentView
        assert_eq!(app.navigator().active_view, ViewMode::Agent);
        // @step When the input is submitted with text "<cmd>"
        app.dispatch(Action::InputSubmitted(cmd.to_string()));
        drain_pending(&mut app).await;
        // @step Then the text is NOT forwarded to backend.send_input
        assert_eq!(
            mock.send_input_calls(),
            prior_send,
            "{cmd} must NOT be forwarded to backend.send_input"
        );
        // @step And the observable side effect <effect> lands (the same handler a popup pick would invoke)
        assert_popup_pick_effect(&mut app, &mock, cmd);
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Tab-fill then Enter on a typed command submits the command, not
// the text (reported bug)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tab_fill_then_enter_submits_the_command_not_the_text() {
    // @step Given an App with one open session SessionId("s-1") in AgentView
    let (mut app, mock) = given_app_in_agent_view(&Arc::new(MockBackend::new())).await;
    let prior_send = mock.send_input_calls();

    // @step When the user types "/provide" and the slash popup is open with "provider" highlighted
    type_chars(&mut app, "/provide");
    assert_eq!(
        app.navigator().agent.input.value(),
        "/provide",
        "buffer must hold the partial word"
    );
    let popup = app
        .navigator()
        .agent
        .slash_popup
        .as_ref()
        .expect("slash popup must be open while typing /provide");
    assert_eq!(popup.filter(), "provide");
    assert_eq!(popup.matches()[0].name(), "provider");

    // @step And the user presses Tab so the input fills with "/provider" and the popup closes
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.navigator().agent.input.value(), "/provider");
    assert!(app.navigator().agent.slash_popup.is_none());

    // @step And the user presses Enter
    press(&mut app, KeyCode::Enter);
    drain_pending(&mut app).await;

    // @step Then the text "/provider" is NOT forwarded to backend.send_input
    assert_eq!(
        mock.send_input_calls(),
        prior_send,
        "Tab-then-Enter must NOT send the literal '/provider' to the LLM"
    );
    // @step And the Navigator flips to ViewMode::ProviderSettings
    assert_eq!(
        app.navigator().active_view,
        ViewMode::ProviderSettings,
        "Tab-then-Enter must open the ProviderSettingsView"
    );
    // @step And the same effect is observed as a popup pick of the Provider command
    assert_popup_pick_effect(&mut app, &mock, "/provider");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc-dismiss then Enter on a typed command also intercepts
// (second trigger of the bug)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_dismiss_then_enter_also_intercepts() {
    // @step Given an App with one open session SessionId("s-1") in AgentView
    let (mut app, mock) = given_app_in_agent_view(&Arc::new(MockBackend::new())).await;
    let prior_send = mock.send_input_calls();

    // @step When the user types "/provider" and the slash popup is open
    type_chars(&mut app, "/provider");
    assert!(app.navigator().agent.slash_popup.is_some());
    assert_eq!(app.navigator().agent.input.value(), "/provider");

    // @step And the user presses Esc so the popup closes and the input buffer is unchanged ("/provider")
    press(&mut app, KeyCode::Esc);
    assert!(app.navigator().agent.slash_popup.is_none());
    assert_eq!(
        app.navigator().agent.input.value(),
        "/provider",
        "Esc must NOT modify the input buffer"
    );

    // @step And the user presses Enter
    press(&mut app, KeyCode::Enter);
    drain_pending(&mut app).await;

    // @step Then the text "/provider" is NOT forwarded to backend.send_input
    assert_eq!(
        mock.send_input_calls(),
        prior_send,
        "Esc-then-Enter must NOT send the literal '/provider' to the LLM"
    );
    // @step And the Navigator flips to ViewMode::ProviderSettings
    assert_eq!(
        app.navigator().active_view,
        ViewMode::ProviderSettings,
        "Esc-then-Enter must open the ProviderSettingsView"
    );
    // @step And the same effect is observed as a popup pick of the Provider command
    assert_popup_pick_effect(&mut app, &mock, "/provider");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A registered name with a trailing argument is NOT a bare
// command and goes to the LLM
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn registered_name_with_trailing_argument_goes_to_the_llm() {
    // @step Given an App with one open session SessionId("s-1") in AgentView
    let (mut app, mock) = given_app_in_agent_view(&Arc::new(MockBackend::new())).await;
    let prior_send = mock.send_input_calls();

    // @step When the input is submitted with text "/provider openai"
    app.dispatch(Action::InputSubmitted("/provider openai".to_string()));
    drain_pending(&mut app).await;

    // @step Then a tokio task is spawned that calls backend.send_input(SessionId("s-1"), "/provider openai")
    assert_eq!(mock.send_input_calls(), prior_send + 1);
    assert_eq!(
        mock.last_send_input(),
        Some((sid("s-1"), "/provider openai".to_string()))
    );
    // @step And the Navigator's active_view stays ViewMode::Agent (no ProviderSettings flip)
    assert_eq!(
        app.navigator().active_view,
        ViewMode::Agent,
        "a name-with-argument must NOT open the ProviderSettingsView"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Unknown slash lines are unchanged
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_slash_lines_still_go_to_the_llm() {
    // @step Given an App with one open session SessionId("s-1") in AgentView
    let (mut app, mock) = given_app_in_agent_view(&Arc::new(MockBackend::new())).await;
    let prior_send = mock.send_input_calls();

    // @step When the input is submitted with text "/unknown anything"
    app.dispatch(Action::InputSubmitted("/unknown anything".to_string()));
    drain_pending(&mut app).await;

    // @step Then a tokio task is spawned that calls backend.send_input(SessionId("s-1"), "/unknown anything")
    assert_eq!(mock.send_input_calls(), prior_send + 1);
    assert_eq!(
        mock.last_send_input(),
        Some((sid("s-1"), "/unknown anything".to_string()))
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario Outline: parse_slash_command resolves exact bare registered
// names case-insensitively and trims surrounding whitespace
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn parse_slash_command_resolves_bare_registered_names_to_their_action_variant() {
    use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
    // @step Given the function parse_slash_command from app/slash_parser.rs
    // @step When it is called with text=<input>
    // @step Then it returns the new variant BareCommand(<action>)
    assert_eq!(
        parse_slash_command("/provider"),
        SlashCommandParse::BareCommand(SlashCommandAction::Provider)
    );
    assert_eq!(
        parse_slash_command("/HELP"),
        SlashCommandParse::BareCommand(SlashCommandAction::Help)
    );
    assert_eq!(
        parse_slash_command("  /clear  "),
        SlashCommandParse::BareCommand(SlashCommandAction::Clear)
    );
    assert_eq!(
        parse_slash_command("/merge-worktree"),
        SlashCommandParse::BareCommand(SlashCommandAction::MergeWorktree)
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario Outline: parse_slash_command keeps NotASlashCommand for
// unregistered or argument-carrying lines
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn parse_slash_command_keeps_not_a_slash_command_for_unregistered_lines() {
    // @step Given the function parse_slash_command from app/slash_parser.rs
    // @step When it is called with text=<input>
    // @step Then it returns NotASlashCommand
    assert_eq!(
        parse_slash_command("/provider openai"),
        SlashCommandParse::NotASlashCommand
    );
    assert_eq!(
        parse_slash_command("/unknown"),
        SlashCommandParse::NotASlashCommand
    );
    assert_eq!(
        parse_slash_command("/"),
        SlashCommandParse::NotASlashCommand
    );
    assert_eq!(
        parse_slash_command("hello world"),
        SlashCommandParse::NotASlashCommand
    );
    // '/providers' is NOT a registered command (RPC-054: singular only).
    assert_eq!(
        parse_slash_command("/providers"),
        SlashCommandParse::NotASlashCommand
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Existing path-B families still parse to their existing variants
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn legacy_family_inputs_still_parse_to_their_existing_variants() {
    use codelet_fspec_tui::app::continue_parser::ContinueSubcommand;
    use codelet_fspec_tui::app::goal_parser::GoalSubcommand;
    use codelet_fspec_tui::app::loop_parser::LoopSubcommand;
    use codelet_fspec_tui::app::schedule_parser::ScheduleSubcommand;
    use codelet_fspec_tui::app::update_parser::UpdateSubcommand;
    // @step Given the function parse_slash_command from app/slash_parser.rs
    // @step When it is called with each of the legacy family inputs
    // @step Then it returns the same variant as before the fix:
    assert_eq!(
        parse_slash_command("/thinking high"),
        SlashCommandParse::SetThinkingLevel(ThinkingLevel::High)
    );
    assert_eq!(
        parse_slash_command("/role clear"),
        SlashCommandParse::ClearRole
    );
    assert_eq!(
        parse_slash_command("/goal"),
        SlashCommandParse::GoalSubcommand(GoalSubcommand::Show)
    );
    assert_eq!(
        parse_slash_command("/update check"),
        SlashCommandParse::UpdateSubcommand(UpdateSubcommand::CheckOnly)
    );
    assert_eq!(
        parse_slash_command("/schedule"),
        SlashCommandParse::ScheduleSubcommand(ScheduleSubcommand::Help)
    );
    assert_eq!(
        parse_slash_command("/loop list"),
        SlashCommandParse::LoopSubcommand(LoopSubcommand::List)
    );
    assert_eq!(
        parse_slash_command("/continue"),
        SlashCommandParse::ContinueSubcommand(ContinueSubcommand::Toggle)
    );
    assert_eq!(
        parse_slash_command("/model"),
        SlashCommandParse::OpenModelDialog
    );
    assert_eq!(
        parse_slash_command("/mux"),
        SlashCommandParse::MuxCommand("/mux".to_string())
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Intercepted bare commands do NOT append to the per-session
// history
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn intercepted_bare_commands_do_not_append_to_history() {
    // @step Given an App with one open session SessionId("s-1")
    let (mut app, mock) = given_app_in_agent_view(&Arc::new(MockBackend::new())).await;
    let prior_history = mock.persistence_add_history_calls();

    // @step When the input is submitted with text "/provider"
    app.dispatch(Action::InputSubmitted("/provider".to_string()));
    drain_pending(&mut app).await;

    // @step Then no tokio task is spawned that calls backend.persistence_add_history
    assert_eq!(
        mock.persistence_add_history_calls(),
        prior_history,
        "intercepted /provider must NOT call persistence_add_history (RPC-022 rule)"
    );

    // @step When the input is submitted with text "hello"
    app.dispatch(Action::InputSubmitted("hello".to_string()));
    drain_pending(&mut app).await;

    // @step Then exactly one tokio task is spawned that calls backend.persistence_add_history(SessionId("s-1"), "hello")
    assert_eq!(
        mock.persistence_add_history_calls(),
        prior_history + 1,
        "plain `hello` must call persistence_add_history exactly once"
    );
    assert_eq!(
        mock.last_persistence_add_history(),
        Some((sid("s-1"), "hello".to_string()))
    );
}
