//! TUI-113 — Auto-restore last active session on client startup and reconnect.
//!
//! Feature: spec/features/auto-restore-last-active-session-on-client-startup-and-reconnect.feature
//!
//! Green-phase integration tests: every scenario exercises the
//! `App::spawn_session_restore` / `App::resolve_restore_target` restore
//! path (app/session_restore.rs) and the `set_active_session`
//! fire-and-forget calls (R3) against the MockBackend.
//!
//! MockBackend note: the active-session surface (`seed_active_session` /
//! `set_active_session_calls` / `last_set_active_session`) is added to
//! `tests/common/mod.rs` as part of this work unit's test scaffolding —
//! it only stores scripted state (no production behaviour), so the RED
//! assertions still fail for the right reason (no restore dispatch /
//! no set_active_session call from App).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, SessionInfo, StreamChunk, WorkspaceInfo};

mod common;
use common::MockBackend;

fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

fn session_info(id: &str, project: &str) -> SessionInfo {
    SessionInfo {
        id: id.to_string(),
        name: format!("Session {id}"),
        status: "idle".to_string(),
        project: project.to_string(),
        message_count: 0,
        provider_id: None,
        model_id: None,
        is_isolated: false,
        worktree_path: None,
        role: None,
        updated_at_ms: Some(1_000),
    }
}

fn workspace(cwd: &str) -> WorkspaceInfo {
    WorkspaceInfo {
        cwd: cwd.to_string(),
        git_branch: None,
    }
}

/// Drain the action bus + await every spawned task, pumping new actions
/// until quiescent (bounded iterations to avoid runaway loops in RED).
async fn drain_pending(app: &mut App) {
    for _ in 0..64 {
        let mut progress = false;
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
            progress = true;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
            progress = true;
        }
        if !progress {
            break;
        }
    }
}

/// Poll `pred` until true or 2s elapses.
async fn wait_until(mut pred: impl FnMut() -> bool, what: &str) {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        if pred() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("timed out waiting for: {what}");
}

/// Like [`wait_until`] but also pumps the action bus + awaits pending
/// tasks on every iteration — the restore path only SENDS
/// `Action::AttachToSession` onto the bus; without pumping, the
/// resume_session round-trip it triggers never runs and the wait times
/// out. (The restore task is fire-and-forget by design — R1 — the test
/// harness plays the role of the App run loop.)
async fn wait_until_drained(app: &mut App, mut pred: impl FnMut() -> bool, what: &str) {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        if pred() {
            return;
        }
        drain_pending(app).await;
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("timed out waiting for: {what}");
}

fn session_ids(app: &App, session_id: &SessionId) -> usize {
    app.agent_view_store()
        .open_sessions()
        .iter()
        .filter(|c| c.id == *session_id)
        .count()
}

/// Flatten every visible chunk's text in `id`'s scrollback into one
/// String (mirrors `session_scrollback_text` in slash_resume_rpc049.rs).
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

fn last_set_active_is(mock: &MockBackend, id: &SessionId) -> bool {
    mock.last_set_active_session().as_ref() == Some(id)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 1: Reconnect re-attaches the focused session instead of
//             creating a new one
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reconnect_reattaches_the_focused_session_instead_of_creating_a_new_one() {
    // @step Given a client is viewing session "S-focus" and the WebSocket connection drops
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("S-focus")));
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("S-focus")),
        "precondition: focused session is S-focus"
    );
    app.dispatch(Action::Disconnected);

    // @step When the supervisor reconnects and Action::Reconnected is dispatched
    let create_calls_before = mock.create_session_calls();
    app.dispatch(Action::Reconnected);
    drain_pending(&mut app).await;

    // @step Then the focused session "S-focus" is re-attached via the existing attach path
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("S-focus")),
        "the focused session must stay the current session after Reconnected"
    );
    wait_until(
        || mock.resume_session_calls() >= 1,
        "backend.resume_session for the re-attach round-trip",
    )
    .await;
    assert_eq!(
        mock.last_resume_session(),
        Some(sid("S-focus")),
        "re-attach must go through the existing RPC-049 resume_session path"
    );

    // @step And no create_session RPC is issued
    assert_eq!(
        mock.create_session_calls(),
        create_calls_before,
        "TUI-113 R4/R5: reconnect with a focused session must NOT create a new one"
    );

    // @step And the inline reconnect notice lands in the "S-focus" session's scrollback
    let text = session_scrollback_text(&app, &sid("S-focus"));
    assert!(
        text.contains("Reconnected"),
        "the inline 'Reconnected' success line must land in S-focus, got: {text:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 2: Reconnect replays output streamed while the connection was down
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reconnect_replays_output_streamed_while_the_connection_was_down() {
    // @step Given a client is viewing session "S-focus" and the WebSocket connection drops
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("S-focus")));
    app.dispatch(Action::Disconnected);

    // @step And the daemon streamed 3 assistant text chunks for "S-focus" while the connection was down
    mock.set_buffered_output(vec![
        StreamChunk::text("chunk-1".to_string()),
        StreamChunk::text("chunk-2".to_string()),
        StreamChunk::text("chunk-3".to_string()),
    ]);

    // @step When the supervisor reconnects and the re-attach completes
    app.dispatch(Action::Reconnected);
    wait_until_drained(
        &mut app,
        || mock.get_buffered_output_calls() >= 1,
        "backend.get_buffered_output for the replay",
    )
    .await;
    drain_pending(&mut app).await;

    // @step Then the 3 chunks are replayed into "S-focus"'s scrollback via the buffered-output round-trip
    let text = session_scrollback_text(&app, &sid("S-focus"));
    for i in 1..=3 {
        assert!(
            text.contains(&format!("chunk-{i}")),
            "replayed chunk-{i} missing from S-focus scrollback, got: {text:?}"
        );
    }

    // @step And live streaming chunks continue after the replay
    // (manual poll loop: the live-chunk check reads app state that
    // `wait_until_drained` borrows exclusively)
    mock.push_chunk(
        sid("S-focus"),
        StreamChunk::text("live-after-replay".to_string()),
    );
    let live_deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        drain_pending(&mut app).await;
        if session_scrollback_text(&app, &sid("S-focus")).contains("live-after-replay") {
            break;
        }
        assert!(
            std::time::Instant::now() < live_deadline,
            "live chunk did not arrive after the replay"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 3: Fresh client process restores the daemon's active session on bootstrap
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fresh_client_process_restores_the_daemon_s_active_session_on_bootstrap() {
    // @step Given a daemon has a live session "S-active" set as its active session
    let mock = Arc::new(MockBackend::new());
    mock.seed_active_session(sid("S-active"));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When a new client process starts and App::bootstrap runs
    app.bootstrap().await.expect("bootstrap");

    // @step Then bootstrap returns without awaiting the restore
    // (bootstrap returned above; the restore is a detached task — the
    // assertions below observe it asynchronously; the harness pumps the
    // action bus because it plays the role of the App run loop)
    wait_until_drained(
        &mut app,
        || mock.resume_session_calls() >= 1,
        "bootstrap restore to dispatch AttachToSession(S-active)",
    )
    .await;
    drain_pending(&mut app).await;

    // @step And the client re-attaches "S-active" via the existing durable-restore path
    assert_eq!(
        mock.last_resume_session(),
        Some(sid("S-active")),
        "the restore must target the daemon's active session via the RPC-049 path"
    );
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("S-active")),
        "S-active must become the open+focused session"
    );

    // @step And the TUI lands in the Agent view on "S-active" with its scrollback replayed
    assert_eq!(
        app.active_view(),
        codelet_fspec_tui::ViewMode::Agent,
        "restore must flip the TUI into the Agent view"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 4: Fresh client process restores the most-recent workspace
//             session when no active pointer exists
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fresh_client_process_restores_the_most_recent_workspace_session_when_no_active_pointer_exists(
) {
    // @step Given a daemon's active-session pointer is unset (e.g. no client has attached yet after a daemon restart)
    let mock = Arc::new(MockBackend::new());
    // get_active_session defaults to Ok(None) on the MockBackend.
    mock.set_workspace_info(workspace("/home/rquast/projects/fspec"));

    // @step And the daemon's workspace has persisted sessions ["S-older", "S-recent"] by recency
    // TUI-099: list_sessions is sorted by updated_at_ms DESC, most-recent first.
    mock.seed_sessions(vec![
        session_info("S-recent", "/home/rquast/projects/fspec"),
        session_info("S-older", "/home/rquast/projects/fspec"),
    ]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When a new client process starts and App::bootstrap runs
    app.bootstrap().await.expect("bootstrap");
    wait_until_drained(
        &mut app,
        || mock.resume_session_calls() >= 1,
        "bootstrap restore to pick the most-recent session",
    )
    .await;
    drain_pending(&mut app).await;

    // @step Then the client restores "S-recent" — the most-recent entry of list_sessions(daemon_cwd)
    assert_eq!(
        mock.last_resume_session(),
        Some(sid("S-recent")),
        "the restore must pick the most-recent entry of list_sessions(daemon_cwd)"
    );

    // @step And "S-older" is not restored
    assert_eq!(
        mock.resume_session_calls(),
        1,
        "exactly one session (S-recent) may be restored, got {:#?}",
        mock.resume_session_calls()
    );
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("S-recent")),
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 5: Daemon restart restores the persisted most-recent session
//             via resume-from-disk
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn daemon_restart_restores_the_persisted_most_recent_session_via_resume_from_disk() {
    // @step Given a daemon was killed and relaunched so its in-memory session map is empty
    let mock = Arc::new(MockBackend::new());
    mock.set_workspace_info(workspace("/home/rquast/projects/fspec"));

    // @step And a client stays connected and its supervisor reconnects
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    // Fresh client: no focused session yet (open_sessions empty).

    // @step And the re-attach finds no focused session on the fresh daemon
    // (get_active_session → Ok(None) — the in-memory pointer is lost on
    // daemon restart; the persisted list still holds the session)
    mock.seed_sessions(vec![session_info(
        "S-persisted",
        "/home/rquast/projects/fspec",
    )]);

    // @step When the restore falls back to the persisted sessions for the daemon's workspace
    app.dispatch(Action::Reconnected);
    wait_until_drained(
        &mut app,
        || mock.resume_session_calls() >= 1,
        "reconnect restore to resume the persisted session",
    )
    .await;
    drain_pending(&mut app).await;

    // @step Then the most-recent persisted session is re-attached via the resume_session round-trip
    assert_eq!(
        mock.last_resume_session(),
        Some(sid("S-persisted")),
        "the persisted most-recent session must go through resume_session"
    );
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("S-persisted")),
    );

    // @step And its conversation history replays from disk
    wait_until_drained(
        &mut app,
        || mock.get_buffered_output_calls() >= 1,
        "get_buffered_output replay after resume",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 6: Empty daemon still creates a session on reconnect
//             (RPC-011 safety net preserved)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn empty_daemon_still_creates_a_session_on_reconnect() {
    // @step Given a fresh daemon with no active session and no persisted sessions for its workspace
    let mock = Arc::new(MockBackend::new());
    mock.script_create_session(sid("S-fallback"));
    mock.set_workspace_info(workspace("/empty/workspace"));
    // list_sessions → empty (no seed), get_active_session → None.

    // @step And a client's supervisor reconnects with no focused session
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When the restore resolution yields no candidate
    app.dispatch(Action::Reconnected);

    // @step Then create_session(None) is issued as the fallback
    wait_until_drained(
        &mut app,
        || mock.create_session_calls() >= 1,
        "RPC-011 fallback create_session on an empty daemon",
    )
    .await;
    drain_pending(&mut app).await;

    // @step And the new session becomes the active session exactly as RPC-011 specified
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("S-fallback")),
        "the fallback-created session must become the active session"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 7: Restore queries sessions by the daemon workspace path, not the client cwd
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restore_queries_sessions_by_the_daemon_workspace_path_not_the_client_cwd() {
    // @step Given the client machine's cwd is "/Users/remote/code"
    // (Modelled by the test process's real cwd — whatever it is, it must
    // NOT be what the restore path passes to list_sessions.)

    // @step And the daemon workspace is "/home/rquast/projects/fspec"
    let mock = Arc::new(MockBackend::new());
    mock.set_workspace_info(workspace("/home/rquast/projects/fspec"));

    // @step And a session "S-ds" exists for the daemon workspace but NOT for the client cwd
    mock.seed_sessions(vec![session_info("S-ds", "/home/rquast/projects/fspec")]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When a fresh client process resolves the restore target
    app.bootstrap().await.expect("bootstrap");
    wait_until_drained(
        &mut app,
        || mock.list_sessions_project().is_some(),
        "list_sessions to be called during restore resolution",
    )
    .await;

    // @step Then list_sessions is called with "/home/rquast/projects/fspec" (from get_workspace_info)
    assert_eq!(
        mock.list_sessions_project(),
        Some("/home/rquast/projects/fspec".to_string()),
        "R2(c): the daemon workspace cwd must be passed — never the client cwd"
    );

    // @step And "S-ds" is a candidate and gets restored
    wait_until_drained(
        &mut app,
        || mock.last_resume_session() == Some(sid("S-ds")),
        "S-ds to be restored via resume_session",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Restore does not re-create a closed session
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restore_does_not_recreate_a_closed_session() {
    // @step Given a client whose last-viewed session "S-x" was destroyed and closed while the client was running
    let mock = Arc::new(MockBackend::new());
    // The closed tab must NOT be re-opened: the create fallback (if it
    // fires) returns a FRESH session id, never the destroyed one.
    mock.script_create_session(sid("S-fresh"));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("S-x")));
    let removed = app
        .agent_view_store_mut()
        .remove_session_if_open(&sid("S-x"));
    assert!(removed, "precondition: S-x tab must be closed");

    // @step And the daemon has no sessions and no active pointer for its workspace
    // (the destroyed session left no trace: get_active_session → None,
    // list_sessions → empty)

    // @step When the client reconnects
    app.dispatch(Action::Reconnected);
    drain_pending(&mut app).await;

    // @step Then the R4 create_session fallback fires exactly once (RPC-011 safety net)
    assert_eq!(
        mock.create_session_calls(),
        1,
        "a daemon with nothing restorable still creates a session on reconnect"
    );

    // @step And the closed "S-x" tab is not re-opened
    assert!(
        app.agent_view_store()
            .session_context_for(&sid("S-x"))
            .is_none(),
        "the destroyed/closed session must not be resurrected by the restore path"
    );

    // @step And the fallback-created session becomes the active session
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("S-fresh")),
        "the fallback-created session must be the active session"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 8: Focus switches track the daemon active pointer
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn focus_switches_track_the_daemon_active_pointer() {
    // @step Given a client has three open sessions "S-a", "S-b", "S-c"
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    // Drain between creations so each set_active_session round-trip
    // completes before the next one starts — mirrors the real-world
    // event sequence (creations arrive sequentially over time, not as
    // three synchronously-spawned racing tasks).
    app.dispatch(Action::SessionCreated(sid("S-a")));
    drain_pending(&mut app).await;
    app.dispatch(Action::SessionCreated(sid("S-b")));
    drain_pending(&mut app).await;
    app.dispatch(Action::SessionCreated(sid("S-c")));
    drain_pending(&mut app).await;

    // @step When the user cycles focus S-a → S-b → S-c
    // (Each SessionCreated focuses the new tab: a → b → c. TUI-113 R3
    // must publish set_active_session on every creation/focus switch.)
    wait_until(
        || last_set_active_is(&mock, &sid("S-c")),
        "set_active_session to track the last-focused session S-c",
    )
    .await;

    // @step Then set_active_session is called with "S-c" last
    assert_eq!(
        mock.last_set_active_session(),
        Some(sid("S-c")),
        "R3: the last focus switch must be published to the daemon"
    );

    // @step And on the next client start "S-c" restores — not "S-a" and not the most-recent-by-time session
    let mock2 = Arc::new(MockBackend::new());
    mock2.seed_active_session(sid("S-c"));
    let backend2: Arc<dyn FspecBackend> = mock2.clone();
    let mut app2 = App::new(backend2);
    app2.bootstrap().await.expect("bootstrap");
    wait_until_drained(
        &mut app2,
        || mock2.last_resume_session() == Some(sid("S-c")),
        "next start to restore S-c via the daemon pointer",
    )
    .await;
    assert_eq!(
        mock2.last_resume_session(),
        Some(sid("S-c")),
        "the last-focused session must win over first/most-recent-by-time"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 9: Repeated reconnects re-attach idempotently
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn repeated_reconnects_re_attach_idempotently() {
    // @step Given a client is viewing session "S-focus"
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("S-focus")));

    // @step When the WebSocket flaps and Action::Reconnected fires twice in quick succession
    app.dispatch(Action::Reconnected);
    app.dispatch(Action::Reconnected);
    drain_pending(&mut app).await;

    // @step Then both re-attach targets resolve to the already-open "S-focus" tab
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("S-focus")),
    );

    // @step And exactly one session exists for "S-focus" in the open-session list
    assert_eq!(
        session_ids(&app, &sid("S-focus")),
        1,
        "the append path must be idempotent — no duplicate S-focus tab"
    );

    // @step And no duplicate session tab is appended
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        1,
        "exactly one open session tab after two reconnects"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 10: Esc back to board keeps the active pointer sticky so
//              restart restores the last-viewed session
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_back_to_board_keeps_the_active_pointer_sticky() {
    // @step Given a client is viewing session "S-focus"
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("S-focus")));
    wait_until(
        || last_set_active_is(&mock, &sid("S-focus")),
        "set_active_session to be published on attach/create",
    )
    .await;

    // @step When the user presses Esc to exit to the board view
    app.dispatch(Action::BackToBoard);
    drain_pending(&mut app).await;

    // @step Then the daemon's active-session pointer still references "S-focus" (it is sticky — not cleared on board exit)
    assert_eq!(
        mock.last_set_active_session(),
        Some(sid("S-focus")),
        "user decision 2026-10-10: BackToBoard must NOT clear the daemon pointer"
    );
    assert_eq!(
        app.active_view(),
        codelet_fspec_tui::ViewMode::Board,
        "the view must still flip back to the board"
    );

    // @step And on the next client start "S-focus" restores as the last-viewed session
    let mock2 = Arc::new(MockBackend::new());
    mock2.seed_active_session(sid("S-focus"));
    let backend2: Arc<dyn FspecBackend> = mock2.clone();
    let mut app2 = App::new(backend2);
    app2.bootstrap().await.expect("bootstrap");
    wait_until_drained(
        &mut app2,
        || mock2.last_resume_session() == Some(sid("S-focus")),
        "sticky pointer to restore S-focus on the next start",
    )
    .await;
}
