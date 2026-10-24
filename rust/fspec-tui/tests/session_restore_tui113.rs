//! BUG-206 — Auto-restore of the last active session on client startup and
//! reconnect. MEMORY-ONLY (supersedes the TUI-113 disk-aware fallbacks).
//!
//! Feature: spec/features/auto-restore-last-active-session-on-client-startup-and-reconnect.feature
//!
//! Every scenario exercises the `App::spawn_session_restore` /
//! `resolve_restore_target` restore path (app/session_restore.rs) against
//! the MockBackend. The liveness gate is the MockBackend's
//! `seed_live_sessions` scripting surface (BUG-206 R3: `list_live_sessions`
//! returns the daemon's in-memory session map — never persisted manifests).
//!
//! MockBackend note: the active-session surface (`seed_active_session` /
//! `set_active_session_calls` / `last_set_active_session`) and the
//! live-sessions surface (`seed_live_sessions`) only store scripted
//! state, so the assertions fail for the right reason (no restore
//! dispatch / no set_active_session call from App).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk};

mod common;
use common::MockBackend;

fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

/// A persisted (disk-only) session for `list_sessions` seeding — the
/// /resume picker list the auto-restore path must NEVER scan.
fn disk_session(id: &str) -> codelet_rpc_types::SessionInfo {
    codelet_rpc_types::SessionInfo {
        id: id.to_string(),
        name: format!("Session {id}"),
        status: "idle".to_string(),
        project: "/home/rquast/projects/fspec".to_string(),
        message_count: 0,
        provider_id: None,
        model_id: None,
        is_isolated: false,
        worktree_path: None,
        role: None,
        updated_at_ms: Some(1_000),
        work_unit_focus: Default::default(),
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
/// `Action::ReattachSession` onto the bus; without pumping, the
/// in-memory catch-up it triggers never runs and the wait times out.
/// (The restore task is fire-and-forget by design — the test harness
/// plays the role of the App run loop.)
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
    mock.seed_live_sessions(vec![sid("S-focus")]);
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

    // @step Then the focused session "S-focus" is re-attached via the live re-attach path
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("S-focus")),
        "the focused session must stay the current session after Reconnected"
    );

    // @step And no create_session RPC is issued
    assert_eq!(
        mock.create_session_calls(),
        create_calls_before,
        "BUG-206: reconnect with a focused LIVE session must NOT create a new one"
    );

    // @step And the re-attach NEVER runs the disk resume_session round-trip (BUG-206: the candidate is live — the disk durable-restore path would double-inject its LLM context)
    assert_eq!(
        mock.resume_session_calls(),
        0,
        "the memory-only re-attach must not call backend.resume_session"
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
    mock.seed_live_sessions(vec![sid("S-focus")]);
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
    drain_pending(&mut app).await;
    wait_until_drained(
        &mut app,
        || !mock.transcript_requests().is_empty(),
        "backend transcript pages for catch-up",
    )
    .await;
    drain_pending(&mut app).await;

    // @step Then the 3 chunks are replayed into "S-focus"'s scrollback via the in-memory buffered-output catch-up
    let text = session_scrollback_text(&app, &sid("S-focus"));
    for i in 1..=3 {
        assert!(
            text.contains(&format!("chunk-{i}")),
            "replayed chunk-{i} missing from S-focus scrollback, got: {text:?}"
        );
    }

    // @step And the catch-up came from the IN-MEMORY buffer — no disk resume_session
    assert_eq!(
        mock.resume_session_calls(),
        0,
        "BUG-206: the reconnect catch-up must not resume from disk"
    );

    // @step And live streaming chunks continue after the replay
    // (manual poll loop: the live-chunk check reads app state that
    // `wait_until_drained` borrows exclusively)
    mock.set_buffered_output(vec![
        StreamChunk::text("chunk-1".to_string()),
        StreamChunk::text("chunk-2".to_string()),
        StreamChunk::text("chunk-3".to_string()),
        StreamChunk::text("live-after-replay".to_string()),
    ]);
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
    mock.seed_live_sessions(vec![sid("S-active")]);
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
        || mock.list_live_sessions_calls() >= 1,
        "bootstrap restore to query the daemon's in-memory session map",
    )
    .await;
    drain_pending(&mut app).await;

    // @step And the client re-attaches "S-active" via the live re-attach path
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("S-active")),
        "S-active must become the open+focused session"
    );

    // @step And the re-attach is memory-only — NO disk resume_session round-trip
    assert_eq!(
        mock.resume_session_calls(),
        0,
        "BUG-206: restoring a live session must not resume from disk (it would double-inject the LLM context)"
    );

    // @step And the TUI lands in the Agent view on "S-active" with its scrollback replayed
    assert_eq!(
        app.active_view(),
        codelet_fspec_tui::ViewMode::Agent,
        "restore must flip the TUI into the Agent view"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 4: Fresh client process lands on the board when no session is live
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fresh_client_process_lands_on_the_board_when_no_session_is_live() {
    // @step Given a daemon's active-session pointer is unset (e.g. no client has attached yet after a daemon restart)
    let mock = Arc::new(MockBackend::new());
    // get_active_session defaults to Ok(None) on the MockBackend.

    // @step And the daemon's workspace has ONLY persisted sessions on disk (["S-older", "S-recent"]) — none of them is in the daemon's in-memory map
    // (BUG-206: persisted manifests are NEVER candidates — the auto-
    //  restore path must not scan list_sessions at all.)
    mock.seed_sessions(vec![disk_session("S-recent"), disk_session("S-older")]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When a new client process starts and App::bootstrap runs
    app.bootstrap().await.expect("bootstrap");
    // No candidates (no pointer, no focused session) — the resolver
    // returns None without even querying liveness. The restore task is
    // on pending_tasks, so drain it.
    drain_pending(&mut app).await;

    // @step Then nothing is restored — the TUI stays on the board with NO open session
    assert!(
        app.agent_view_store().open_sessions().is_empty(),
        "no session may be re-opened when nothing is live in the daemon"
    );
    assert_eq!(
        app.active_view(),
        codelet_fspec_tui::ViewMode::Board,
        "the client must land on the board, not the Agent view"
    );

    // @step And the restore path never touched the disk — no resume_session and no list_sessions call
    assert_eq!(
        mock.resume_session_calls(),
        0,
        "BUG-206: the auto-restore path must never resume from disk"
    );
    assert_eq!(
        mock.list_sessions_calls(),
        0,
        "BUG-206: the auto-restore path must never scan list_sessions (disk manifests)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 5: Daemon restart lands the client on the board (the pointer is gone)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn daemon_restart_lands_the_client_on_the_board() {
    // @step Given a daemon was killed and relaunched so its in-memory session map is empty
    let mock = Arc::new(MockBackend::new());

    // @step And a client stays connected and its supervisor reconnects
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    // Fresh client: no focused session yet (open_sessions empty).

    // @step And the re-attach finds no focused session on the fresh daemon
    // (get_active_session → Ok(None) — the in-memory pointer is lost on
    // daemon restart; the persisted manifests still hold the old
    // sessions, but BUG-206: they are NOT candidates)
    app.dispatch(Action::Reconnected);
    drain_pending(&mut app).await;

    // @step Then nothing is re-attached — the persisted sessions are NOT resumed from disk
    assert!(
        app.agent_view_store().open_sessions().is_empty(),
        "a restarted daemon with an empty in-memory map must not resurrect closed sessions"
    );
    assert_eq!(
        mock.resume_session_calls(),
        0,
        "BUG-206: the reconnect path must never resume-from-disk"
    );
    assert_eq!(
        mock.create_session_calls(),
        0,
        "BUG-206: the reconnect path must never create a session (RPC-012 lazy create is the only creation path)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 6: Empty daemon lands on the board on reconnect (NO create fallback)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn empty_daemon_lands_on_the_board_on_reconnect_without_creating_a_session() {
    // @step Given a fresh daemon with no active session and no live sessions
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step And a client's supervisor reconnects with no focused session
    app.dispatch(Action::Reconnected);

    // @step When the restore resolution yields no candidate
    // (the resolution task queries list_live_sessions → empty; draining
    // lets the fire-and-forget resolution complete)
    drain_pending(&mut app).await;

    // @step Then NO create_session is issued — the client lands on the board
    assert_eq!(
        mock.create_session_calls(),
        0,
        "BUG-206: an empty daemon must NOT trigger create_session on reconnect (RPC-012 lazy create-on-enter-work-unit is the only creation path)"
    );
    assert!(
        app.agent_view_store().open_sessions().is_empty(),
        "no session tab may exist after an empty-daemon reconnect"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 7: A closed candidate is never restored — the liveness gate drops it
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_closed_candidate_is_never_restored() {
    // @step Given the daemon's active pointer still references session "S-closed" but "S-closed" is NOT in the daemon's in-memory map (it was closed)
    let mock = Arc::new(MockBackend::new());
    mock.seed_active_session(sid("S-closed"));
    // BUG-206 R4: the daemon clears the pointer on destroy — model the
    // pointer-survives-older-daemon case by pointing at a session that
    // simply is not live.
    mock.seed_live_sessions(vec![sid("S-live-other")]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When a new client process starts and App::bootstrap runs
    app.bootstrap().await.expect("bootstrap");
    wait_until_drained(
        &mut app,
        || mock.list_live_sessions_calls() >= 1,
        "the restore resolution to query liveness",
    )
    .await;
    drain_pending(&mut app).await;

    // @step Then the closed "S-closed" candidate is dropped by the liveness gate — nothing restores
    assert!(
        app.agent_view_store().open_sessions().is_empty(),
        "the closed pointer target must NOT be re-opened"
    );
    assert_eq!(
        mock.resume_session_calls(),
        0,
        "BUG-206: a non-live candidate must never be resumed from disk"
    );
    assert!(
        !app.agent_view_store()
            .open_sessions()
            .iter()
            .any(|c| c.id == sid("S-closed")),
        "the closed session must not appear in open_sessions"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Reconnect does not recreate a destroyed session
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reconnect_does_not_recreate_a_destroyed_session() {
    // @step Given a client whose last-viewed session "S-x" was destroyed and closed while the client was running
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("S-x")));
    let removed = app
        .agent_view_store_mut()
        .remove_session_if_open(&sid("S-x"));
    assert!(removed, "precondition: S-x tab must be closed");

    // @step And the daemon has no sessions and no active pointer for its workspace
    // (the destroyed session left no trace: get_active_session → None,
    //  list_live_sessions → empty)

    // @step When the client reconnects
    app.dispatch(Action::Reconnected);
    drain_pending(&mut app).await;

    // @step Then the closed "S-x" tab is NOT re-opened and NO session is created
    assert!(
        app.agent_view_store()
            .session_context_for(&sid("S-x"))
            .is_none(),
        "the destroyed/closed session must not be resurrected by the restore path"
    );
    assert_eq!(
        mock.create_session_calls(),
        0,
        "BUG-206: no create_session fallback on reconnect"
    );
    assert_eq!(
        mock.resume_session_calls(),
        0,
        "BUG-206: no disk resume of a destroyed session"
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
    // (Each SessionCreated focuses the new tab: a → b → c. R3
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

    // @step And on the next client start "S-c" restores (because it is LIVE) — not "S-a" and not a disk entry
    let mock2 = Arc::new(MockBackend::new());
    mock2.seed_active_session(sid("S-c"));
    mock2.seed_live_sessions(vec![sid("S-c")]);
    let backend2: Arc<dyn FspecBackend> = mock2.clone();
    let mut app2 = App::new(backend2);
    app2.bootstrap().await.expect("bootstrap");
    wait_until_drained(
        &mut app2,
        || mock2.list_live_sessions_calls() >= 1,
        "next start to query liveness for the live pointer restore",
    )
    .await;
    drain_pending(&mut app2).await;
    assert_eq!(
        app2.agent_view_store().current_session(),
        Some(&sid("S-c")),
        "the last-focused LIVE session must win over first/most-recent-by-time"
    );
    assert_eq!(
        mock2.resume_session_calls(),
        0,
        "BUG-206: the restore of a live pointer target must not resume from disk"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 9: Repeated reconnects re-attach idempotently
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn repeated_reconnects_re_attach_idempotently() {
    // @step Given a client is viewing session "S-focus"
    let mock = Arc::new(MockBackend::new());
    mock.seed_live_sessions(vec![sid("S-focus")]);
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
// Scenario 10: Esc back to board keeps the active pointer sticky so restart
//             restores the last-viewed session
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

    // @step And on the next client start "S-focus" restores as the last-viewed session (while it is still live)
    let mock2 = Arc::new(MockBackend::new());
    mock2.seed_active_session(sid("S-focus"));
    mock2.seed_live_sessions(vec![sid("S-focus")]);
    let backend2: Arc<dyn FspecBackend> = mock2.clone();
    let mut app2 = App::new(backend2);
    app2.bootstrap().await.expect("bootstrap");
    wait_until_drained(
        &mut app2,
        || mock2.list_live_sessions_calls() >= 1,
        "sticky pointer restore to query liveness on the next start",
    )
    .await;
    drain_pending(&mut app2).await;
    assert_eq!(
        app2.agent_view_store().current_session(),
        Some(&sid("S-focus")),
        "the sticky pointer must restore S-focus while it is still live"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 11: A stale pointer is skipped — the daemon's live sessions win
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stale_pointer_is_skipped_and_the_focused_live_session_wins() {
    // @step Given a client reconnects while focused on session "S-focus" (the tab survived the WS drop) and the daemon pointer still references the CLOSED "S-closed" session
    let mock = Arc::new(MockBackend::new());
    mock.seed_active_session(sid("S-closed"));
    mock.seed_live_sessions(vec![sid("S-focus")]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("S-focus")));
    app.dispatch(Action::Disconnected);

    // @step When the supervisor reconnects and Action::Reconnected is dispatched
    app.dispatch(Action::Reconnected);
    drain_pending(&mut app).await;

    // @step Then the client re-attaches the focused LIVE session "S-focus"
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("S-focus")),
        "the focused live session must win over the stale pointer"
    );

    // @step And the closed pointer target is never restored
    assert!(
        app.agent_view_store()
            .session_context_for(&sid("S-closed"))
            .is_none(),
        "the closed pointer target must not be re-opened"
    );
    assert_eq!(
        mock.resume_session_calls(),
        0,
        "BUG-206: memory-only re-attach — no disk resume"
    );
}
