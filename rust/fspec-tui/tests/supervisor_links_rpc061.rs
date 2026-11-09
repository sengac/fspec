//! RPC-061 — Supervisor / subordinate links App + MockBackend integration.
//!
//! Feature: spec/features/rpc061-supervisor-links.feature
//!
//! Drives:
//!  - `Action::SupervisorsLoaded` → AgentViewStore state mutation.
//!  - `StreamChunk::SupervisorPendingInjection` → pending count bumped.
//!  - `SessionCreated` → spawn_load_supervisors → SupervisorsLoaded fires.
//!
//! Mirrors the layout of isolated_session_dialog_rpc060.rs.

#![allow(clippy::too_many_lines)]

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::views::agent::header_build::format_subordinate_label;
use codelet_fspec_tui::views::agent::SessionFooter;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{CompactionProgress, SessionId, StreamChunk, WorkspaceInfo};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

async fn drain_pending(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
    }
    Ok(())
}

async fn wait_until<F: FnMut() -> bool>(
    mut predicate: F,
    label: &str,
) -> Result<(), codelet_test_helpers::TestError> {
    codelet_test_helpers::deadlock_guard(label, async {
        loop {
            if predicate() {
                return;
            }
            codelet_test_helpers::yield_now().await;
        }
    })
    .await
    .or_fail(&format!("timed out waiting for: {label}"))?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: App::dispatch routes Action::SupervisorsLoaded through try_dispatch_supervisor_links
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn supervisors_loaded_writes_into_agent_view_store() -> TestResult {
    // @step Given an App wired to a MockBackend
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.agent_view_store_mut()
        .append_session(codelet_fspec_tui::SessionContext::new(sid("s-1")));

    // @step When Action::SupervisorsLoaded(SessionId("s-1"), vec![SessionId("sup")]) is dispatched
    app.dispatch(Action::SupervisorsLoaded(sid("s-1"), vec![sid("sup")]));
    drain_pending(&mut app).await?;

    // @step Then store.supervisors_for(&SessionId("s-1")) returns &[SessionId("sup")]
    let supervisors = app.agent_view_store().supervisors_for(&sid("s-1"));
    assert_eq!(supervisors, &[sid("sup")]);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: StreamChunk::SupervisorPendingInjection bumps per-session pending count
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn supervisor_pending_injection_chunk_bumps_pending_count() -> TestResult {
    // @step Given an App with open session s-sub wired to a MockBackend
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.agent_view_store_mut()
        .append_session(codelet_fspec_tui::SessionContext::new(sid("s-sub")));

    // @step And store.supervisor_pending_count_for(&SessionId("s-sub")) == 0
    assert_eq!(
        app.agent_view_store()
            .supervisor_pending_count_for(&sid("s-sub")),
        0
    );

    // @step When Action::ChunkReceived(s-sub, StreamChunk::SupervisorPendingInjection) is dispatched
    let chunk = StreamChunk::supervisor_pending_injection(false, "check the build".to_string());
    app.dispatch(Action::ChunkReceived(sid("s-sub"), chunk));
    drain_pending(&mut app).await?;

    // @step Then store.supervisor_pending_count_for(&SessionId("s-sub")) returns 1
    assert_eq!(
        app.agent_view_store()
            .supervisor_pending_count_for(&sid("s-sub")),
        1
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Two consecutive SupervisorPendingInjection chunks bump count to 2
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_supervisor_pending_injection_chunks_bump_count_to_two() -> TestResult {
    // @step Given an App with open session s-sub wired to a MockBackend
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.agent_view_store_mut()
        .append_session(codelet_fspec_tui::SessionContext::new(sid("s-sub")));

    // @step When two Action::ChunkReceived events carrying SupervisorPendingInjection are dispatched
    let chunk1 = StreamChunk::supervisor_pending_injection(false, "first".to_string());
    let chunk2 = StreamChunk::supervisor_pending_injection(false, "second".to_string());
    app.dispatch(Action::ChunkReceived(sid("s-sub"), chunk1));
    app.dispatch(Action::ChunkReceived(sid("s-sub"), chunk2));
    drain_pending(&mut app).await?;

    // @step Then store.supervisor_pending_count_for(&SessionId("s-sub")) returns 2
    assert_eq!(
        app.agent_view_store()
            .supervisor_pending_count_for(&sid("s-sub")),
        2
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: SessionCreated triggers spawn_load_supervisors
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_created_spawns_get_supervisors_and_fires_supervisors_loaded() -> TestResult {
    // @step Given an App wired to a MockBackend whose get_supervisors(SessionId("s-1")) returns vec![SessionId("sup")]
    let mock = Arc::new(MockBackend::new());
    mock.seed_supervisors_for(sid("s-1"), vec![sid("sup")]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When Action::SessionCreated(SessionId("s-1")) is dispatched
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await?;

    // @step Then within 1 second store.supervisors_for(&SessionId("s-1")) returns [SessionId("sup")]
    wait_until(
        || {
            app.agent_view_store()
                .supervisors_for(&sid("s-1"))
                .iter()
                .any(|s| s == &sid("sup"))
        },
        "supervisors loaded for s-1",
    )
    .await?;
    drain_pending(&mut app).await?;

    let supervisors = app.agent_view_store().supervisors_for(&sid("s-1"));
    assert_eq!(supervisors, &[sid("sup")]);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: A session with no supervisors shows no subordinate badge
// ─────────────────────────────────────────────────────────────────────

#[test]
fn no_supervisors_yields_no_subordinate_badge_label() -> TestResult {
    // @step Given the AgentViewStore has no supervisors recorded for session s-sub
    let supervisors: Vec<SessionId> = Vec::new();

    // @step When format_subordinate_label is called with an empty supervisors slice
    let label = format_subordinate_label(&supervisors);

    // @step Then the helper returns None and the SessionHeader paints no [Subordinate of: ...] badge
    assert!(
        label.is_none(),
        "format_subordinate_label(empty) must return None so the header paints no badge"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Multi-supervisor session renders subordinate badge with +N count
// ─────────────────────────────────────────────────────────────────────

#[test]
fn multi_supervisor_subordinate_label_renders_first8_plus_n() -> TestResult {
    // @step Given a session is recorded with three supervisors (s-sup-aaa, s-sup-bbb, s-sup-ccc)
    let supervisors = vec![sid("s-sup-aaa"), sid("s-sup-bbb"), sid("s-sup-ccc")];

    // @step When format_subordinate_label is called with that supervisors slice
    let label = format_subordinate_label(&supervisors);

    // @step Then the helper returns Some("s-sup-aa+2") (first 8 chars of the first supervisor id, plus +<count of remaining supervisors>)
    assert_eq!(label.as_deref(), Some("s-sup-aa+2"));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Supervisor pending chip suppresses the compaction chip
// ─────────────────────────────────────────────────────────────────────

fn row_text(buf: &Buffer, y: u16) -> String {
    let mut s = String::with_capacity(buf.area.width as usize);
    for x in 0..buf.area.width {
        s.push_str(buf[(x, y)].symbol());
    }
    s
}

#[test]
fn supervisor_pending_chip_suppresses_compaction_chip() -> TestResult {
    // @step Given a SessionFooter is constructed with supervisor_pending_count=1 AND a CompactionProgress in flight
    let progress = CompactionProgress {
        phase: "summarising messages".to_string(),
        current: 5,
        total: 10,
    };
    let workspace = WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: None,
    };

    // @step When the footer is rendered to a buffer
    let mut buf = Buffer::empty(Rect::new(0, 0, 80, 1));
    SessionFooter {
        workspace: Some(&workspace),
        compaction_progress: Some(&progress),
        supervisor_pending_count: 1,
        continue_indicator: None,
    }
    .render(buf.area, &mut buf);
    let row = row_text(&buf, 0);

    // @step Then the left-aligned slot paints [1 pending from supervisor] in yellow
    assert!(
        row.contains("[1 pending from supervisor]"),
        "expected `[1 pending from supervisor]` substring in row, got {row:?}"
    );

    // @step And no [compacting: ...] chip is painted (the supervisor signal wins for that frame)
    assert!(
        !row.contains("[compacting:"),
        "expected NO `[compacting:` substring when supervisor chip is active, got {row:?}"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: AttachToSession triggers spawn_load_supervisors and SupervisorsLoaded
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn attach_to_session_spawns_get_supervisors_and_fires_supervisors_loaded() -> TestResult {
    // @step Given an App wired to a MockBackend whose get_supervisors(SessionId("s-sub")) returns vec![SessionId("sup")]
    let mock = Arc::new(MockBackend::new());
    mock.seed_supervisors_for(sid("s-sub"), vec![sid("sup")]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When Action::AttachToSession(SessionId("s-sub")) is dispatched
    app.dispatch(Action::AttachToSession(sid("s-sub")));
    drain_pending(&mut app).await?;

    // @step Then within 1 second store.supervisors_for(&SessionId("s-sub")) returns [SessionId("sup")]
    wait_until(
        || {
            app.agent_view_store()
                .supervisors_for(&sid("s-sub"))
                .iter()
                .any(|s| s == &sid("sup"))
        },
        "supervisors loaded for s-sub via AttachToSession",
    )
    .await?;
    drain_pending(&mut app).await?;

    let supervisors = app.agent_view_store().supervisors_for(&sid("s-sub"));
    assert_eq!(supervisors, &[sid("sup")]);
    Ok(())
}
