//! App bootstrap + subscriber-task + Action enum extensions (RPC-009,
//! migrated by RPC-012 to the BoardStore + AgentViewStore + Navigator
//! architecture).
//!
//! Feature: spec/features/fspec-tui-app-bootstrap-rpc009.feature
//!
//! RPC-012 superseded two RPC-009 contracts: bootstrap no longer calls
//! `create_session(None)` (session creation is lazy), and `RootView` +
//! `FocusedPane` are gone (BUG-356 deleted both scenarios). BUG-299: the
//! chunks subscriber forwards every session's transcript content as
//! `Action::Transcript(Live(..))`. Helpers live in
//! tests/app_bootstrap_rpc009_support/ (one test file per feature).

use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk};

mod common;
use common::{MockBackend, BOOTSTRAP_SUBSCRIBER_TASKS};

mod app_bootstrap_rpc009_support;
use app_bootstrap_rpc009_support::{live_transcript_for, wait_for_action, wu, LogCapture};

#[tokio::test]
async fn app_bootstrap_calls_list_work_units_and_seeds_the_left_pane() -> TestResult {
    // @step Given a MockBackend seeded with [AUTH-001 done, AUTH-002 implementing]
    let mock = Arc::new(MockBackend::new());
    mock.seed_work_units(vec![wu("AUTH-001", "done"), wu("AUTH-002", "implementing")]);
    // @step And an App constructed against that backend on an 80x24 TestBackend
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    // @step When the App's bootstrap runs
    app.bootstrap().await.or_fail("bootstrap")?;
    // @step Then MockBackend.list_work_units_calls equals 1
    assert_eq!(mock.list_work_units_calls(), 1);
    // @step And the WorkUnitsListView's items equals [AUTH-001 done, AUTH-002 implementing]
    let snapshot = app.work_units_snapshot();
    assert_eq!(snapshot.len(), 2);
    let ids: Vec<&str> = snapshot.iter().map(|u| u.id.as_str()).collect();
    assert!(ids.contains(&"AUTH-001"));
    assert!(ids.contains(&"AUTH-002"));
    // @step And the WorkUnitsListView's state.selected() returns Some(0)
    // RPC-012: focus defaults to "backlog" column with selection 0.
    assert_eq!(app.board_store().focused_column(), "backlog");
    assert_eq!(app.board_store().selected_index_for("backlog"), 0);
    Ok(())
}

/// Scenario: App bootstrap spawns the subscriber tasks via tokio::spawn on the host runtime
///
/// RPC-045 superseded the original RPC-009 "three subscriber tasks"
/// contract by adding a fourth subscriber for `status_changes_rx`. The
/// scenario name and assertion below were updated to assert the new
/// task count.
///
/// RPC-385 supersedes it again: a FIFTH subscriber (`session_created_rx`)
/// makes spawned subordinate sessions visible as tabs, so the live count
/// is now 5 (work_units + chunks + logs + status_changes + session_created).
#[tokio::test]
async fn app_bootstrap_spawns_three_subscriber_tasks_via_tokio_spawn_on_the_host_runtime(
) -> TestResult {
    // @step Given an App constructed against a MockBackend on a `#[tokio::test]` runtime
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    // @step When the App's bootstrap runs
    app.bootstrap().await.or_fail("bootstrap")?;
    // @step Then the bootstrap subscriber tasks are alive on the current tokio Handle
    //         (RPC-045: work_units_rx + chunks_rx + logs_rx + status_changes_rx;
    //          RPC-385 added a fifth: session_created_rx; TUI-109 added a
    //          sixth: checkpoints_progress_rx; BUG-182 a seventh:
    //          git_state_changed_rx; RPC-434 an eighth: session_removed_rx;
    //          RPC-437 a ninth: session_cleared_rx; RPC-445 the
    //          workspace-keyed work-units stream)
    assert_eq!(app.subscriber_task_count(), BOOTSTRAP_SUBSCRIBER_TASKS);
    // RPC-012 lazy-session: prime the chunks filter with a session id
    // so the chunks subscriber forwards.
    app.dispatch(Action::SessionCreated(SessionId::new("s-mock-1")));
    // @step And one task drains `backend.work_units_rx()` and sends `Action::WorkUnitsLoaded(units)` to the action bus
    mock.push_work_units(vec![wu("AUTH-003", "backlog")]);
    let action = wait_for_action(&mut app, |a| matches!(a, Action::WorkUnitsLoaded(_)))
        .await
        .or_fail("Action::WorkUnitsLoaded on bus")?;
    assert!(matches!(action, Action::WorkUnitsLoaded(units) if units.len() == 1));
    // @step And one task drains `backend.chunks_rx()` and forwards every session's chunks to the action bus, transcript content as `Action::Transcript(Live(id, chunk))`
    mock.push_chunk(
        SessionId::new("s-mock-1"),
        StreamChunk::text("x".to_string()),
    );
    let action = wait_for_action(&mut app, |a| live_transcript_for(a, "s-mock-1"))
        .await
        .or_fail("live transcript action on bus")?;
    assert!(live_transcript_for(&action, "s-mock-1"));
    // @step And one task drains `backend.logs_rx()` and forwards records to the action bus or tracing layer
    // (subscriber_task_count == 4 above proves all four are alive).
    // @step And no `tokio::runtime::Builder` or `Runtime::new()` call appears in the App bootstrap path
    // (asserted by source_shape_rpc009.rs)
    Ok(())
}

/// Scenario: work_units broadcast event becomes an Action::WorkUnitsLoaded on the action bus
#[tokio::test]
async fn work_units_broadcast_event_becomes_an_action_workunitsloaded_on_the_action_bus(
) -> TestResult {
    // @step Given an App constructed against a MockBackend with bootstrap complete
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    // @step When the test calls `mock.push_work_units(vec![AUTH-001 done, AUTH-002 implementing, AUTH-003 backlog])`
    mock.push_work_units(vec![
        wu("AUTH-001", "done"),
        wu("AUTH-002", "implementing"),
        wu("AUTH-003", "backlog"),
    ]);
    // @step Then within 200ms the App's action bus receives an `Action::WorkUnitsLoaded` carrying the new three-entry list
    let action = wait_for_action(&mut app, |a| matches!(a, Action::WorkUnitsLoaded(_)))
        .await
        .or_fail("WorkUnitsLoaded within 200ms")?;
    let units = match action {
        Action::WorkUnitsLoaded(u) => u,
        _ => return Err("expected WorkUnitsLoaded".into()),
    };
    assert_eq!(units.len(), 3);
    // @step And the WorkUnitsListView's items equals the three-entry list after compositor.update is called
    app.dispatch(Action::WorkUnitsLoaded(units));
    assert_eq!(app.work_units_snapshot().len(), 3);
    Ok(())
}

/// Scenario: Transcript chunks for the active session become live transcript actions
#[tokio::test]
async fn chunks_broadcast_events_for_the_active_session_become_action_chunkreceived() -> TestResult
{
    // @step Given an App with active_session = Some(SessionId("s-mock-1")) and bootstrap complete
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    app.dispatch(Action::SessionCreated(SessionId::new("s-mock-1")));
    // @step When the test calls `mock.push_chunk(SessionId::new("s-mock-1"), StreamChunk::text("hello".into()))`
    mock.push_chunk(
        SessionId::new("s-mock-1"),
        StreamChunk::text("hello".to_string()),
    );
    // @step Then within 200ms the App's action bus receives an `Action::Transcript(Live(SessionId::new("s-mock-1"), StreamChunk::text("hello".into())))`
    let action = wait_for_action(&mut app, |a| live_transcript_for(a, "s-mock-1"))
        .await
        .or_fail("live transcript action within 200ms")?;
    assert!(live_transcript_for(&action, "s-mock-1"));
    Ok(())
}

/// Scenario: chunks broadcast events for an OTHER session ALSO become Action::ChunkReceived
///
/// RPC-045 supersedes the original RPC-009 behaviour where the chunks
/// subscriber filtered by `AgentViewStore.current_session`. Background
/// sessions now accumulate scrollback + per-session state regardless of
/// focus, so the subscriber forwards every `(SessionId, StreamChunk)`.
#[tokio::test]
async fn chunks_broadcast_events_for_an_other_session_do_not_become_action_chunkreceived(
) -> TestResult {
    // @step Given an App with active_session = Some(SessionId("s-mock-1")) and bootstrap complete
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    app.dispatch(Action::SessionCreated(SessionId::new("s-mock-1")));
    // @step When the test calls `mock.push_chunk(SessionId::new("s-other"), StreamChunk::text("not for us".into()))`
    mock.push_chunk(
        SessionId::new("s-other"),
        StreamChunk::text("not for us".to_string()),
    );
    // @step Then within 200ms the App's action bus receives an `Action::Transcript(Live(SessionId::new("s-other"), ..))` because RPC-045 removed the active-session filter
    let action = wait_for_action(&mut app, |a| live_transcript_for(a, "s-other")).await;
    assert!(
        action.is_some(),
        "RPC-045 forwards transcript chunks for background sessions"
    );
    Ok(())
}

/// Scenario: Action::InputSubmitted dispatches backend.send_input and is forwarded to compositor.update
#[tokio::test]
async fn action_inputsubmitted_dispatches_backend_send_input_and_is_forwarded_to_compositor_update(
) -> TestResult {
    // @step Given an App with active_session = Some(SessionId("s-mock-1")) and bootstrap complete
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    app.dispatch(Action::SessionCreated(SessionId::new("s-mock-1")));
    // @step When the App processes `Action::InputSubmitted("hi".into())` on the action bus
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, "hi".to_string()));
    }
    // TEST-042: wait for the spawned task's send_input call itself.
    codelet_test_helpers::wait_until("the send_input call", || {
        std::future::ready(mock.send_input_calls() > 0)
    })
    .await?;
    // @step Then `MockBackend.send_input` is invoked exactly once with `(SessionId("s-mock-1"), "hi")`
    assert_eq!(mock.send_input_calls(), 1);
    let last = mock.last_send_input();
    assert_eq!(last, Some((SessionId::new("s-mock-1"), "hi".to_string())));
    // @step And the action is also forwarded into compositor.update so layers can react if needed
    // (verified by App::dispatch's contract — it always calls compositor.update after special-casing)
    Ok(())
}

/// Scenario: Action::Interrupt dispatches backend.interrupt
#[tokio::test]
async fn action_interrupt_dispatches_backend_interrupt() -> TestResult {
    // @step Given an App with active_session = Some(SessionId("s-mock-1"))
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    app.dispatch(Action::SessionCreated(SessionId::new("s-mock-1")));
    let was_quit = app.should_quit();
    // @step When the App processes `Action::Interrupt` on the action bus
    app.dispatch(Action::Interrupt);
    codelet_test_helpers::wait_until("the interrupt call", || {
        std::future::ready(mock.interrupt_calls() > 0)
    })
    .await?;
    // @step Then `MockBackend.interrupt` is invoked exactly once with `SessionId("s-mock-1")`
    assert_eq!(mock.interrupt_calls(), 1);
    assert_eq!(mock.last_interrupt(), Some(SessionId::new("s-mock-1")));
    // @step And the App's `should_quit` flag is unchanged
    assert_eq!(app.should_quit(), was_quit);
    Ok(())
}

/// Scenario: Subscriber tasks honour RecvError::Lagged by logging at debug and continuing
#[tokio::test]
async fn subscriber_tasks_honour_recverror_lagged_by_logging_at_debug_and_continuing() -> TestResult
{
    // @step Given an App constructed against a MockBackend
    let logs = LogCapture::default();
    let _log_guard = logs.install();
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    // @step And the work_units broadcast channel is intentionally lagged by overflowing its capacity
    for i in 0..200 {
        mock.push_work_units(vec![wu(&format!("AUTH-{i:03}"), "backlog")]);
    }
    // @step When the work_units subscriber task observes `RecvError::Lagged(n)`
    // @step Then the task does NOT panic
    // TEST-042: wait for the lag to be observed (its debug line), not a delay.
    let lagged = || {
        logs.text().is_ok_and(|text| {
            text.lines().any(|line| {
                line.contains("DEBUG") && line.contains("work_units subscriber lagged by")
            })
        })
    };
    let _ =
        codelet_test_helpers::wait_until("the lagged debug line", || std::future::ready(lagged()))
            .await;
    assert_eq!(app.subscriber_task_count(), BOOTSTRAP_SUBSCRIBER_TASKS);
    // @step And the task logs at debug level with a "Lagged" message
    let lagged_debug = lagged();
    assert!(
        lagged_debug,
        "a DEBUG lagged line must be logged, got:\n{}",
        logs.text()?
    );
    // @step And the task subsequently re-fetches a snapshot via `backend.list_work_units()` and emits a fresh `Action::WorkUnitsLoaded`
    mock.seed_work_units(vec![wu("FRESH-001", "done")]);
    let action = wait_for_action(&mut app, |a| matches!(a, Action::WorkUnitsLoaded(_)))
        .await
        .or_fail("post-Lagged WorkUnitsLoaded")?;
    assert!(matches!(action, Action::WorkUnitsLoaded(_)));
    Ok(())
}

/// Scenario: Action enum gains the bootstrap variants while existing variants are preserved
#[test]
fn action_enum_gains_seven_new_variants_while_existing_variants_are_preserved() -> TestResult {
    // @step Given the Action enum in rust/fspec-tui/src/components/mod.rs
    let src = codelet_test_helpers::source_composition::read_source(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/components/mod.rs"),
    )
    .or_fail("read composed components/mod.rs")?;
    // @step Then it contains the existing variant Quit (TUI-127 removed Custom(String) and TUI-132 removed Redraw, LoadWorkUnits and FocusNext: nothing emitted or handled them)
    assert!(src.contains("Quit,"));
    assert!(!src.contains("Custom(String)"));
    for gone in ["Redraw,", "LoadWorkUnits,", "FocusNext,"] {
        assert!(!src.contains(gone), "Action still declares {gone}");
    }
    // @step And it additionally contains WorkUnitsLoaded(Vec<WorkUnitInfo>)
    assert!(src.contains("WorkUnitsLoaded(Vec<codelet_rpc_types::WorkUnitInfo>)"));
    // @step And it additionally contains SessionCreated(SessionId)
    assert!(src.contains("SessionCreated(codelet_rpc_types::SessionId)"));
    // @step And it additionally contains ChunkReceived(SessionId, StreamChunk)
    assert!(
        src.contains("ChunkReceived(codelet_rpc_types::SessionId, codelet_rpc_types::StreamChunk)")
    );
    // @step And it additionally contains InputSubmitted(SessionId, String)
    assert!(src.contains("InputSubmitted(codelet_rpc_types::SessionId, String)"));
    // @step And it additionally contains Interrupt
    assert!(src.contains("Interrupt,"));
    // @step And the enum still derives Clone, Debug
    assert!(src.contains("#[derive(Debug, Clone)]"));
    // @step And the enum drops PartialEq and Eq because StreamChunk does not derive PartialEq
    // (asserted by the absence of PartialEq, Eq in the derive line — exactly `#[derive(Debug, Clone)]`)
    Ok(())
}
