//! TUI-109 — the git and rpc halves of per-item checkpoint-enumeration
//! progress.
//!
//! Feature: spec/features/checkpoint-enumeration-streams-per-item-progress-frames.feature
//!
//! git: `list_all_ghost_checkpoints_stream` ticks per ref and the
//! non-streaming entry point delegates to it (byte-identical CLI). rpc:
//! `collect_checkpoints_stream` shapes `{loaded, total, done}` frames
//! (pending total, cap 200/250), the non-streaming `collect_checkpoints`
//! delegates with a no-op callback, and `FspecServiceImpl::list_checkpoints`
//! publishes the frames on the shared broadcast. The first three tests are
//! the producer side of the TUI fold pinned in tui109_checkpoints_progress.rs
//! (supporting tests, comments name the step they feed). Split out by
//! BUG-355 (one test file per feature).

use codelet_test_helpers::{OrFail, TestResult};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use codelet_git::ghost_commit::{list_all_ghost_checkpoints, list_all_ghost_checkpoints_stream};
use codelet_rpc::checkpoints::{collect_checkpoints, collect_checkpoints_stream};
use codelet_rpc::{FspecService, FspecServiceImpl};
use codelet_rpc_types::CheckpointsProgress;
use tarpc::context;

mod common;
use common::checkpoint_repo::{make_checkpoint, service_for, setup_test_repo};

/// Write an index sidecar so timestamps are deterministic (mirrors
/// `checkpoint_transport_rpc362.rs::write_index`).
fn write_index(
    repo: &Path,
    work_unit_id: &str,
    entries: &[(&str, &str)],
) -> Result<(), codelet_test_helpers::TestError> {
    let dir = repo.join(".git").join("fspec-checkpoints-index");
    fs::create_dir_all(&dir).or_fail("mkdir index dir")?;
    let checkpoints: Vec<serde_json::Value> = entries
        .iter()
        .map(|(name, ts)| serde_json::json!({ "name": name, "timestamp": ts }))
        .collect();
    let body = serde_json::json!({ "checkpoints": checkpoints });
    let text = serde_json::to_string_pretty(&body).or_fail("serialize index")?;
    fs::write(dir.join(format!("{work_unit_id}.json")), text).or_fail("write index")?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Producer side of the TUI-109 counter scenario (supporting tests)
// ─────────────────────────────────────────────────────────────────────────

/// git half — the streaming variant ticks the callback per checkpoint
/// ref, so the rpc layer can shape a per-item progress frame.
#[test]
fn git_streaming_variant_ticks_the_callback_per_checkpoint_ref() -> TestResult {
    // (feeds TUI step) Given a Checkpoints view whose list load is in flight on a transport that streams progress events
    let tmp = setup_test_repo()?;
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "cp-0")?;
    make_checkpoint(repo, "AUTH-002", "cp-1")?;
    make_checkpoint(repo, "AUTH-003", "cp-2")?;

    // (feeds TUI step) When the transport emits CheckpointsProgress events (1/…), (47/…), then (150/150) with done=true before the list result folds
    let mut seen: Vec<(String, String)> = Vec::new();
    let out = list_all_ghost_checkpoints_stream(repo, &mut |pair| {
        seen.push(pair.clone());
    })
    .or_fail("streaming list")?;

    // (feeds TUI step) Then the loading dialog counter row shows (1/…) while the total is still unknown
    // (the callback fires per item — 3 ticks for 3 refs)
    assert_eq!(seen.len(), 3, "one tick per checkpoint ref");
    // (feeds TUI step) And the counter row climbs through the intermediate values as each progress event folds
    let mut seen_sorted = seen;
    seen_sorted.sort();
    let mut all_sorted: Vec<(String, String)> = out.to_vec();
    all_sorted.sort();
    assert_eq!(
        seen_sorted, all_sorted,
        "ticks cover exactly the returned pairs"
    );
    // (feeds TUI step) And the counter row shows the final (150/150) just before the list appears
    assert_eq!(out.len(), 3, "return value unchanged (backward-compatible)");
    // (feeds TUI step) When the CheckpointsLoaded fold arrives with 150 checkpoints
    // (the fold half is exercised by the TUI tests below; the git layer
    // only guarantees the per-item ticks the fold consumes)
    // (feeds TUI step) Then the list renders and the loading dialog is dismissed
    // (same note — the git layer's contract is the tick stream)
    Ok(())
}

/// rpc shaping half — per-item ticks shaped into `{loaded, total, done}`
/// frames (pending total until the done frame).
#[test]
fn rpc_collect_checkpoints_stream_ticks_per_item_with_pending_total_then_done() -> TestResult {
    // (feeds TUI step) Given a Checkpoints view whose list load is in flight on a transport that streams progress events
    let tmp = setup_test_repo()?;
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "cp-0")?;
    make_checkpoint(repo, "AUTH-002", "cp-1")?;
    make_checkpoint(repo, "AUTH-003", "cp-2")?;

    // (feeds TUI step) When the transport emits CheckpointsProgress events (1/…), (47/…), then (150/150) with done=true before the list result folds
    let mut frames: Vec<CheckpointsProgress> = Vec::new();
    let list = collect_checkpoints_stream(repo, &mut |p| frames.push(p)).or_fail("stream")?;

    // (feeds TUI step) Then the loading dialog counter row shows (1/…) while the total is still unknown
    assert_eq!(frames.len(), 4, "one frame per item + the done frame");
    assert_eq!(frames[0].loaded, 1, "first frame starts at loaded=1");
    assert!(
        !frames.iter().take(3).any(|f| f.done),
        "no per-item frame carries done=true"
    );

    // (feeds TUI step) And the counter row climbs through the intermediate values as each progress event folds
    assert_eq!(frames[1].loaded, 2);
    assert_eq!(frames[2].loaded, 3);

    // (feeds TUI step) And the counter row shows the final (150/150) just before the list appears
    assert!(frames[3].done, "final frame carries done=true");
    assert_eq!(frames[3].total, 3, "total = full enumeration count");
    assert_eq!(frames[3].loaded, 3);
    assert_eq!(
        list.len(),
        3,
        "return value unchanged (backward-compatible)"
    );
    Ok(())
}

/// wire half — `FspecServiceImpl::list_checkpoints` publishes a frame on
/// the shared broadcast per collected item, then the done frame, while
/// the final Vec still returns through the same RPC.
#[tokio::test]
async fn rpc_list_checkpoints_emits_progress_frames_on_the_broadcast() -> TestResult {
    // (feeds TUI step) Given a Checkpoints view whose list load is in flight on a transport that streams progress events
    let tmp = setup_test_repo()?;
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "cp-0")?;
    make_checkpoint(repo, "AUTH-002", "cp-1")?;

    let service = service_for(repo)?;
    let mut rx = service.checkpoints_progress_rx();
    let impl_ = FspecServiceImpl::new(Arc::clone(&service));

    // (feeds TUI step) When the transport emits CheckpointsProgress events (1/…), (47/…), then (150/150) with done=true before the list result folds
    let list = impl_
        .list_checkpoints(context::current())
        .await
        .or_fail("the checkpoints are listed")?;

    // (feeds TUI step) Then the loading dialog counter row shows (1/…) while the total is still unknown
    let mut frames = Vec::new();
    while let Ok(frame) = rx.try_recv() {
        frames.push(frame);
    }
    assert_eq!(frames.len(), 3, "one frame per item + the done frame");
    assert_eq!(frames[0].loaded, 1);
    assert!(!frames[0].done);

    // (feeds TUI step) And the counter row climbs through the intermediate values as each progress event folds
    assert_eq!(frames[1].loaded, 2);

    // (feeds TUI step) And the counter row shows the final (150/150) just before the list appears
    assert!(frames.last().or_fail("frame")?.done);
    assert_eq!(frames.last().or_fail("frame")?.total, 2);

    // (feeds TUI step) When the CheckpointsLoaded fold arrives with 150 checkpoints
    // (feeds TUI step) Then the list renders and the loading dialog is dismissed
    // (the final Vec still returns via the same RPC)
    assert_eq!(list.len(), 2, "RPC return value unchanged");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Capped enumeration shows (200/250) - truncation does not hide progress
// ─────────────────────────────────────────────────────────────────────────

/// rpc shaping half — loaded stops at the 200 cap while total reflects
/// the full enumeration.
#[test]
fn rpc_capped_enumeration_reports_loaded_at_cap_with_full_total() -> TestResult {
    // @step Given a repository with 250 checkpoints
    let tmp = setup_test_repo()?;
    let repo = tmp.path();
    for i in 0..250 {
        make_checkpoint(repo, "AUTH-001", &format!("cp-{i:03}"))?;
    }

    // @step When the streaming enumeration collects items with the 200-entry cap applied
    let mut frames: Vec<CheckpointsProgress> = Vec::new();
    let list = collect_checkpoints_stream(repo, &mut |p| frames.push(p)).or_fail("stream")?;

    // @step Then the progress counter reaches (200/250) - loaded stops at the cap while the total reflects the full enumeration
    let last = frames.last().or_fail("at least one frame")?;
    assert!(last.done, "final frame done");
    assert_eq!(last.loaded, 200, "loaded stops at the 200 cap");
    assert_eq!(last.total, 250, "total reflects the full enumeration");

    // @step And the returned list contains exactly 200 entries
    assert_eq!(list.len(), 200);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The non-streaming CLI path is byte-identical
// ─────────────────────────────────────────────────────────────────────────

/// git half — the non-streaming entry point delegates to the streaming
/// one with a no-op callback, so its output cannot drift.
#[test]
fn git_non_streaming_list_all_delegates_to_streaming_with_no_op_callback() -> TestResult {
    // @step Given the non-streaming collect_checkpoints delegates to the streaming variant with a no-op callback
    let tmp = setup_test_repo()?;
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline")?;
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-a")?;
    make_checkpoint(repo, "AUTH-002", "pre-refactor")?;

    // @step When the CLI list-checkpoints command runs against a repository with checkpoints
    let baseline = list_all_ghost_checkpoints(repo).or_fail("non-streaming list")?;

    // @step Then the output is byte-identical to the pre-streaming behavior
    let mut ticks = 0usize;
    let streamed = list_all_ghost_checkpoints_stream(repo, &mut |_| {
        ticks += 1;
    })
    .or_fail("streaming list")?;

    // @step And the existing list_checkpoints tests pass unmodified
    assert_eq!(
        baseline, streamed,
        "non-streaming output must equal the streaming variant's return value"
    );
    assert_eq!(ticks, 3, "callback ticks once per checkpoint ref");
    Ok(())
}

/// rpc half — the non-streaming `collect_checkpoints` delegates with a
/// no-op callback; its return value is identical to the streaming
/// variant's (the CLI `fspec list-checkpoints` output stays
/// byte-identical — the existing `codelet-fspec-core` tests in
/// `tests/list_checkpoints.rs` pass unmodified).
#[test]
fn rpc_non_streaming_collect_checkpoints_matches_streaming_return_value() -> TestResult {
    // @step Given the non-streaming collect_checkpoints delegates to the streaming variant with a no-op callback
    let tmp = setup_test_repo()?;
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline")?;
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-a")?;
    make_checkpoint(repo, "AUTH-002", "pre-refactor")?;
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
    )?;
    write_index(
        repo,
        "AUTH-002",
        &[("pre-refactor", "2026-06-03T10:00:00.000Z")],
    )?;

    // @step When the CLI list-checkpoints command runs against a repository with checkpoints
    let baseline = collect_checkpoints(repo).or_fail("non-streaming collect")?;

    // @step Then the output is byte-identical to the pre-streaming behavior
    let mut ticks = 0usize;
    let streamed =
        collect_checkpoints_stream(repo, &mut |_| ticks += 1).or_fail("streaming collect")?;

    // @step And the existing list_checkpoints tests pass unmodified
    assert_eq!(
        baseline, streamed,
        "non-streaming output must equal the streaming variant's return value"
    );
    assert_eq!(
        ticks, 4,
        "callback ticks once per collected item + the done frame"
    );
    Ok(())
}
