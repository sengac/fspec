//! RPC-052 — Pending-input draft persistence on session switch.
//!
//! Feature: spec/features/pending-input-draft-persistence.feature
//!
//! Drives the App::dispatch state machine through the new
//! `Action::PendingInputChanged` debounce + `Action::SeedPendingInput`
//! hydration flow against the shared `MockBackend` (scripted
//! `pending_input` store + per-call counters from
//! `tests/common/mod.rs`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: mods,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

/// Drain the App's action bus AND every pending tokio task spawned
/// inside `App::dispatch` so subsequent assertions observe a settled
/// state. Mirrors the helper in keyboard_cascade_rpc051.rs.
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

/// Spin until `predicate()` returns true or 1 second elapses. Used to
/// observe debounced background tasks without sprinkling magic sleeps.
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

/// Build an App with the supplied MockBackend, in ViewMode::Agent,
/// with a single open session at `session_id`. Drains the initial
/// SessionCreated wiring before returning.
async fn agent_app_with_session(mock: Arc<MockBackend>, session_id: SessionId) -> App {
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.navigator_mut().active_view = ViewMode::Agent;
    app.dispatch(Action::SessionCreated(session_id));
    drain_pending(&mut app).await;
    app
}

/// Build an App in ViewMode::Agent with NO open session yet.
fn agent_app_no_session(mock: Arc<MockBackend>) -> App {
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.navigator_mut().active_view = ViewMode::Agent;
    app
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 1: Typing fires Action::PendingInputChanged when the buffer
// text changes
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typing_fires_pending_input_changed_when_buffer_changes() {
    // @step Given an App with a MockBackend
    let mock = Arc::new(MockBackend::new());
    // @step And session s-1 is the current session
    let mut app = agent_app_with_session(mock.clone(), sid("s-1")).await;
    // @step And the live MultiLineInput is empty
    assert!(app.navigator().agent.input.is_empty());

    // @step When the user types the character "h"
    let _ = app.handle_event(&key(KeyCode::Char('h'), KeyModifiers::NONE));

    // @step Then Action::PendingInputChanged("h") is dispatched
    let mut saw_pending_input_changed = false;
    while let Some(action) = app.try_recv_action() {
        if matches!(&action, Action::PendingInputChanged(text) if text == "h") {
            saw_pending_input_changed = true;
        }
        app.dispatch(action);
    }
    assert!(
        saw_pending_input_changed,
        "expected Action::PendingInputChanged(\"h\") on the action bus"
    );
    // @step And the AgentView's MultiLineInput value equals "h"
    assert_eq!(app.navigator().agent.input.value(), "h");
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 2: Cursor-only key events do not fire Action::PendingInputChanged
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cursor_only_key_events_do_not_fire_pending_input_changed() {
    // @step Given an App with a MockBackend
    let mock = Arc::new(MockBackend::new());
    // @step And session s-1 is the current session
    let mut app = agent_app_with_session(mock.clone(), sid("s-1")).await;
    // @step And the live MultiLineInput contains "hello"
    app.navigator_mut().agent.input.set_value("hello");
    // Drain the priming actions before exercising the cursor move.
    drain_pending(&mut app).await;
    while app.try_recv_action().is_some() {}
    // @step And the cursor is at the end of the buffer
    // (set_value lands the cursor at end of the last line per
    //  multiline_input::set_value contract.)

    // @step When the user presses Left arrow with no modifiers
    let _ = app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));

    // @step Then no Action::PendingInputChanged is dispatched
    while let Some(action) = app.try_recv_action() {
        assert!(
            !matches!(&action, Action::PendingInputChanged(_)),
            "cursor-only Left arrow must NOT emit PendingInputChanged"
        );
        app.dispatch(action);
    }
    // @step And the AgentView's MultiLineInput value still equals "hello"
    assert_eq!(app.navigator().agent.input.value(), "hello");
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 3: Single PendingInputChanged triggers exactly one
// debounced backend save
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn single_pending_input_changed_triggers_one_debounced_save() {
    // @step Given an App with a MockBackend
    let mock = Arc::new(MockBackend::new());
    // @step And session s-1 is the current session
    let mut app = agent_app_with_session(mock.clone(), sid("s-1")).await;
    let calls_before = mock.set_pending_input_calls();

    // @step When Action::PendingInputChanged("hello") is dispatched
    app.dispatch(Action::PendingInputChanged("hello".to_string()));
    // @step And the debounce timer of 300ms elapses
    // (drain pending then wait for the spawned task to fire)

    // @step Then within 1 second backend.set_pending_input is called exactly once with (s-1, Some("hello"))
    wait_until(
        || mock.set_pending_input_calls() == calls_before + 1,
        "set_pending_input to be called once",
    )
    .await;
    drain_pending(&mut app).await;
    let last = mock.last_set_pending_input();
    assert_eq!(
        last,
        Some((sid("s-1"), Some("hello".to_string()))),
        "expected last set_pending_input to be (s-1, Some(\"hello\"))"
    );
    assert_eq!(mock.set_pending_input_calls(), calls_before + 1);
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 4: Rapid PendingInputChanged events coalesce into a single
// backend save with the final value
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rapid_pending_input_changed_coalesces_into_one_save() {
    // @step Given an App with a MockBackend
    let mock = Arc::new(MockBackend::new());
    // @step And session s-1 is the current session
    let mut app = agent_app_with_session(mock.clone(), sid("s-1")).await;
    let calls_before = mock.set_pending_input_calls();

    // @step When Action::PendingInputChanged("h") is dispatched
    app.dispatch(Action::PendingInputChanged("h".to_string()));
    // @step And Action::PendingInputChanged("hi") is dispatched within 50ms
    tokio::time::sleep(Duration::from_millis(50)).await;
    app.dispatch(Action::PendingInputChanged("hi".to_string()));

    // @step And the debounce timer of 300ms elapses since the last edit
    wait_until(
        || mock.set_pending_input_calls() > calls_before,
        "set_pending_input to fire at least once",
    )
    .await;
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.set_pending_input is called exactly once with (s-1, Some("hi"))
    assert_eq!(
        mock.set_pending_input_calls(),
        calls_before + 1,
        "rapid edits within debounce window must coalesce into one save"
    );
    let writes = mock.pending_input_writes();
    // @step And backend.set_pending_input is NOT called with (s-1, Some("h"))
    assert!(
        !writes
            .iter()
            .any(|(s, t)| s == &sid("s-1") && t.as_deref() == Some("h")),
        "stale 'h' save must NOT reach the backend (got writes: {writes:?})"
    );
    assert_eq!(
        mock.last_set_pending_input(),
        Some((sid("s-1"), Some("hi".to_string()))),
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 5: PendingInputChanged with no current session is a silent
// no-op
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_input_changed_with_no_session_is_silent_noop() {
    // @step Given an App with a MockBackend
    let mock = Arc::new(MockBackend::new());
    // @step And there is NO current session
    let mut app = agent_app_no_session(mock.clone());
    assert!(app.current_session().is_none());
    let calls_before = mock.set_pending_input_calls();

    // @step When Action::PendingInputChanged("orphan") is dispatched
    app.dispatch(Action::PendingInputChanged("orphan".to_string()));
    // @step And the debounce timer of 300ms elapses
    tokio::time::sleep(Duration::from_millis(400)).await;
    drain_pending(&mut app).await;

    // @step Then backend.set_pending_input is NEVER called
    assert_eq!(
        mock.set_pending_input_calls(),
        calls_before,
        "no current session must mean set_pending_input is never called"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 6: PendingInputChanged updates SessionContext.input_draft
// mirror immediately
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_input_changed_updates_session_context_input_draft_mirror() {
    // @step Given an App with a MockBackend
    let mock = Arc::new(MockBackend::new());
    // @step And session s-1 is the current session
    let mut app = agent_app_with_session(mock.clone(), sid("s-1")).await;

    // @step When Action::PendingInputChanged("draft-mirror") is dispatched
    app.dispatch(Action::PendingInputChanged("draft-mirror".to_string()));

    // @step Then AgentViewStore.session_context_for(s-1).input_draft equals "draft-mirror" synchronously
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("s-1 SessionContext exists");
    assert_eq!(ctx.input_draft, "draft-mirror");
    drain_pending(&mut app).await;
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 7: SessionCreated hydrates the live input from
// backend.get_pending_input
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_created_hydrates_live_input_from_backend() {
    // @step Given an App with a MockBackend
    let mock = Arc::new(MockBackend::new());
    // @step And the MockBackend's get_pending_input is scripted to return Some("restored draft") for s-1
    mock.script_pending_input(sid("s-1"), Some("restored draft".to_string()));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.navigator_mut().active_view = ViewMode::Agent;

    // @step When Action::SessionCreated(s-1) is dispatched
    app.dispatch(Action::SessionCreated(sid("s-1")));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;
    // Hydration is async — wait until the seed lands.
    wait_until(
        || app.navigator().agent.input.value() == "restored draft",
        "MultiLineInput to be hydrated to \"restored draft\"",
    )
    .await;

    // @step Then backend.get_pending_input is called exactly once with s-1
    assert!(
        mock.get_pending_input_calls() >= 1,
        "get_pending_input must have been called at least once for s-1"
    );
    // @step And the AgentView's MultiLineInput value equals "restored draft"
    assert_eq!(app.navigator().agent.input.value(), "restored draft");
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 8: SessionCreated with backend returning None leaves the
// input empty
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_created_with_none_leaves_input_empty() {
    // @step Given an App with a MockBackend
    let mock = Arc::new(MockBackend::new());
    // @step And the MockBackend's get_pending_input is scripted to return None for s-1
    mock.script_pending_input(sid("s-1"), None);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.navigator_mut().active_view = ViewMode::Agent;

    // @step When Action::SessionCreated(s-1) is dispatched
    app.dispatch(Action::SessionCreated(sid("s-1")));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;
    // Allow any hydrate task to fire.
    tokio::time::sleep(Duration::from_millis(50)).await;
    drain_pending(&mut app).await;

    // @step Then backend.get_pending_input is called exactly once with s-1
    assert!(mock.get_pending_input_calls() >= 1);
    // @step And the AgentView's MultiLineInput is empty
    assert!(
        app.navigator().agent.input.is_empty(),
        "MultiLineInput must remain empty when backend pending_input is None"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 9: AttachToSession hydrates the live input from
// backend.get_pending_input
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn attach_to_session_hydrates_live_input_from_backend() {
    // @step Given an App with a MockBackend
    let mock = Arc::new(MockBackend::new());
    // @step And session s-1 is open (already created)
    let mut app = agent_app_with_session(mock.clone(), sid("s-1")).await;
    // @step And the MockBackend's get_pending_input is scripted to return Some("attach-draft") for s-2
    mock.script_pending_input(sid("s-2"), Some("attach-draft".to_string()));
    let get_calls_before = mock.get_pending_input_calls();

    // @step When Action::AttachToSession(s-2) is dispatched
    app.dispatch(Action::AttachToSession(sid("s-2")));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;
    wait_until(
        || app.navigator().agent.input.value() == "attach-draft",
        "MultiLineInput to be hydrated to \"attach-draft\" on attach",
    )
    .await;

    // @step Then backend.get_pending_input is called at least once with s-2
    assert!(
        mock.get_pending_input_calls() > get_calls_before,
        "AttachToSession must spawn at least one get_pending_input(s-2)"
    );
    // @step And the AgentView's MultiLineInput value equals "attach-draft"
    assert_eq!(app.navigator().agent.input.value(), "attach-draft");
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 10: SeedPendingInput is dropped when the activated session
// is no longer focused
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seed_pending_input_dropped_when_session_no_longer_focused() {
    // @step Given an App with a MockBackend
    let mock = Arc::new(MockBackend::new());
    // @step And session s-1 is the current session
    let mut app = agent_app_with_session(mock.clone(), sid("s-1")).await;
    // @step And the AgentView's MultiLineInput contains "typing-now"
    app.navigator_mut().agent.input.set_value("typing-now");
    // Mirror the draft into the SessionContext to mimic the live
    // state — the dispatcher path normally keeps these in sync.
    if let Some(ctx) = app
        .agent_view_store_mut()
        .session_context_mut_for(&sid("s-1"))
    {
        ctx.input_draft = "typing-now".to_string();
    }
    drain_pending(&mut app).await;
    while app.try_recv_action().is_some() {}

    // @step When Action::SeedPendingInput { session_id: s-2, text: "stale" } is dispatched
    app.dispatch(Action::SeedPendingInput {
        session_id: sid("s-2"),
        text: "stale".to_string(),
    });
    drain_pending(&mut app).await;

    // @step Then the AgentView's MultiLineInput value still equals "typing-now"
    assert_eq!(app.navigator().agent.input.value(), "typing-now");
    // @step And AgentViewStore.session_context_for(s-1).input_draft equals "typing-now"
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("s-1 still open");
    assert_eq!(ctx.input_draft, "typing-now");
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 11: SeedPendingInput updates SessionContext.input_draft for
// the target session even when not focused
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seed_pending_input_updates_target_session_draft_even_when_unfocused() {
    // @step Given an App with a MockBackend
    let mock = Arc::new(MockBackend::new());
    // @step And open sessions are [s-1, s-2] and s-1 is focused
    let mut app = agent_app_with_session(mock.clone(), sid("s-1")).await;
    // Open a second session by attaching to it.
    app.dispatch(Action::AttachToSession(sid("s-2")));
    drain_pending(&mut app).await;
    // Re-focus s-1 so s-2 is the "background" target.
    app.dispatch(Action::AttachToSession(sid("s-1")));
    drain_pending(&mut app).await;
    assert_eq!(app.current_session(), Some(sid("s-1")));
    // Snapshot s-1's live input so we can assert it stays unchanged.
    app.navigator_mut().agent.input.set_value("s1-buffer");

    // @step When Action::SeedPendingInput { session_id: s-2, text: "background-draft" } is dispatched
    app.dispatch(Action::SeedPendingInput {
        session_id: sid("s-2"),
        text: "background-draft".to_string(),
    });
    drain_pending(&mut app).await;

    // @step Then AgentViewStore.session_context_for(s-2).input_draft equals "background-draft"
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-2"))
        .expect("s-2 must be open");
    assert_eq!(ctx.input_draft, "background-draft");
    // @step And the AgentView's MultiLineInput value still reflects s-1's buffer (unchanged)
    assert_eq!(app.navigator().agent.input.value(), "s1-buffer");
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 12: InputSubmitted clears the durable draft via
// backend.set_pending_input(None)
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn input_submitted_clears_durable_draft_via_set_pending_input_none() {
    // @step Given an App with a MockBackend
    let mock = Arc::new(MockBackend::new());
    // @step And session s-1 is the current session
    let mut app = agent_app_with_session(mock.clone(), sid("s-1")).await;
    let calls_before = mock.set_pending_input_calls();

    // @step When the user submits the input "hello world" via Enter
    app.dispatch(Action::InputSubmitted("hello world".to_string()));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.set_pending_input is called with (s-1, None)
    wait_until(
        || {
            mock.pending_input_writes()
                .iter()
                .any(|(s, t)| s == &sid("s-1") && t.is_none())
        },
        "set_pending_input(s-1, None) to be observed",
    )
    .await;
    drain_pending(&mut app).await;
    assert!(
        mock.set_pending_input_calls() > calls_before,
        "InputSubmitted must trigger at least one set_pending_input call"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 13: Backend.set_pending_input error does not panic and
// emits no scrollback notice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_pending_input_error_does_not_panic_or_emit_scrollback() {
    // @step Given an App with a MockBackend
    let mock = Arc::new(MockBackend::new());
    // @step And session s-1 is the current session
    let mut app = agent_app_with_session(mock.clone(), sid("s-1")).await;
    // @step And the MockBackend's set_pending_input is scripted to return Err("network blip")
    mock.set_set_pending_input_error("network blip".to_string());
    let calls_before = mock.set_pending_input_calls();

    // @step When Action::PendingInputChanged("draft") is dispatched
    app.dispatch(Action::PendingInputChanged("draft".to_string()));
    // @step And the debounce timer of 300ms elapses
    // @step And all pending tasks have drained
    wait_until(
        || mock.set_pending_input_calls() > calls_before,
        "set_pending_input to be attempted",
    )
    .await;
    drain_pending(&mut app).await;

    // @step Then the App must not panic
    // (Reaching this line is the proof — no panic propagated.)
    // @step And the session s-1 scrollback contains no chunks mentioning "set_pending_input"
    // @step And the session s-1 scrollback contains no chunks mentioning "network blip"
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("s-1 SessionContext exists");
    let chunk_count = ctx.scrollback.chunk_count();
    assert_eq!(
        chunk_count, 0,
        "no scrollback notices must be emitted on set_pending_input error \
         (chunk_count={chunk_count})"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 14: Backend.get_pending_input error during hydration
// leaves the input empty without panicking
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_pending_input_error_during_hydration_leaves_input_empty() {
    // @step Given an App with a MockBackend
    let mock = Arc::new(MockBackend::new());
    // @step And the MockBackend's get_pending_input is scripted to return Err("decode failed") for s-1
    mock.set_get_pending_input_error("decode failed".to_string());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.navigator_mut().active_view = ViewMode::Agent;

    // @step When Action::SessionCreated(s-1) is dispatched
    app.dispatch(Action::SessionCreated(sid("s-1")));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    drain_pending(&mut app).await;

    // @step Then the App must not panic (reaching here proves it)
    // @step And the AgentView's MultiLineInput is empty
    assert!(
        app.navigator().agent.input.is_empty(),
        "get_pending_input error must leave the live input empty"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario 15: Shift+Left/Right cycling uses SessionContext.input_draft
// (no backend round-trip per cycle)
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shift_arrow_cycling_uses_session_context_input_draft() {
    // @step Given an App with a MockBackend
    let mock = Arc::new(MockBackend::new());
    // @step And open sessions are [s-1, s-2] and s-1 is focused
    let mut app = agent_app_with_session(mock.clone(), sid("s-1")).await;
    app.dispatch(Action::AttachToSession(sid("s-2")));
    drain_pending(&mut app).await;
    app.dispatch(Action::AttachToSession(sid("s-1")));
    drain_pending(&mut app).await;
    assert_eq!(app.current_session(), Some(sid("s-1")));
    // @step And the AgentView's MultiLineInput contains "draft-on-s1"
    app.navigator_mut().agent.input.set_value("draft-on-s1");
    while app.try_recv_action().is_some() {}
    let get_calls_before = mock.get_pending_input_calls();

    // @step When the user presses Shift+Right
    app.dispatch(Action::SessionNext);
    drain_pending(&mut app).await;

    // @step Then AgentViewStore.session_context_for(s-1).input_draft equals "draft-on-s1"
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("s-1 still open");
    assert_eq!(ctx.input_draft, "draft-on-s1");
    // @step And the focused session is s-2
    assert_eq!(app.current_session(), Some(sid("s-2")));
    // @step And the AgentView's MultiLineInput reflects s-2's SessionContext.input_draft
    let s2_draft = app
        .agent_view_store()
        .session_context_for(&sid("s-2"))
        .expect("s-2 open")
        .input_draft
        .clone();
    assert_eq!(app.navigator().agent.input.value(), s2_draft);

    // @step When the user presses Shift+Left
    app.dispatch(Action::SessionPrev);
    drain_pending(&mut app).await;

    // @step Then the focused session is s-1
    assert_eq!(app.current_session(), Some(sid("s-1")));
    // @step And the AgentView's MultiLineInput value equals "draft-on-s1"
    assert_eq!(app.navigator().agent.input.value(), "draft-on-s1");
    // Sanity: cycling did NOT round-trip through the backend.
    assert_eq!(
        mock.get_pending_input_calls(),
        get_calls_before,
        "Shift+Left/Right must NOT call backend.get_pending_input — the \
         SessionContext.input_draft fast path is preserved"
    );
}
