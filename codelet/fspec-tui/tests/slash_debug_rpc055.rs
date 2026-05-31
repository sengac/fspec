//! RPC-055 — `/debug` slash command end-to-end (Phase 7.2 of RPC-030).
//!
//! Feature: spec/features/rpc055-slash-debug-dispatch.feature
//!
//! Drives the App::dispatch routing for `SlashCommandSelected(Debug)` so
//! it reaches the backend's `toggle_debug(session_id, debug_dir)` RPC.
//! On success the focused session's scrollback gains a `[debug] capture
//! toggled → {file_path}` notice; on failure it gains a `[error] /debug
//! failed: {reason}` notice. With no current session, /debug is a silent
//! no-op. Background sessions are never touched.
//!
//! Mirrors the spawned-task + action-bus round-trip pattern established
//! by RPC-046 (`/clear`) and RPC-054 (`/provider`).
//!
//! ALSO covers the SessionHeader `[DEBUG]` badge wiring (the
//! `is_debug_enabled` field in `views/agent.rs` reads from
//! `agent_view_store.debug_enabled_for(session_id)` instead of
//! hardcoding to false).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use tokio::time::timeout;

mod common;
use common::MockBackend;

/// Process-wide serialisation guard for env-var-sensitive tests.
/// Tests that read OR write `FSPEC_DEBUG_DIR` MUST hold this lock for
/// their entire duration so the cross-test race observed in the very
/// first /debug dispatch run (where a sibling tokio test had set the
/// env var while we were asserting the default) cannot recur.
static FSPEC_DEBUG_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
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

fn session_chunk_count(app: &App, id: &SessionId) -> usize {
    app.agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.chunk_count())
        .unwrap_or(0)
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

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /debug calls backend.toggle_debug for the focused session
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::await_holding_lock)] // FSPEC_DEBUG_DIR_LOCK serialises env-var-touching tests; std::sync::Mutex is intentional.
async fn debug_calls_backend_toggle_debug_for_focused_session() {
    // Serialise with the FSPEC_DEBUG_DIR-setting test so the env var
    // can't race underneath us.
    let _guard = FSPEC_DEBUG_DIR_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    std::env::remove_var("FSPEC_DEBUG_DIR");

    // @step Given an App with an open session s-1 wired to a MockBackend whose toggle_debug returns Ok("/tmp/debug/s-1/session-x.jsonl")
    let mock = Arc::new(MockBackend::new());
    mock.set_toggle_debug_result_ok("/tmp/debug/s-1/session-x.jsonl".to_string());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Debug));

    // @step Then within 1 second backend.toggle_debug is called exactly once with session_id s-1
    wait_until(
        || mock.toggle_debug_calls() == 1,
        "backend.toggle_debug call count to reach 1",
    )
    .await;
    assert_eq!(mock.toggle_debug_calls(), 1);
    let last = mock.last_toggle_debug().expect("toggle_debug captured");
    assert_eq!(last.0, sid("s-1"));

    // @step And the debug_dir argument equals ".fspec/debug"
    assert_eq!(last.1, ".fspec/debug");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /debug emits a success notice with the resolved file path on Ok
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debug_emits_success_notice_with_resolved_file_path_on_ok() {
    // @step Given an App with an open session s-1 wired to a MockBackend whose toggle_debug returns Ok("/tmp/debug/s-1/session-x.jsonl")
    let mock = Arc::new(MockBackend::new());
    mock.set_toggle_debug_result_ok("/tmp/debug/s-1/session-x.jsonl".to_string());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Debug));

    drain_pending(&mut app).await;

    // @step Then within 1 second s-1's scrollback contains a chunk whose text equals "[debug] capture toggled → /tmp/debug/s-1/session-x.jsonl"
    let text = session_scrollback_text(&app, &sid("s-1"));
    assert!(
        text.lines()
            .any(|l| l == "[debug] capture toggled \u{2192} /tmp/debug/s-1/session-x.jsonl"),
        "expected `[debug] capture toggled → /tmp/debug/s-1/session-x.jsonl` line in s-1 scrollback, got {text:?}",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /debug emits an error notice on Err
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debug_emits_error_notice_on_err() {
    // @step Given an App with an open session s-1 wired to a MockBackend whose toggle_debug returns Err("disk full")
    let mock = Arc::new(MockBackend::new());
    mock.set_toggle_debug_result_err("disk full".to_string());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Debug));

    drain_pending(&mut app).await;

    // @step Then within 1 second s-1's scrollback contains a chunk whose text equals "[error] /debug failed: disk full"
    let text = session_scrollback_text(&app, &sid("s-1"));
    assert!(
        text.lines().any(|l| l == "[error] /debug failed: disk full"),
        "expected `[error] /debug failed: disk full` line in s-1 scrollback, got {text:?}",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /debug with no current session is a silent no-op
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debug_with_no_current_session_is_a_silent_no_op() {
    // @step Given an App with NO current session
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    assert!(app.agent_view_store().current_session().is_none());

    // @step When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Debug));

    tokio::time::sleep(Duration::from_millis(100)).await;
    drain_pending(&mut app).await;

    // @step Then backend.toggle_debug is never called
    assert_eq!(
        mock.toggle_debug_calls(),
        0,
        "toggle_debug must NOT be called when there is no current session",
    );

    // @step And no scrollback chunk is appended to any session
    // The no-op path must not silently spawn a session or queue a notice.
    // Assert both: (a) no sessions exist (so no scrollback target was
    // created), and (b) every open session — should none exist, the
    // iteration is empty — has zero scrollback chunks.
    let session_ids: Vec<SessionId> = app
        .agent_view_store()
        .open_sessions()
        .iter()
        .map(|ctx| ctx.id.clone())
        .collect();
    assert_eq!(
        session_ids.len(),
        0,
        "no sessions should exist for the no-op assertion",
    );
    for session_id in &session_ids {
        assert_eq!(
            session_chunk_count(&app, session_id),
            0,
            "no scrollback chunk should be appended to session {session_id:?} for the no-op",
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /debug only affects the focused session — background sessions
// are untouched
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debug_only_affects_focused_session_background_untouched() {
    // @step Given an App with two open sessions s-1 (focused) and s-2 (background)
    let mock = Arc::new(MockBackend::new());
    // @step And the MockBackend's toggle_debug returns Ok("/tmp/debug/s-1.jsonl")
    mock.set_toggle_debug_result_ok("/tmp/debug/s-1.jsonl".to_string());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::SessionCreated(sid("s-2")));
    // s-2 was created second → it is currently focused. Cycle back to s-1.
    app.dispatch(Action::SessionPrev);
    assert_eq!(app.agent_view_store().current_session(), Some(&sid("s-1")));
    let initial_s2_count = session_chunk_count(&app, &sid("s-2"));

    // @step When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Debug));

    // @step Then within 1 second backend.toggle_debug is called exactly once with session_id s-1
    wait_until(
        || mock.toggle_debug_calls() == 1,
        "backend.toggle_debug call count to reach 1",
    )
    .await;
    assert_eq!(mock.toggle_debug_calls(), 1);
    assert_eq!(
        mock.last_toggle_debug().map(|(s, _)| s),
        Some(sid("s-1"))
    );

    drain_pending(&mut app).await;

    // @step And within 1 second s-1's scrollback contains a chunk whose text starts with "[debug] capture toggled"
    let s1_text = session_scrollback_text(&app, &sid("s-1"));
    assert!(
        s1_text
            .lines()
            .any(|l| l.starts_with("[debug] capture toggled")),
        "expected debug notice in s-1 scrollback, got {s1_text:?}",
    );

    // @step And s-2's scrollback does NOT contain any chunk whose text starts with "[debug] capture toggled"
    let s2_text = session_scrollback_text(&app, &sid("s-2"));
    assert!(
        s2_text
            .lines()
            .all(|l| !l.starts_with("[debug] capture toggled")),
        "s-2 scrollback must NOT contain the debug notice, got {s2_text:?}",
    );
    // s-2's existing chunk count is unchanged.
    assert_eq!(session_chunk_count(&app, &sid("s-2")), initial_s2_count);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /debug honours the FSPEC_DEBUG_DIR environment variable
// ─────────────────────────────────────────────────────────────────────────

/// Serialised env-var test — runs in its own subprocess via `serial_test`
/// would be preferred, but since the integration suite does not declare
/// the `serial_test` dependency we use a unique env var name per test
/// instead. Production code reads `FSPEC_DEBUG_DIR` directly so we set
/// it for the duration of the test and clear it at the end.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::await_holding_lock)] // FSPEC_DEBUG_DIR_LOCK serialises env-var-touching tests; std::sync::Mutex is intentional.
async fn debug_honours_fspec_debug_dir_env_var() {
    // Serialise with the FSPEC_DEBUG_DIR-default test so the env var
    // can't race underneath sibling tests.
    let _guard = FSPEC_DEBUG_DIR_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given the environment variable FSPEC_DEBUG_DIR is set to "/custom/path"
    std::env::set_var("FSPEC_DEBUG_DIR", "/custom/path");

    // @step And an App with an open session s-1 wired to a MockBackend whose toggle_debug returns Ok("/custom/path/s-1/session-x.jsonl")
    let mock = Arc::new(MockBackend::new());
    mock.set_toggle_debug_result_ok("/custom/path/s-1/session-x.jsonl".to_string());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Debug));

    // @step Then within 1 second backend.toggle_debug is called exactly once with session_id s-1
    wait_until(
        || mock.toggle_debug_calls() == 1,
        "backend.toggle_debug call count to reach 1",
    )
    .await;
    assert_eq!(mock.toggle_debug_calls(), 1);

    // @step And the debug_dir argument equals "/custom/path"
    let last = mock.last_toggle_debug().expect("toggle_debug captured");
    assert_eq!(last.1, "/custom/path");

    std::env::remove_var("FSPEC_DEBUG_DIR");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: SessionHeader [DEBUG] badge reflects per-session debug_enabled
// ─────────────────────────────────────────────────────────────────────────

/// Render the App into an 80x6 buffer and return every span's (text, fg, modifier)
/// triple that appears on the SessionHeader row (row 0).
fn header_spans(app: &mut App) -> Vec<(String, Option<Color>, Modifier)> {
    let area = Rect::new(0, 0, 80, 6);
    let mut buf = Buffer::empty(area);
    app.render(area, &mut buf);
    // Walk row 0 and collect every (style, content) tuple.
    let mut spans = Vec::new();
    let mut current_text = String::new();
    let mut current_fg: Option<Color> = None;
    let mut current_mod: Modifier = Modifier::empty();
    let mut started = false;
    for x in 0..area.width {
        let cell = buf.cell((area.x + x, area.y)).expect("cell in bounds");
        let symbol = cell.symbol().to_string();
        let fg = cell.style().fg;
        let modifier = cell.style().add_modifier;
        if !started {
            current_text = symbol;
            current_fg = fg;
            current_mod = modifier;
            started = true;
        } else if fg == current_fg && modifier == current_mod {
            current_text.push_str(&symbol);
        } else {
            spans.push((current_text.clone(), current_fg, current_mod));
            current_text = symbol;
            current_fg = fg;
            current_mod = modifier;
        }
    }
    if started {
        spans.push((current_text, current_fg, current_mod));
    }
    spans
}

#[test]
fn session_header_debug_badge_appears_when_debug_enabled_is_true() {
    // @step Given an App with an open session s-1
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    // RPC-097 fix: OpenAgentView(None) now mounts the dialog; use direct mode set when only switching view
    app.navigator_mut().active_view = ViewMode::Agent;

    // @step And the AgentViewStore has debug_enabled_by_session[s-1] set to true
    app.agent_view_store_mut().set_debug_enabled(sid("s-1"), true);

    // @step When the AgentView is rendered for s-1
    let spans = header_spans(&mut app);

    // @step Then the SessionHeader emits a span with text " [DEBUG]" styled red bold
    let debug_span = spans
        .iter()
        .find(|(text, _, _)| text.contains("[DEBUG]"));
    assert!(
        debug_span.is_some(),
        "expected [DEBUG] span in SessionHeader, got spans={spans:?}",
    );
    let (_, fg, modifier) = debug_span.expect("[DEBUG] span present");
    assert_eq!(*fg, Some(Color::Red), "[DEBUG] span should be red");
    assert!(
        modifier.contains(Modifier::BOLD),
        "[DEBUG] span should be bold, got modifier {modifier:?}",
    );
}

#[test]
fn session_header_debug_badge_absent_when_debug_enabled_is_false() {
    // @step Given an App with an open session s-1
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    // RPC-097 fix: OpenAgentView(None) now mounts the dialog; use direct mode set when only switching view
    app.navigator_mut().active_view = ViewMode::Agent;

    // @step And the AgentViewStore has debug_enabled_by_session[s-1] set to false
    app.agent_view_store_mut().set_debug_enabled(sid("s-1"), false);

    // @step When the AgentView is rendered for s-1
    let spans = header_spans(&mut app);

    // @step Then the SessionHeader emits NO span with text " [DEBUG]"
    let debug_span = spans.iter().find(|(text, _, _)| text.contains("[DEBUG]"));
    assert!(
        debug_span.is_none(),
        "expected no [DEBUG] span when debug_enabled=false, got spans={spans:?}",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: DebugStateChange chunk from backend updates the badge state
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn debug_state_change_chunk_updates_badge_state_for_focused_session() {
    // @step Given an App with an open session s-1 whose debug_enabled is initially false
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    // RPC-097 fix: OpenAgentView(None) now mounts the dialog; use direct mode set when only switching view
    app.navigator_mut().active_view = ViewMode::Agent;
    assert_eq!(
        app.agent_view_store().debug_enabled_for(&sid("s-1")),
        None,
    );

    // @step When a StreamChunk::DebugStateChange { enabled: true } is delivered as Action::ChunkReceived(s-1, chunk)
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::debug_state_change(true),
    ));

    // @step Then the AgentViewStore.debug_enabled_for(s-1) returns Some(true)
    assert_eq!(
        app.agent_view_store().debug_enabled_for(&sid("s-1")),
        Some(true),
    );

    // @step And the SessionHeader for s-1 emits a span with text " [DEBUG]" styled red bold
    let spans = header_spans(&mut app);
    let debug_span = spans
        .iter()
        .find(|(text, _, _)| text.contains("[DEBUG]"));
    assert!(
        debug_span.is_some(),
        "expected [DEBUG] span after DebugStateChange{{enabled:true}}, got spans={spans:?}",
    );
    let (_, fg, modifier) = debug_span.expect("[DEBUG] span present");
    assert_eq!(*fg, Some(Color::Red));
    assert!(modifier.contains(Modifier::BOLD));
}
