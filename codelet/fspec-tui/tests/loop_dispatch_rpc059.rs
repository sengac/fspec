//! RPC-059 — /loop slash command + parser dispatch end-to-end.
//!
//! Feature: spec/features/rpc059-loop-dispatch.feature
//!
//! Drives the App::dispatch routing for `SlashCommandSelected(Loop)`
//! and for `Action::LoopSubcommandParsed(...)` through the matching
//! handle_loop_* helper, the backend round-trip, and the
//! `Action::EmitSessionNotice` notice formatter. Mirrors the
//! `schedule_dispatch_rpc058.rs` test layout.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::too_many_lines)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::app::loop_parser::{parse_loop_command, LoopSubcommand};
use codelet_fspec_tui::app::slash_parser::{parse_slash_command, SlashCommandParse};
use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{RegisteredLoop, SessionId};
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn reg(id: &str, session: &str, interval: u32, prompt: &str) -> RegisteredLoop {
    RegisteredLoop {
        id: id.to_string(),
        session_id: SessionId::new(session),
        prompt: prompt.to_string(),
        interval_seconds: interval,
        created_at: "2026-05-24T00:00:00Z".to_string(),
        expires_at: "2026-05-27T00:00:00Z".to_string(),
        last_run_at: None,
    }
}

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

async fn wait_until<F: FnMut() -> bool>(mut predicate: F, label: &str) {
    timeout(Duration::from_secs(1), async {
        loop {
            if predicate() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for: {label}"));
}

fn fresh_app(mock: Arc<MockBackend>) -> App {
    let backend: Arc<dyn FspecBackend> = mock;
    App::new(backend)
}

fn session_scrollback_text(app: &App, id: &SessionId) -> String {
    let chunks = app
        .agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default();
    chunks
        .iter()
        .flat_map(|c| {
            c.lines
                .iter()
                .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        })
        .collect::<Vec<String>>()
        .join("\n")
}

// ─────────────────────────────────────────────────────────────────────
// Parser scenarios
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parse_loop_command_resolves_bare_loop_to_help() {
    // @step When parse_loop_command("/loop") is invoked
    let out = parse_loop_command("/loop");
    // @step Then it returns LoopSubcommand::Help
    assert_eq!(out, LoopSubcommand::Help);
}

#[test]
fn parse_loop_command_resolves_loop_list() {
    // @step When parse_loop_command("/loop list") is invoked
    let out = parse_loop_command("/loop list");
    // @step Then it returns LoopSubcommand::List
    assert_eq!(out, LoopSubcommand::List);
}

#[test]
fn parse_loop_command_resolves_loop_cancel_id() {
    // @step When parse_loop_command("/loop cancel a1b2c3d4") is invoked
    let out = parse_loop_command("/loop cancel a1b2c3d4");
    // @step Then it returns LoopSubcommand::Cancel with id "a1b2c3d4"
    assert_eq!(
        out,
        LoopSubcommand::Cancel {
            id: "a1b2c3d4".to_string()
        }
    );
}

#[test]
fn parse_loop_command_resolves_leading_interval_seconds() {
    // @step When parse_loop_command("/loop 30s check the build") is invoked
    let out = parse_loop_command("/loop 30s check the build");
    // @step Then it returns LoopSubcommand::Add with interval_seconds 30 and prompt "check the build"
    assert_eq!(
        out,
        LoopSubcommand::Add {
            interval_seconds: 30,
            prompt: "check the build".to_string(),
        }
    );
}

#[test]
fn parse_loop_command_resolves_leading_interval_minutes() {
    // @step When parse_loop_command("/loop 5m check deployment status") is invoked
    let out = parse_loop_command("/loop 5m check deployment status");
    // @step Then it returns LoopSubcommand::Add with interval_seconds 300 and prompt "check deployment status"
    assert_eq!(
        out,
        LoopSubcommand::Add {
            interval_seconds: 300,
            prompt: "check deployment status".to_string(),
        }
    );
}

#[test]
fn parse_loop_command_resolves_leading_interval_hours() {
    // @step When parse_loop_command("/loop 2h check build") is invoked
    let out = parse_loop_command("/loop 2h check build");
    // @step Then it returns LoopSubcommand::Add with interval_seconds 7200 and prompt "check build"
    assert_eq!(
        out,
        LoopSubcommand::Add {
            interval_seconds: 7200,
            prompt: "check build".to_string(),
        }
    );
}

#[test]
fn parse_loop_command_resolves_leading_interval_days() {
    // @step When parse_loop_command("/loop 1d nightly summary") is invoked
    let out = parse_loop_command("/loop 1d nightly summary");
    // @step Then it returns LoopSubcommand::Add with interval_seconds 86400 and prompt "nightly summary"
    assert_eq!(
        out,
        LoopSubcommand::Add {
            interval_seconds: 86400,
            prompt: "nightly summary".to_string(),
        }
    );
}

#[test]
fn parse_loop_command_resolves_trailing_every_clause() {
    // @step When parse_loop_command("/loop check status every 2 hours") is invoked
    let out = parse_loop_command("/loop check status every 2 hours");
    // @step Then it returns LoopSubcommand::Add with interval_seconds 7200 and prompt "check status"
    assert_eq!(
        out,
        LoopSubcommand::Add {
            interval_seconds: 7200,
            prompt: "check status".to_string(),
        }
    );
}

#[test]
fn parse_loop_command_defaults_to_10_minutes_when_no_interval_specified() {
    // @step When parse_loop_command("/loop check the build") is invoked
    let out = parse_loop_command("/loop check the build");
    // @step Then it returns LoopSubcommand::Add with interval_seconds 600 and prompt "check the build"
    assert_eq!(
        out,
        LoopSubcommand::Add {
            interval_seconds: 600,
            prompt: "check the build".to_string(),
        }
    );
}

#[test]
fn parse_loop_command_treats_minimum_interval_as_1_second() {
    // @step When parse_loop_command("/loop 0s prompt") is invoked
    let out = parse_loop_command("/loop 0s prompt");
    // @step Then it returns LoopSubcommand::Add with interval_seconds 1 and prompt "prompt"
    assert_eq!(
        out,
        LoopSubcommand::Add {
            interval_seconds: 1,
            prompt: "prompt".to_string(),
        }
    );
}

// ─────────────────────────────────────────────────────────────────────
// slash_parser interception
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parse_slash_command_routes_loop_submit_line_input() {
    // @step When parse_slash_command("/loop list") is invoked
    let out = parse_slash_command("/loop list");
    // @step Then it returns SlashCommandParse::LoopSubcommand(LoopSubcommand::List)
    assert_eq!(
        out,
        SlashCommandParse::LoopSubcommand(LoopSubcommand::List)
    );
}

#[test]
fn parse_slash_command_routes_bare_loop_to_help() {
    // @step When parse_slash_command("/loop") is invoked
    let out = parse_slash_command("/loop");
    // @step Then it returns SlashCommandParse::LoopSubcommand(LoopSubcommand::Help)
    assert_eq!(
        out,
        SlashCommandParse::LoopSubcommand(LoopSubcommand::Help)
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: /loop popup pick with no current session is a silent no-op
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn loop_popup_pick_with_no_session_is_silent_noop() {
    // @step Given an App with NO open AgentView session
    let mock = Arc::new(MockBackend::new());
    let mut app = fresh_app(mock.clone());

    let initial_add = mock.loop_add_calls();
    let initial_list = mock.loop_list_calls();
    let initial_cancel = mock.loop_cancel_calls();

    // @step When SlashCommandSelected(SlashCommandAction::Loop) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Loop));
    drain_pending(&mut app).await;

    // @step Then no backend method is called
    assert_eq!(mock.loop_add_calls(), initial_add);
    assert_eq!(mock.loop_list_calls(), initial_list);
    assert_eq!(mock.loop_cancel_calls(), initial_cancel);

    // @step And no scrollback notice is emitted
    assert!(
        app.agent_view_store().open_sessions().is_empty(),
        "expected zero open sessions",
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: /loop popup pick with an open session emits the Help notice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn loop_popup_pick_with_session_emits_help_notice() {
    // @step Given an App with open session s-1
    let mock = Arc::new(MockBackend::new());
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    let initial_list = mock.loop_list_calls();

    // @step When SlashCommandSelected(SlashCommandAction::Loop) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Loop));
    drain_pending(&mut app).await;

    // @step Then Action::EmitSessionNotice for s-1 with text starting with "[loop] Usage:" is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1")).contains("[loop] Usage:"),
        "loop help notice",
    )
    .await;

    // @step And no backend method is called
    assert_eq!(mock.loop_list_calls(), initial_list);
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: /loop list with two loops emits a multi-line list notice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn loop_list_two_rows_emits_multiline_notice() {
    // @step Given an App with open session s-1 wired to a MockBackend whose loop_list returns two RegisteredLoop rows
    let mock = Arc::new(MockBackend::new());
    mock.seed_loop_list_result(Ok(vec![
        reg("a1b2c3d4", "s-1", 30, "check build"),
        reg("e5f6g7h8", "s-1", 300, "check status"),
    ]));
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    let initial = mock.loop_list_calls();

    // @step When Action::LoopSubcommandParsed(LoopSubcommand::List) is dispatched
    app.dispatch(Action::LoopSubcommandParsed(LoopSubcommand::List));
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.loop_list is called exactly once with session_id s-1
    wait_until(
        || mock.loop_list_calls() - initial == 1,
        "loop_list called once",
    )
    .await;

    // @step And within 1 second Action::EmitSessionNotice for s-1 with text containing "Active loops:" is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1")).contains("Active loops:"),
        "loop list notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: /loop list with no loops emits "No active loops."
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn loop_list_empty_emits_no_loops_notice() {
    // @step Given an App with open session s-1 wired to a MockBackend whose loop_list returns an empty Vec
    let mock = Arc::new(MockBackend::new());
    mock.seed_loop_list_result(Ok(vec![]));
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    // @step When Action::LoopSubcommandParsed(LoopSubcommand::List) is dispatched
    app.dispatch(Action::LoopSubcommandParsed(LoopSubcommand::List));
    drain_pending(&mut app).await;

    // @step Then within 1 second Action::EmitSessionNotice for s-1 with text "[loop] No active loops." is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1")).contains("[loop] No active loops."),
        "no-loops notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: /loop add success emits the "scheduled" notice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn loop_add_success_emits_scheduled_notice() {
    // @step Given an App with open session s-1 wired to a MockBackend whose loop_add returns Ok(RegisteredLoop { id: "ab12cd34", session_id: SessionId::new("s-1"), prompt: "check the build", interval_seconds: 30, created_at: "2026-05-24T00:00:00Z", expires_at: "2026-05-27T00:00:00Z", last_run_at: None })
    let mock = Arc::new(MockBackend::new());
    mock.seed_loop_add_result(Ok(reg("ab12cd34", "s-1", 30, "check the build")));
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    let initial = mock.loop_add_calls();

    // @step When Action::LoopSubcommandParsed(LoopSubcommand::Add { interval_seconds: 30, prompt: "check the build" }) is dispatched
    app.dispatch(Action::LoopSubcommandParsed(LoopSubcommand::Add {
        interval_seconds: 30,
        prompt: "check the build".to_string(),
    }));
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.loop_add is called exactly once with session_id s-1 and interval_seconds 30 and prompt "check the build"
    wait_until(
        || mock.loop_add_calls() - initial == 1,
        "loop_add called once",
    )
    .await;

    // @step And within 1 second Action::EmitSessionNotice for s-1 with text "[loop] scheduled every 30 seconds [job: ab12cd34]" is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1"))
            .contains("[loop] scheduled every 30 seconds [job: ab12cd34]"),
        "loop add notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: /loop add error emits an error notice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn loop_add_error_emits_error_notice() {
    // @step Given an App with open session s-1 wired to a MockBackend whose loop_add returns Err("Session not found: s-1")
    let mock = Arc::new(MockBackend::new());
    mock.seed_loop_add_result(Err("Session not found: s-1".to_string()));
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    // @step When Action::LoopSubcommandParsed(LoopSubcommand::Add { interval_seconds: 30, prompt: "p" }) is dispatched
    app.dispatch(Action::LoopSubcommandParsed(LoopSubcommand::Add {
        interval_seconds: 30,
        prompt: "p".to_string(),
    }));
    drain_pending(&mut app).await;

    // @step Then within 1 second Action::EmitSessionNotice for s-1 with text "[error] /loop add: Session not found: s-1" is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1"))
            .contains("[error] /loop add: Session not found: s-1"),
        "add error notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: /loop cancel success emits the "cancelled" notice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn loop_cancel_success_emits_cancelled_notice() {
    // @step Given an App with open session s-1 wired to a MockBackend whose loop_cancel returns Ok(true)
    let mock = Arc::new(MockBackend::new());
    mock.seed_loop_cancel_result(Ok(true));
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    let initial = mock.loop_cancel_calls();

    // @step When Action::LoopSubcommandParsed(LoopSubcommand::Cancel { id: "a1b2c3d4" }) is dispatched
    app.dispatch(Action::LoopSubcommandParsed(LoopSubcommand::Cancel {
        id: "a1b2c3d4".to_string(),
    }));
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.loop_cancel is called exactly once with id "a1b2c3d4"
    wait_until(
        || mock.loop_cancel_calls() - initial == 1,
        "loop_cancel called once",
    )
    .await;

    // @step And within 1 second Action::EmitSessionNotice for s-1 with text "[loop] cancelled a1b2c3d4" is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1"))
            .contains("[loop] cancelled a1b2c3d4"),
        "loop cancel notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: /loop cancel unknown id emits a "not found" error notice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn loop_cancel_unknown_emits_not_found_notice() {
    // @step Given an App with open session s-1 wired to a MockBackend whose loop_cancel returns Ok(false)
    let mock = Arc::new(MockBackend::new());
    mock.seed_loop_cancel_result(Ok(false));
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    // @step When Action::LoopSubcommandParsed(LoopSubcommand::Cancel { id: "does-not-exist" }) is dispatched
    app.dispatch(Action::LoopSubcommandParsed(LoopSubcommand::Cancel {
        id: "does-not-exist".to_string(),
    }));
    drain_pending(&mut app).await;

    // @step Then within 1 second Action::EmitSessionNotice for s-1 with text "[error] /loop cancel: Loop \"does-not-exist\" not found" is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1"))
            .contains("[error] /loop cancel: Loop \"does-not-exist\" not found"),
        "loop cancel missing notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Bare /loop submit-line input emits the Help notice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bare_loop_submit_emits_help_notice() {
    // @step Given an App with open session s-1
    let mock = Arc::new(MockBackend::new());
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    let initial_add = mock.loop_add_calls();
    let initial_list = mock.loop_list_calls();
    let initial_cancel = mock.loop_cancel_calls();

    // @step When Action::LoopSubcommandParsed(LoopSubcommand::Help) is dispatched
    app.dispatch(Action::LoopSubcommandParsed(LoopSubcommand::Help));
    drain_pending(&mut app).await;

    // @step Then no backend method is called
    assert_eq!(mock.loop_add_calls(), initial_add);
    assert_eq!(mock.loop_list_calls(), initial_list);
    assert_eq!(mock.loop_cancel_calls(), initial_cancel);

    // @step And Action::EmitSessionNotice for s-1 with text starting with "[loop] Usage:" is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1")).contains("[loop] Usage:"),
        "loop help notice",
    )
    .await;
}
