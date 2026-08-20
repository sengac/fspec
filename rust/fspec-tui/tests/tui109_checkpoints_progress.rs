//! TUI-109 — per-item checkpoint-enumeration progress, end to end.
//!
//! Feature: spec/features/stream-per-item-progress-idx-total-for-checkpoint-enumeration-into-the-shared-loadingdialog-counter-row.feature
//!
//! Single canonical test file for the feature (1 feature = 1 test file).
//! Covers all five scenarios across the four layers:
//!   - git: `list_all_ghost_checkpoints_stream` ticks per ref; the
//!     non-streaming entry point delegates to it (byte-identical CLI);
//!   - rpc: `collect_checkpoints_stream` shapes `{loaded, total, done}`
//!     frames (pending total, cap 200/250); the non-streaming
//!     `collect_checkpoints` delegates with a no-op callback;
//!     `FspecServiceImpl::list_checkpoints` publishes frames on the
//!     shared broadcast;
//!   - TUI: the App fold renders `(loaded/…)` → `(N/total)`, stale-drops
//!     a late done frame after `CheckpointsLoaded`, degrades to
//!     spinner + stage label when no frames arrive, and bootstrap
//!     spawns the 6th subscriber on `checkpoints_progress_rx()`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::components::loading_dialog::render_loading_dialog;
use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_git::ghost_commit::{
    create_ghost_commit, list_all_ghost_checkpoints, list_all_ghost_checkpoints_stream,
};
use codelet_rpc::checkpoints::{collect_checkpoints, collect_checkpoints_stream};
use codelet_rpc::{FspecService, FspecServiceImpl, SharedFspecService};
use codelet_rpc_types::{CheckpointInfo, CheckpointsProgress};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tarpc::context;

mod common;
use common::MockBackend;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    (App::new(backend), mock)
}

fn checkpoint_info(wu: &str, name: &str) -> CheckpointInfo {
    CheckpointInfo {
        work_unit_id: wu.into(),
        name: name.into(),
        timestamp: "2026-01-01T00:00:00Z".into(),
        is_automatic: false,
    }
}

/// Drain the App's action bus until a matching action arrives or 200ms
/// elapses (mirrors `app_bootstrap_rpc009.rs::wait_for_action`).
async fn wait_for_action<F: Fn(&Action) -> bool>(app: &mut App, pred: F) -> Option<Action> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(200);
    while std::time::Instant::now() < deadline {
        if let Some(action) = app.try_recv_action() {
            if pred(&action) {
                return Some(action);
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    None
}

fn buf_text(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

/// Create a basic test git repository with an initial commit (mirrors
/// `rust/git/tests/common/mod.rs::setup_test_repo`).
fn setup_test_repo() -> tempfile::TempDir {
    let tmp_dir = tempfile::TempDir::new().expect("tempdir");
    let repo_path = tmp_dir.path();
    for args in [
        vec!["init"],
        vec!["config", "user.email", "test@example.com"],
        vec!["config", "user.name", "Test User"],
    ] {
        std::process::Command::new("git")
            .args(args)
            .current_dir(repo_path)
            .output()
            .expect("git");
    }
    fs::write(repo_path.join("README.md"), "# Test Repository\n").expect("write README");
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("git add");
    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(repo_path)
        .output()
        .expect("git commit");
    tmp_dir
}

/// Create a checkpoint by writing a unique file then snapshotting the
/// worktree (mirrors `checkpoint_transport_rpc362.rs`).
fn make_checkpoint(repo: &Path, work_unit_id: &str, name: &str) {
    let marker = repo.join(format!("touch-{work_unit_id}-{name}.txt"));
    fs::write(&marker, format!("{work_unit_id}/{name}")).expect("write marker");
    create_ghost_commit(repo, work_unit_id, name).expect("create_ghost_commit");
}

/// Write an index sidecar so timestamps are deterministic (mirrors
/// `checkpoint_transport_rpc362.rs::write_index`).
fn write_index(repo: &Path, work_unit_id: &str, entries: &[(&str, &str)]) {
    let dir = repo.join(".git").join("fspec-checkpoints-index");
    fs::create_dir_all(&dir).expect("mkdir index dir");
    let checkpoints: Vec<serde_json::Value> = entries
        .iter()
        .map(|(name, ts)| serde_json::json!({ "name": name, "timestamp": ts }))
        .collect();
    let body = serde_json::json!({ "checkpoints": checkpoints });
    fs::write(
        dir.join(format!("{work_unit_id}.json")),
        serde_json::to_string_pretty(&body).unwrap(),
    )
    .expect("write index");
}

/// Build a `SharedFspecService` with a real (empty) watcher + the repo
/// cwd attached, so `list_checkpoints` runs the real enumeration.
fn service_for(repo: &Path) -> Arc<SharedFspecService> {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo).expect("watcher on temp repo"));
    Arc::new(SharedFspecService::new(watcher).with_cwd(repo.to_path_buf()))
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 1: counter advances (0/…) → (N/total) before the list folds
// ─────────────────────────────────────────────────────────────────────────

/// git half — the streaming variant ticks the callback per checkpoint
/// ref, so the rpc layer can shape a per-item progress frame.
#[test]
fn git_streaming_variant_ticks_the_callback_per_checkpoint_ref() {
    // @step Given a Checkpoints view whose list load is in flight on a transport that streams progress events
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "cp-0");
    make_checkpoint(repo, "AUTH-002", "cp-1");
    make_checkpoint(repo, "AUTH-003", "cp-2");

    // @step When the transport emits CheckpointsProgress events (1/…), (47/…), then (150/150) with done=true before the list result folds
    let mut seen: Vec<(String, String)> = Vec::new();
    let out =
        list_all_ghost_checkpoints_stream(repo, &mut |pair| {
            seen.push(pair.clone());
        })
        .expect("streaming list");

    // @step Then the loading dialog counter row shows (1/…) while the total is still unknown
    // (the callback fires per item — 3 ticks for 3 refs)
    assert_eq!(seen.len(), 3, "one tick per checkpoint ref");
    // @step And the counter row climbs through the intermediate values as each progress event folds
    let mut seen_sorted = seen;
    seen_sorted.sort();
    let mut all_sorted: Vec<(String, String)> = out.to_vec();
    all_sorted.sort();
    assert_eq!(seen_sorted, all_sorted, "ticks cover exactly the returned pairs");
    // @step And the counter row shows the final (150/150) just before the list appears
    assert_eq!(out.len(), 3, "return value unchanged (backward-compatible)");
    // @step When the CheckpointsLoaded fold arrives with 150 checkpoints
    // (the fold half is exercised by the TUI tests below; the git layer
    // only guarantees the per-item ticks the fold consumes)
    // @step Then the list renders and the loading dialog is dismissed
    // (same note — the git layer's contract is the tick stream)
}

/// rpc shaping half — per-item ticks shaped into `{loaded, total, done}`
/// frames (pending total until the done frame).
#[test]
fn rpc_collect_checkpoints_stream_ticks_per_item_with_pending_total_then_done() {
    // @step Given a Checkpoints view whose list load is in flight on a transport that streams progress events
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "cp-0");
    make_checkpoint(repo, "AUTH-002", "cp-1");
    make_checkpoint(repo, "AUTH-003", "cp-2");

    // @step When the transport emits CheckpointsProgress events (1/…), (47/…), then (150/150) with done=true before the list result folds
    let mut frames: Vec<CheckpointsProgress> = Vec::new();
    let list = collect_checkpoints_stream(repo, &mut |p| frames.push(p)).expect("stream");

    // @step Then the loading dialog counter row shows (1/…) while the total is still unknown
    assert_eq!(frames.len(), 4, "one frame per item + the done frame");
    assert_eq!(frames[0].loaded, 1, "first frame starts at loaded=1");
    assert!(
        !frames.iter().take(3).any(|f| f.done),
        "no per-item frame carries done=true"
    );

    // @step And the counter row climbs through the intermediate values as each progress event folds
    assert_eq!(frames[1].loaded, 2);
    assert_eq!(frames[2].loaded, 3);

    // @step And the counter row shows the final (150/150) just before the list appears
    assert!(frames[3].done, "final frame carries done=true");
    assert_eq!(frames[3].total, 3, "total = full enumeration count");
    assert_eq!(frames[3].loaded, 3);
    assert_eq!(list.len(), 3, "return value unchanged (backward-compatible)");
}

/// wire half — `FspecServiceImpl::list_checkpoints` publishes a frame on
/// the shared broadcast per collected item, then the done frame, while
/// the final Vec still returns through the same RPC.
#[tokio::test]
async fn rpc_list_checkpoints_emits_progress_frames_on_the_broadcast() {
    // @step Given a Checkpoints view whose list load is in flight on a transport that streams progress events
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "cp-0");
    make_checkpoint(repo, "AUTH-002", "cp-1");

    let service = service_for(repo);
    let mut rx = service.checkpoints_progress_rx();
    let impl_ = FspecServiceImpl::new(Arc::clone(&service));

    // @step When the transport emits CheckpointsProgress events (1/…), (47/…), then (150/150) with done=true before the list result folds
    let list = impl_.list_checkpoints(context::current()).await;

    // @step Then the loading dialog counter row shows (1/…) while the total is still unknown
    let mut frames = Vec::new();
    while let Ok(frame) = rx.try_recv() {
        frames.push(frame);
    }
    assert_eq!(frames.len(), 3, "one frame per item + the done frame");
    assert_eq!(frames[0].loaded, 1);
    assert!(!frames[0].done);

    // @step And the counter row climbs through the intermediate values as each progress event folds
    assert_eq!(frames[1].loaded, 2);

    // @step And the counter row shows the final (150/150) just before the list appears
    assert!(frames.last().expect("frame").done);
    assert_eq!(frames.last().expect("frame").total, 2);

    // @step When the CheckpointsLoaded fold arrives with 150 checkpoints
    // @step Then the list renders and the loading dialog is dismissed
    // (the final Vec still returns via the same RPC)
    assert_eq!(list.len(), 2, "RPC return value unchanged");
}

/// TUI fold half — the counter row renders `(loaded/…)` while the total
/// is unknown, climbs, and shows `(N/total)` just before the list folds.
#[tokio::test]
async fn tui_counter_advances_from_pending_total_to_final_before_list_folds() {
    // @step Given a Checkpoints view whose list load is in flight on a transport that streams progress events
    let (mut app, mock) = fresh_app();
    app.bootstrap().await.expect("bootstrap");
    app.dispatch(Action::OpenCheckpointsView);
    assert!(app.is_view_loading(), "list load in flight");

    // @step When the transport emits CheckpointsProgress events (1/…), (47/…), then (150/150) with done=true before the list result folds
    mock.push_checkpoints_progress(CheckpointsProgress {
        loaded: 1,
        total: 0,
        done: false,
    });
    let action = wait_for_action(&mut app, |a| matches!(a, Action::CheckpointsProgress(_)))
        .await
        .expect("CheckpointsProgress on the action bus");
    let (loaded, total) = match action {
        Action::CheckpointsProgress(p) => (p.loaded, p.total),
        _ => unreachable!(),
    };
    assert_eq!((loaded, total), (1, 0));

    // @step Then the loading dialog counter row shows (1/…) while the total is still unknown
    app.dispatch(Action::CheckpointsProgress(CheckpointsProgress {
        loaded: 1,
        total: 0,
        done: false,
    }));
    assert!(app.is_view_loading(), "still loading after a pending-total frame");
    let dialog = app.navigator_checkpoints_loading_dialog();
    assert!(
        dialog.progress.is_some(),
        "progress slot fed while the list stage is in flight"
    );
    let mut buf = Buffer::empty(Rect::new(0, 0, 60, 14));
    render_loading_dialog(Rect::new(0, 0, 60, 14), &mut buf, &dialog, 0);
    assert!(
        buf_text(&buf).contains("(1/…)"),
        "pending total renders as (1/…)"
    );

    // @step And the counter row climbs through the intermediate values as each progress event folds
    app.dispatch(Action::CheckpointsProgress(CheckpointsProgress {
        loaded: 47,
        total: 0,
        done: false,
    }));
    let mut buf = Buffer::empty(Rect::new(0, 0, 60, 14));
    render_loading_dialog(
        Rect::new(0, 0, 60, 14),
        &mut buf,
        &app.navigator_checkpoints_loading_dialog(),
        0,
    );
    assert!(buf_text(&buf).contains("(47/…)"), "counter climbed");

    // @step And the counter row shows the final (150/150) just before the list appears
    app.dispatch(Action::CheckpointsProgress(CheckpointsProgress {
        loaded: 150,
        total: 150,
        done: true,
    }));
    let mut buf = Buffer::empty(Rect::new(0, 0, 60, 14));
    render_loading_dialog(
        Rect::new(0, 0, 60, 14),
        &mut buf,
        &app.navigator_checkpoints_loading_dialog(),
        0,
    );
    assert!(
        buf_text(&buf).contains("(150/150)"),
        "final (N/total) before the list folds"
    );

    // @step When the CheckpointsLoaded fold arrives with 150 checkpoints
    let list: Vec<CheckpointInfo> = (0..150)
        .map(|i| checkpoint_info("AUTH-001", &format!("cp-{i:03}")))
        .collect();
    app.dispatch(Action::CheckpointsLoaded(list));
    // Settle the files stage (the cascade continues onto the first
    // selected checkpoint) so the dialog can dismiss.
    app.dispatch(Action::CheckpointFilesLoaded {
        work_unit_id: "AUTH-001".into(),
        name: "cp-000".into(),
        files: vec![],
    });

    // @step Then the list renders and the loading dialog is dismissed
    assert!(!app.is_view_loading(), "list flushed → idle");
    assert_eq!(app.navigator_checkpoints_len(), 150);
}

/// subscriber half — bootstrap spawns a subscriber on
/// `checkpoints_progress_rx()` that forwards frames onto the action bus.
#[tokio::test]
async fn tui_bootstrap_spawns_checkpoints_progress_subscriber_forwarding_to_the_bus() {
    // @step Given a Checkpoints view whose list load is in flight on a transport that streams progress events
    let (mut app, mock) = fresh_app();
    app.bootstrap().await.expect("bootstrap");
    // TUI-109 adds a 6th subscriber (checkpoints_progress_rx) alongside
    // the existing five (work_units / chunks / logs / status_changes /
    // session_created).
    assert_eq!(
        app.subscriber_task_count(),
        6,
        "bootstrap must spawn the checkpoints-progress subscriber task"
    );

    // @step When the transport emits CheckpointsProgress events (1/…), (47/…), then (150/150) with done=true before the list result folds
    mock.push_checkpoints_progress(CheckpointsProgress {
        loaded: 1,
        total: 0,
        done: false,
    });

    // @step Then the loading dialog counter row shows (1/…) while the total is still unknown
    let action = wait_for_action(&mut app, |a| matches!(a, Action::CheckpointsProgress(_)))
        .await
        .expect("CheckpointsProgress forwarded by the subscriber");
    assert!(matches!(
        action,
        Action::CheckpointsProgress(p) if p.loaded == 1 && p.total == 0 && !p.done
    ));

    // @step And the counter row climbs through the intermediate values as each progress event folds
    mock.push_checkpoints_progress(CheckpointsProgress {
        loaded: 47,
        total: 0,
        done: false,
    });
    let action = wait_for_action(&mut app, |a| matches!(a, Action::CheckpointsProgress(_)))
        .await
        .expect("second frame forwarded");
    assert!(matches!(
        action,
        Action::CheckpointsProgress(p) if p.loaded == 47
    ));

    // @step And the counter row shows the final (150/150) just before the list appears
    mock.push_checkpoints_progress(CheckpointsProgress {
        loaded: 150,
        total: 150,
        done: true,
    });
    let action = wait_for_action(&mut app, |a| matches!(a, Action::CheckpointsProgress(_)))
        .await
        .expect("done frame forwarded");
    assert!(matches!(
        action,
        Action::CheckpointsProgress(p) if p.loaded == 150 && p.total == 150 && p.done
    ));

    // @step When the CheckpointsLoaded fold arrives with 150 checkpoints
    // @step Then the list renders and the loading dialog is dismissed
    // (the fold itself is covered by the first TUI scenario; here the
    // subscriber must keep forwarding every frame it receives)
    assert!(app.active_view() == ViewMode::Board, "view untouched by the subscriber");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 2: capped enumeration shows (200/250)
// ─────────────────────────────────────────────────────────────────────────

/// rpc shaping half — loaded stops at the 200 cap while total reflects
/// the full enumeration.
#[test]
fn rpc_capped_enumeration_reports_loaded_at_cap_with_full_total() {
    // @step Given a repository with 250 checkpoints
    let tmp = setup_test_repo();
    let repo = tmp.path();
    for i in 0..250 {
        make_checkpoint(repo, "AUTH-001", &format!("cp-{i:03}"));
    }

    // @step When the streaming enumeration collects items with the 200-entry cap applied
    let mut frames: Vec<CheckpointsProgress> = Vec::new();
    let list = collect_checkpoints_stream(repo, &mut |p| frames.push(p)).expect("stream");

    // @step Then the progress counter reaches (200/250) - loaded stops at the cap while the total reflects the full enumeration
    let last = frames.last().expect("at least one frame");
    assert!(last.done, "final frame done");
    assert_eq!(last.loaded, 200, "loaded stops at the 200 cap");
    assert_eq!(last.total, 250, "total reflects the full enumeration");

    // @step And the returned list contains exactly 200 entries
    assert_eq!(list.len(), 200);
}

/// TUI fold half — the dialog renders whatever the wire carries.
#[test]
fn tui_capped_progress_frame_renders_200_of_250() {
    // @step Given a repository with 250 checkpoints
    // (fixture lives in the rpc half above; here the fold only renders
    // the wire frame it is given)
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::OpenCheckpointsView);

    // @step When the streaming enumeration collects items with the 200-entry cap applied
    app.dispatch(Action::CheckpointsProgress(CheckpointsProgress {
        loaded: 200,
        total: 250,
        done: true,
    }));

    // @step Then the progress counter reaches (200/250) - loaded stops at the cap while the total reflects the full enumeration
    let dialog = app.navigator_checkpoints_loading_dialog();
    let mut buf = Buffer::empty(Rect::new(0, 0, 60, 14));
    render_loading_dialog(Rect::new(0, 0, 60, 14), &mut buf, &dialog, 0);
    assert!(
        buf_text(&buf).contains("(200/250)"),
        "counter shows loaded=cap, total=full enumeration"
    );

    // @step And the returned list contains exactly 200 entries
    let list: Vec<CheckpointInfo> = (0..200)
        .map(|i| checkpoint_info("AUTH-001", &format!("cp-{i:03}")))
        .collect();
    app.dispatch(Action::CheckpointsLoaded(list));
    assert_eq!(app.navigator_checkpoints_len(), 200);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 3: late progress event after CheckpointsLoaded is stale-dropped
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn tui_late_done_event_after_checkpoints_loaded_is_stale_dropped() {
    // @step Given a Checkpoints view whose list has already flushed via the CheckpointsLoaded fold
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::OpenCheckpointsView);
    app.dispatch(Action::CheckpointsLoaded(vec![checkpoint_info(
        "AUTH-001",
        "baseline",
    )]));
    // The list stage has flushed (the cascade may continue onto the
    // files stage — that is the list presentation state).
    assert!(
        app.navigator_checkpoints_list_loaded(),
        "list stage flushed"
    );

    // @step When a late CheckpointsProgress event with done=true arrives after the fold
    app.dispatch(Action::CheckpointsProgress(CheckpointsProgress {
        loaded: 1,
        total: 1,
        done: true,
    }));

    // @step Then the view stays in the list presentation state and is not loading
    assert!(
        app.navigator_checkpoints_list_loaded(),
        "late event must not re-open the list load"
    );
    assert_eq!(app.navigator_checkpoints_len(), 1);

    // @step And the loading dialog is not re-painted - progress events after the list fold are ignored
    let dialog = app.navigator_checkpoints_loading_dialog();
    assert!(
        dialog.progress.is_none(),
        "progress slot must NOT be re-fed after the list fold"
    );
    let mut buf = Buffer::empty(Rect::new(0, 0, 60, 14));
    render_loading_dialog(Rect::new(0, 0, 60, 14), &mut buf, &dialog, 0);
    assert!(
        !buf_text(&buf).contains("/"),
        "no counter row painted for a stale frame"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 4: no-progress transport degrades to spinner + stage label
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn tui_no_progress_transport_degrades_to_spinner_and_stage_label() {
    // @step Given a Checkpoints view whose list load is in flight on a transport that never emits progress events
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::OpenCheckpointsView);
    assert!(app.is_view_loading(), "list load in flight");

    // @step When the loading dialog is painted while the list load is still in flight
    let dialog = app.navigator_checkpoints_loading_dialog();
    let mut buf = Buffer::empty(Rect::new(0, 0, 60, 14));
    render_loading_dialog(Rect::new(0, 0, 60, 14), &mut buf, &dialog, 0);
    let text = buf_text(&buf);

    // @step Then the dialog shows the spinner and the stage label "Loading checkpoint list…"
    assert!(text.contains("⠋"), "spinner glyph painted");
    assert!(
        text.contains("Loading checkpoint list…"),
        "stage label painted"
    );

    // @step And no counter row is painted - the TUI-107 behavior is preserved with no timeout or extra logic
    assert!(
        !text.contains("(/") && !text.contains("/…"),
        "no counter row without progress events"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 5: the non-streaming CLI path is byte-identical
// ─────────────────────────────────────────────────────────────────────────

/// git half — the non-streaming entry point delegates to the streaming
/// one with a no-op callback, so its output cannot drift.
#[test]
fn git_non_streaming_list_all_delegates_to_streaming_with_no_op_callback() {
    // @step Given the non-streaming collect_checkpoints delegates to the streaming variant with a no-op callback
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline");
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-a");
    make_checkpoint(repo, "AUTH-002", "pre-refactor");

    // @step When the CLI list-checkpoints command runs against a repository with checkpoints
    let baseline = list_all_ghost_checkpoints(repo).expect("non-streaming list");

    // @step Then the output is byte-identical to the pre-streaming behavior
    let mut ticks = 0usize;
    let streamed =
        list_all_ghost_checkpoints_stream(repo, &mut |_| {
            ticks += 1;
        })
        .expect("streaming list");

    // @step And the existing list_checkpoints tests pass unmodified
    assert_eq!(
        baseline, streamed,
        "non-streaming output must equal the streaming variant's return value"
    );
    assert_eq!(ticks, 3, "callback ticks once per checkpoint ref");
}

/// rpc half — the non-streaming `collect_checkpoints` delegates with a
/// no-op callback; its return value is identical to the streaming
/// variant's (the CLI `fspec list-checkpoints` output stays
/// byte-identical — the existing `codelet-fspec-core` tests in
/// `tests/list_checkpoints.rs` pass unmodified).
#[test]
fn rpc_non_streaming_collect_checkpoints_matches_streaming_return_value() {
    // @step Given the non-streaming collect_checkpoints delegates to the streaming variant with a no-op callback
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline");
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-a");
    make_checkpoint(repo, "AUTH-002", "pre-refactor");
    // Deterministic timestamps — without the index sidecar the fallback
    // timestamp is wall-clock and two back-to-back collections can land
    // on different milliseconds.
    write_index(
        repo,
        "AUTH-001",
        &[
            ("baseline", "2026-06-01T10:00:00.000Z"),
            ("AUTH-001-auto-a", "2026-06-02T10:00:00.000Z"),
        ],
    );
    write_index(repo, "AUTH-002", &[("pre-refactor", "2026-06-03T10:00:00.000Z")]);

    // @step When the CLI list-checkpoints command runs against a repository with checkpoints
    let baseline = collect_checkpoints(repo).expect("non-streaming collect");

    // @step Then the output is byte-identical to the pre-streaming behavior
    let mut ticks = 0usize;
    let streamed =
        collect_checkpoints_stream(repo, &mut |_| ticks += 1).expect("streaming collect");

    // @step And the existing list_checkpoints tests pass unmodified
    assert_eq!(
        baseline, streamed,
        "non-streaming output must equal the streaming variant's return value"
    );
    assert_eq!(ticks, 4, "callback ticks once per collected item + the done frame");
}
