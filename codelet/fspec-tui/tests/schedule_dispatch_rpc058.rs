//! RPC-058 — /schedule slash command + parser dispatch end-to-end.
//!
//! Feature: spec/features/rpc058-schedule-dispatch.feature
//!
//! Drives the App::dispatch routing for `SlashCommandSelected(Schedule)`
//! and for `Action::ScheduleSubcommandParsed(...)` through the matching
//! handle_schedule_* helper, the backend round-trip, and the
//! `Action::EmitSessionNotice` notice formatter. Mirrors the
//! `merge_worktree_rpc057.rs` test layout.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::app::schedule_parser::{parse_schedule_command, ScheduleSubcommand};
use codelet_fspec_tui::app::slash_parser::{parse_slash_command, SlashCommandParse};
use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{ScheduledJob, SessionId};
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn job(name: &str, status: &str, job_type: &str) -> ScheduledJob {
    ScheduledJob {
        name: name.to_string(),
        cron: "0 9 * * *".to_string(),
        timezone: "UTC".to_string(),
        job_type: job_type.to_string(),
        status: status.to_string(),
        created_at: None,
        last_run_at: None,
        last_run_status: None,
        role: Some("reviewer".to_string()),
        prompt: Some("daily standup".to_string()),
        command: None,
        overlap_policy: Some("skip".to_string()),
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

// ─────────────────────────────────────────────────────────────────────
// Scenario: /schedule popup pick with no current session is a silent no-op
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schedule_popup_pick_with_no_session_is_silent_noop() {
    // @step Given an App with NO open AgentView session
    let mock = Arc::new(MockBackend::new());
    let mut app = fresh_app(mock.clone());

    let initial_add = mock.schedule_add_calls();
    let initial_list = mock.schedule_list_calls();

    // @step When SlashCommandSelected(SlashCommandAction::Schedule) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Schedule));
    drain_pending(&mut app).await;

    // @step Then no backend method is called
    assert_eq!(mock.schedule_add_calls(), initial_add);
    assert_eq!(mock.schedule_list_calls(), initial_list);

    // @step And no scrollback notice is emitted
    // With no AgentView session at all, there is no SessionContext to
    // emit a notice into. Pin the invariant by asserting the
    // open_sessions slice is empty — any leaked notice would have
    // spawned a context via Action::SessionCreated.
    assert!(
        app.agent_view_store().open_sessions().is_empty(),
        "expected zero open sessions; got {} — a notice may have spawned a context",
        app.agent_view_store().open_sessions().len()
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: /schedule popup pick with an open session emits the Help notice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schedule_popup_pick_with_session_emits_help_notice() {
    // @step Given an App with open session s-1
    let mock = Arc::new(MockBackend::new());
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    let initial_list = mock.schedule_list_calls();

    // @step When SlashCommandSelected(SlashCommandAction::Schedule) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Schedule));
    drain_pending(&mut app).await;

    // @step Then Action::EmitSessionNotice for s-1 with text starting with "[schedule] Usage: /schedule" is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1")).contains("[schedule] Usage: /schedule"),
        "schedule help notice",
    )
    .await;

    // @step And no backend method is called
    assert_eq!(mock.schedule_list_calls(), initial_list);
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: /schedule list with two schedules emits a multi-line list notice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schedule_list_two_rows_emits_multiline_notice() {
    // @step Given an App with open session s-1 wired to a MockBackend whose schedule_list returns two ScheduledJob rows
    let mock = Arc::new(MockBackend::new());
    mock.seed_schedule_list_result(Ok(vec![
        job("daily", "active", "agent"),
        job("backup", "active", "shell"),
    ]));
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    let initial = mock.schedule_list_calls();

    // @step When Action::ScheduleSubcommandParsed(ScheduleSubcommand::List) is dispatched
    app.dispatch(Action::ScheduleSubcommandParsed(ScheduleSubcommand::List));
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.schedule_list is called exactly once
    wait_until(
        || mock.schedule_list_calls() - initial == 1,
        "schedule_list called once",
    )
    .await;

    // @step And within 1 second Action::EmitSessionNotice for s-1 with text containing "[schedule] 2 schedule(s)" is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1")).contains("[schedule] 2 schedule(s)"),
        "schedule list notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: /schedule list with no schedules emits "No schedules configured."
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schedule_list_empty_emits_no_schedules_notice() {
    // @step Given an App with open session s-1 wired to a MockBackend whose schedule_list returns an empty Vec
    let mock = Arc::new(MockBackend::new());
    mock.seed_schedule_list_result(Ok(vec![]));
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    // @step When Action::ScheduleSubcommandParsed(ScheduleSubcommand::List) is dispatched
    app.dispatch(Action::ScheduleSubcommandParsed(ScheduleSubcommand::List));
    drain_pending(&mut app).await;

    // @step Then within 1 second Action::EmitSessionNotice for s-1 with text "[schedule] No schedules configured." is observed on the action bus
    wait_until(
        || {
            session_scrollback_text(&app, &sid("s-1"))
                .contains("[schedule] No schedules configured.")
        },
        "no-schedules notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: /schedule add success emits the "added" notice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schedule_add_success_emits_added_notice() {
    // @step Given an App with open session s-1 wired to a MockBackend whose schedule_add returns Ok(ScheduledJob { name: "daily", cron: "0 9 * * *", timezone: "UTC", job_type: "agent", status: "active", role: Some("reviewer"), prompt: Some("daily standup"), command: None, overlap_policy: Some("skip"), created_at: None, last_run_at: None, last_run_status: None })
    let mock = Arc::new(MockBackend::new());
    mock.seed_schedule_add_result(Ok(job("daily", "active", "agent")));
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    let initial = mock.schedule_add_calls();

    // @step When Action::ScheduleSubcommandParsed(ScheduleSubcommand::Add { name: "daily", cron: "0 9 * * *", timezone: "UTC", job_type: "agent", role: Some("reviewer"), prompt: Some("daily standup"), command: None, overlap_policy: Some("skip") }) is dispatched
    app.dispatch(Action::ScheduleSubcommandParsed(ScheduleSubcommand::Add {
        name: "daily".to_string(),
        cron: "0 9 * * *".to_string(),
        timezone: "UTC".to_string(),
        job_type: "agent".to_string(),
        role: Some("reviewer".to_string()),
        prompt: Some("daily standup".to_string()),
        command: None,
        overlap_policy: Some("skip".to_string()),
    }));
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.schedule_add is called exactly once with the matching arguments
    wait_until(
        || mock.schedule_add_calls() - initial == 1,
        "schedule_add called once",
    )
    .await;

    // @step And within 1 second Action::EmitSessionNotice for s-1 with text "[schedule] added \"daily\" (agent, 0 9 * * *, UTC)" is observed on the action bus
    wait_until(
        || {
            session_scrollback_text(&app, &sid("s-1"))
                .contains("[schedule] added \"daily\" (agent, 0 9 * * *, UTC)")
        },
        "added notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: /schedule add error emits an error notice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schedule_add_error_emits_error_notice() {
    // @step Given an App with open session s-1 wired to a MockBackend whose schedule_add returns Err("Timezone is required")
    let mock = Arc::new(MockBackend::new());
    mock.seed_schedule_add_result(Err("Timezone is required".to_string()));
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    // @step When Action::ScheduleSubcommandParsed(ScheduleSubcommand::Add { name: "daily", cron: "0 9 * * *", timezone: "", job_type: "agent", role: Some("r"), prompt: Some("p"), command: None, overlap_policy: None }) is dispatched
    app.dispatch(Action::ScheduleSubcommandParsed(ScheduleSubcommand::Add {
        name: "daily".to_string(),
        cron: "0 9 * * *".to_string(),
        timezone: "".to_string(),
        job_type: "agent".to_string(),
        role: Some("r".to_string()),
        prompt: Some("p".to_string()),
        command: None,
        overlap_policy: None,
    }));
    drain_pending(&mut app).await;

    // @step Then within 1 second Action::EmitSessionNotice for s-1 with text "[error] /schedule add: Timezone is required" is observed on the action bus
    wait_until(
        || {
            session_scrollback_text(&app, &sid("s-1"))
                .contains("[error] /schedule add: Timezone is required")
        },
        "add error notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: /schedule pause success emits the "paused" notice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schedule_pause_success_emits_paused_notice() {
    // @step Given an App with open session s-1 wired to a MockBackend whose schedule_pause returns Ok(ScheduledJob with status "paused")
    let mock = Arc::new(MockBackend::new());
    mock.seed_schedule_pause_result(Ok(job("daily", "paused", "agent")));
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    let initial = mock.schedule_pause_calls();

    // @step When Action::ScheduleSubcommandParsed(ScheduleSubcommand::Pause { name: "daily" }) is dispatched
    app.dispatch(Action::ScheduleSubcommandParsed(
        ScheduleSubcommand::Pause {
            name: "daily".to_string(),
        },
    ));
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.schedule_pause is called exactly once with name "daily"
    wait_until(
        || mock.schedule_pause_calls() - initial == 1,
        "schedule_pause called once",
    )
    .await;

    // @step And within 1 second Action::EmitSessionNotice for s-1 with text "[schedule] paused \"daily\"" is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1")).contains("[schedule] paused \"daily\""),
        "paused notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: /schedule pause unknown schedule emits an error notice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schedule_pause_unknown_emits_error_notice() {
    // @step Given an App with open session s-1 wired to a MockBackend whose schedule_pause returns Err("Schedule not found: unknown-job")
    let mock = Arc::new(MockBackend::new());
    mock.seed_schedule_pause_result(Err("Schedule not found: unknown-job".to_string()));
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    // @step When Action::ScheduleSubcommandParsed(ScheduleSubcommand::Pause { name: "unknown-job" }) is dispatched
    app.dispatch(Action::ScheduleSubcommandParsed(
        ScheduleSubcommand::Pause {
            name: "unknown-job".to_string(),
        },
    ));
    drain_pending(&mut app).await;

    // @step Then within 1 second Action::EmitSessionNotice for s-1 with text "[error] /schedule pause: Schedule not found: unknown-job" is observed on the action bus
    wait_until(
        || {
            session_scrollback_text(&app, &sid("s-1"))
                .contains("[error] /schedule pause: Schedule not found: unknown-job")
        },
        "pause error notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: /schedule resume success emits the "resumed" notice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schedule_resume_success_emits_resumed_notice() {
    // @step Given an App with open session s-1 wired to a MockBackend whose schedule_resume returns Ok(ScheduledJob with status "active")
    let mock = Arc::new(MockBackend::new());
    mock.seed_schedule_resume_result(Ok(job("daily", "active", "agent")));
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    let initial = mock.schedule_resume_calls();

    // @step When Action::ScheduleSubcommandParsed(ScheduleSubcommand::Resume { name: "daily" }) is dispatched
    app.dispatch(Action::ScheduleSubcommandParsed(
        ScheduleSubcommand::Resume {
            name: "daily".to_string(),
        },
    ));
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.schedule_resume is called exactly once with name "daily"
    wait_until(
        || mock.schedule_resume_calls() - initial == 1,
        "schedule_resume called once",
    )
    .await;

    // @step And within 1 second Action::EmitSessionNotice for s-1 with text "[schedule] resumed \"daily\"" is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1")).contains("[schedule] resumed \"daily\""),
        "resumed notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: /schedule remove success emits the "removed" notice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schedule_remove_success_emits_removed_notice() {
    // @step Given an App with open session s-1 wired to a MockBackend whose schedule_remove returns Ok(())
    let mock = Arc::new(MockBackend::new());
    mock.seed_schedule_remove_result(Ok(()));
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    let initial = mock.schedule_remove_calls();

    // @step When Action::ScheduleSubcommandParsed(ScheduleSubcommand::Remove { name: "daily" }) is dispatched
    app.dispatch(Action::ScheduleSubcommandParsed(
        ScheduleSubcommand::Remove {
            name: "daily".to_string(),
        },
    ));
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.schedule_remove is called exactly once with name "daily"
    wait_until(
        || mock.schedule_remove_calls() - initial == 1,
        "schedule_remove called once",
    )
    .await;

    // @step And within 1 second Action::EmitSessionNotice for s-1 with text "[schedule] removed \"daily\"" is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1")).contains("[schedule] removed \"daily\""),
        "removed notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Bare /schedule submit-line input emits the Help notice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bare_schedule_submit_emits_help_notice() {
    // @step Given an App with open session s-1
    let mock = Arc::new(MockBackend::new());
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    let initial_list = mock.schedule_list_calls();
    let initial_add = mock.schedule_add_calls();

    // @step When Action::ScheduleSubcommandParsed(ScheduleSubcommand::Help) is dispatched
    app.dispatch(Action::ScheduleSubcommandParsed(ScheduleSubcommand::Help));
    drain_pending(&mut app).await;

    // @step Then no backend method is called
    assert_eq!(mock.schedule_list_calls(), initial_list);
    assert_eq!(mock.schedule_add_calls(), initial_add);

    // @step And Action::EmitSessionNotice for s-1 with text starting with "[schedule] Usage: /schedule" is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1")).contains("[schedule] Usage: /schedule"),
        "help notice from bare schedule submit",
    )
    .await;
}

// ═══════════════════════════════════════════════════════════════════════
// PARSER unit tests (moved from schedule_parser_rpc058.rs so the
// dispatch feature file maps to a single test file per fspec's 1:1
// invariant).
// ═══════════════════════════════════════════════════════════════════════

// ─────────────────────────────────────────────────────────────────────
// Scenario: parse_schedule_command resolves bare /schedule to Help
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parse_schedule_command_bare_resolves_to_help() {
    // @step When parse_schedule_command("/schedule") is invoked
    let result = parse_schedule_command("/schedule");

    // @step Then it returns ScheduleSubcommand::Help
    assert!(matches!(result, ScheduleSubcommand::Help));
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: parse_schedule_command resolves /schedule list
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parse_schedule_command_list_resolves_to_list() {
    // @step When parse_schedule_command("/schedule list") is invoked
    let result = parse_schedule_command("/schedule list");

    // @step Then it returns ScheduleSubcommand::List
    assert!(matches!(result, ScheduleSubcommand::List));
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: parse_schedule_command resolves a full /schedule add agent command
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parse_schedule_command_full_add_agent_command() {
    // @step When parse_schedule_command is invoked on "/schedule add daily --cron \"0 9 * * *\" --tz UTC --role reviewer --prompt \"daily standup\""
    let input = r#"/schedule add daily --cron "0 9 * * *" --tz UTC --role reviewer --prompt "daily standup""#;
    let result = parse_schedule_command(input);

    // @step Then it returns ScheduleSubcommand::Add with name "daily" and cron "0 9 * * *" and timezone "UTC" and job_type "agent" and role Some("reviewer") and prompt Some("daily standup") and command None
    match result {
        ScheduleSubcommand::Add {
            name,
            cron,
            timezone,
            job_type,
            role,
            prompt,
            command,
            ..
        } => {
            assert_eq!(name, "daily");
            assert_eq!(cron, "0 9 * * *");
            assert_eq!(timezone, "UTC");
            assert_eq!(job_type, "agent");
            assert_eq!(role.as_deref(), Some("reviewer"));
            assert_eq!(prompt.as_deref(), Some("daily standup"));
            assert!(command.is_none());
        }
        other => panic!("expected ScheduleSubcommand::Add, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: parse_schedule_command infers shell job_type from --command flag
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parse_schedule_command_shell_inferred_from_command_flag() {
    // @step When parse_schedule_command is invoked on "/schedule add backup --cron \"0 2 * * *\" --tz UTC --command \"tar -czf /tmp/backup.tar.gz ~/work\""
    let input = r#"/schedule add backup --cron "0 2 * * *" --tz UTC --command "tar -czf /tmp/backup.tar.gz ~/work""#;
    let result = parse_schedule_command(input);

    // @step Then it returns ScheduleSubcommand::Add with name "backup" and job_type "shell" and command Some("tar -czf /tmp/backup.tar.gz ~/work") and role None and prompt None
    match result {
        ScheduleSubcommand::Add {
            name,
            job_type,
            command,
            role,
            prompt,
            ..
        } => {
            assert_eq!(name, "backup");
            assert_eq!(job_type, "shell");
            assert_eq!(
                command.as_deref(),
                Some("tar -czf /tmp/backup.tar.gz ~/work")
            );
            assert!(role.is_none());
            assert!(prompt.is_none());
        }
        other => panic!("expected ScheduleSubcommand::Add, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: parse_schedule_command resolves /schedule pause <name>
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parse_schedule_command_pause_resolves_with_name() {
    // @step When parse_schedule_command("/schedule pause daily") is invoked
    let result = parse_schedule_command("/schedule pause daily");

    // @step Then it returns ScheduleSubcommand::Pause with name "daily"
    match result {
        ScheduleSubcommand::Pause { name } => assert_eq!(name, "daily"),
        other => panic!("expected ScheduleSubcommand::Pause, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: parse_schedule_command resolves /schedule resume <name>
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parse_schedule_command_resume_resolves_with_name() {
    // @step When parse_schedule_command("/schedule resume daily") is invoked
    let result = parse_schedule_command("/schedule resume daily");

    // @step Then it returns ScheduleSubcommand::Resume with name "daily"
    match result {
        ScheduleSubcommand::Resume { name } => assert_eq!(name, "daily"),
        other => panic!("expected ScheduleSubcommand::Resume, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: parse_schedule_command resolves /schedule remove <name>
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parse_schedule_command_remove_resolves_with_name() {
    // @step When parse_schedule_command("/schedule remove daily") is invoked
    let result = parse_schedule_command("/schedule remove daily");

    // @step Then it returns ScheduleSubcommand::Remove with name "daily"
    match result {
        ScheduleSubcommand::Remove { name } => assert_eq!(name, "daily"),
        other => panic!("expected ScheduleSubcommand::Remove, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: parse_schedule_command falls back to Help on an unknown subcommand
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parse_schedule_command_unknown_falls_back_to_help() {
    // @step When parse_schedule_command("/schedule frobnicate") is invoked
    let result = parse_schedule_command("/schedule frobnicate");

    // @step Then it returns ScheduleSubcommand::Help
    assert!(matches!(result, ScheduleSubcommand::Help));
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: parse_slash_command routes a /schedule submit-line input to ScheduleSubcommand
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parse_slash_command_routes_schedule_to_subcommand_variant() {
    // @step When parse_slash_command("/schedule list") is invoked
    let result = parse_slash_command("/schedule list");

    // @step Then it returns SlashCommandParse::ScheduleSubcommand(ScheduleSubcommand::List)
    match result {
        SlashCommandParse::ScheduleSubcommand(sub) => {
            assert!(matches!(sub, ScheduleSubcommand::List));
        }
        other => panic!("expected SlashCommandParse::ScheduleSubcommand, got {other:?}"),
    }
}
