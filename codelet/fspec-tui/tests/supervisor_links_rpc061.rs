//! RPC-061 — Supervisor / subordinate links App + MockBackend integration.
//!
//! Feature: spec/features/rpc061-supervisor-links.feature
//!
//! Drives:
//!  - `Action::SupervisorsLoaded` → AgentViewStore state mutation.
//!  - `Action::SendToSubordinate` → backend.receive_incoming_message
//!    spawn + EmitSessionNotice on Err.
//!  - `StreamChunk::SupervisorPendingInjection` → pending count bumped.
//!  - `SessionCreated` → spawn_load_supervisors → SupervisorsLoaded fires.
//!
//! Mirrors the layout of isolated_session_dialog_rpc060.rs.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::views::agent::header_build::format_subordinate_label;
use codelet_fspec_tui::views::agent::SessionFooter;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{
    CompactionProgress, IncomingMessageInput, SessionId, StreamChunk, WorkspaceInfo,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn sample_input(source: &str, role: &str, message: &str) -> IncomingMessageInput {
    IncomingMessageInput {
        source_session_id: source.to_string(),
        role_name: role.to_string(),
        message: message.to_string(),
        images: None,
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
// Scenario: App::dispatch routes Action::SupervisorsLoaded through try_dispatch_rpc061
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn supervisors_loaded_writes_into_agent_view_store() {
    // @step Given an App wired to a MockBackend
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.agent_view_store_mut()
        .append_session(codelet_fspec_tui::SessionContext::new(sid("s-1")));

    // @step When Action::SupervisorsLoaded(SessionId("s-1"), vec![SessionId("sup")]) is dispatched
    app.dispatch(Action::SupervisorsLoaded(sid("s-1"), vec![sid("sup")]));
    drain_pending(&mut app).await;

    // @step Then store.supervisors_for(&SessionId("s-1")) returns &[SessionId("sup")]
    let supervisors = app.agent_view_store().supervisors_for(&sid("s-1"));
    assert_eq!(supervisors, &[sid("sup")]);
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Action::SendToSubordinate spawns backend.receive_incoming_message exactly once
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn send_to_subordinate_spawns_receive_incoming_message() {
    // @step Given an App with open session s-sup wired to a MockBackend whose receive_incoming_message returns Ok(())
    let mock = Arc::new(MockBackend::new());
    mock.seed_receive_incoming_message_result(Ok(()));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.agent_view_store_mut()
        .append_session(codelet_fspec_tui::SessionContext::new(sid("s-sup")));

    // @step When Action::SendToSubordinate { subordinate_id: SessionId("s-sub"), message: IncomingMessageInput { ... } } is dispatched
    let input = sample_input("s-sup", "reviewer", "please fix lint");
    app.dispatch(Action::SendToSubordinate {
        subordinate_id: sid("s-sub"),
        message: input.clone(),
    });

    // @step Then within 1 second backend.receive_incoming_message is called exactly once
    wait_until(
        || mock.receive_incoming_message_calls() == 1,
        "receive_incoming_message called once",
    )
    .await;
    drain_pending(&mut app).await;

    // @step And the payload matches subordinate_id=SessionId("s-sub") and the IncomingMessageInput
    let last = mock
        .last_received_incoming_message()
        .expect("must have recorded payload");
    assert_eq!(last.0, sid("s-sub"));
    assert_eq!(last.1, input);
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Action::SendToSubordinate Err path emits EmitSessionNotice
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn send_to_subordinate_err_path_emits_error_notice() {
    // @step Given an App with open session s-sup wired to a MockBackend whose receive_incoming_message returns Err("Failed to queue supervisor input: channel closed")
    let mock = Arc::new(MockBackend::new());
    mock.seed_receive_incoming_message_result(Err(
        "Failed to queue supervisor input: channel closed".to_string(),
    ));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.agent_view_store_mut()
        .append_session(codelet_fspec_tui::SessionContext::new(sid("s-sup")));

    // @step When Action::SendToSubordinate is dispatched
    app.dispatch(Action::SendToSubordinate {
        subordinate_id: sid("s-sub"),
        message: sample_input("s-sup", "reviewer", "noop"),
    });
    drain_pending(&mut app).await;

    // @step Then within 1 second Action::EmitSessionNotice with the documented text is observed
    wait_until(
        || {
            session_scrollback_text(&app, &sid("s-sup")).contains(
                "[error] send to subordinate: Failed to queue supervisor input: channel closed",
            )
        },
        "error notice in s-sup scrollback",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Action::SendToSubordinate Err path is a silent no-op without an open session
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn send_to_subordinate_err_without_open_session_is_silent_no_op() {
    // @step Given an App with NO open AgentView session wired to a MockBackend whose receive_incoming_message returns Err("e")
    let mock = Arc::new(MockBackend::new());
    mock.seed_receive_incoming_message_result(Err("e".to_string()));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When Action::SendToSubordinate is dispatched
    app.dispatch(Action::SendToSubordinate {
        subordinate_id: sid("s-sub"),
        message: sample_input("anywhere", "reviewer", "noop"),
    });

    // @step Then within 1 second backend.receive_incoming_message is called exactly once
    wait_until(
        || mock.receive_incoming_message_calls() == 1,
        "receive_incoming_message called once",
    )
    .await;

    // Drain background tasks so the JoinHandle on receive_incoming_message
    // (which would emit EmitSessionNotice on Err with an open session) has
    // a chance to enqueue any actions into the bus.
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }

    // @step And no Action::EmitSessionNotice is observed on the action bus
    //   (no open session means no scrollback to emit into; we drain the bus
    //   and assert the absence of EmitSessionNotice for sub-id "s-sub" /
    //   any session — silent no-op per the scenario contract).
    let mut observed_notice = false;
    while let Some(action) = app.try_recv_action() {
        if matches!(action, Action::EmitSessionNotice(_, _)) {
            observed_notice = true;
        }
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
    assert!(
        !observed_notice,
        "no Action::EmitSessionNotice should be observed on the action bus when no session is open"
    );
    assert_eq!(mock.receive_incoming_message_calls(), 1);
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: StreamChunk::SupervisorPendingInjection bumps per-session pending count
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn supervisor_pending_injection_chunk_bumps_pending_count() {
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
    drain_pending(&mut app).await;

    // @step Then store.supervisor_pending_count_for(&SessionId("s-sub")) returns 1
    assert_eq!(
        app.agent_view_store()
            .supervisor_pending_count_for(&sid("s-sub")),
        1
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Two consecutive SupervisorPendingInjection chunks bump count to 2
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_supervisor_pending_injection_chunks_bump_count_to_two() {
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
    drain_pending(&mut app).await;

    // @step Then store.supervisor_pending_count_for(&SessionId("s-sub")) returns 2
    assert_eq!(
        app.agent_view_store()
            .supervisor_pending_count_for(&sid("s-sub")),
        2
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: SessionCreated triggers spawn_load_supervisors
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_created_spawns_get_supervisors_and_fires_supervisors_loaded() {
    // @step Given an App wired to a MockBackend whose get_supervisors(SessionId("s-1")) returns vec![SessionId("sup")]
    let mock = Arc::new(MockBackend::new());
    mock.seed_supervisors_for(sid("s-1"), vec![sid("sup")]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When Action::SessionCreated(SessionId("s-1")) is dispatched
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

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
    .await;
    drain_pending(&mut app).await;

    let supervisors = app.agent_view_store().supervisors_for(&sid("s-1"));
    assert_eq!(supervisors, &[sid("sup")]);
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: A session with no supervisors shows no subordinate badge
// ─────────────────────────────────────────────────────────────────────

#[test]
fn no_supervisors_yields_no_subordinate_badge_label() {
    // @step Given the AgentViewStore has no supervisors recorded for session s-sub
    let supervisors: Vec<SessionId> = Vec::new();

    // @step When format_subordinate_label is called with an empty supervisors slice
    let label = format_subordinate_label(&supervisors);

    // @step Then the helper returns None and the SessionHeader paints no [Subordinate of: ...] badge
    assert!(
        label.is_none(),
        "format_subordinate_label(empty) must return None so the header paints no badge"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Multi-supervisor session renders subordinate badge with +N count
// ─────────────────────────────────────────────────────────────────────

#[test]
fn multi_supervisor_subordinate_label_renders_first8_plus_n() {
    // @step Given a session is recorded with three supervisors (s-sup-aaa, s-sup-bbb, s-sup-ccc)
    let supervisors = vec![sid("s-sup-aaa"), sid("s-sup-bbb"), sid("s-sup-ccc")];

    // @step When format_subordinate_label is called with that supervisors slice
    let label = format_subordinate_label(&supervisors);

    // @step Then the helper returns Some("s-sup-aa+2") (first 8 chars of the first supervisor id, plus +<count of remaining supervisors>)
    assert_eq!(label.as_deref(), Some("s-sup-aa+2"));
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
fn supervisor_pending_chip_suppresses_compaction_chip() {
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
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: AttachToSession triggers spawn_load_supervisors and SupervisorsLoaded
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn attach_to_session_spawns_get_supervisors_and_fires_supervisors_loaded() {
    // @step Given an App wired to a MockBackend whose get_supervisors(SessionId("s-sub")) returns vec![SessionId("sup")]
    let mock = Arc::new(MockBackend::new());
    mock.seed_supervisors_for(sid("s-sub"), vec![sid("sup")]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When Action::AttachToSession(SessionId("s-sub")) is dispatched
    app.dispatch(Action::AttachToSession(sid("s-sub")));
    drain_pending(&mut app).await;

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
    .await;
    drain_pending(&mut app).await;

    let supervisors = app.agent_view_store().supervisors_for(&sid("s-sub"));
    assert_eq!(supervisors, &[sid("sup")]);
}
