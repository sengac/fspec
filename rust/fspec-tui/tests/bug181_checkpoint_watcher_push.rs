//! BUG-181 (SUPERSEDED by BUG-182) — the `GitStateWatcher` publishes live
//! checkpoint counts, and the backend exposes its push surface.
//!
//! Feature: spec/features/live-checkpoint-counts-watcher-push.feature
//!
//! Watcher-level scenarios (real temp repo, real watcher): manual and auto
//! checkpoint creation and checkpoint delete publish fresh counts, and a
//! non-repo directory degrades to an empty GitState. Plus the source-shape
//! scenario for the transport-agnostic git-state push surface. Split out of
//! bug181_live_checkpoint_counts.rs by BUG-355 (one test file per feature).

use codelet_test_helpers::{OrFail, TestResult};
use std::fs;
use std::time::Duration;

use codelet_core::git_state::GitStateWatcher;
use codelet_git::ghost_commit::delete_ghost_checkpoint;
use codelet_rpc_types::{CheckpointCounts, GitState};

mod common;
use common::checkpoint_repo::{make_checkpoint, setup_test_repo};

/// BUG-657: the watcher's first capture lands on the blocking pool, never
/// in its constructor; its frame is the initial snapshot.
async fn first_frame(
    watcher: &GitStateWatcher,
) -> Result<GitState, codelet_test_helpers::TestError> {
    let (backfill, mut rx) = watcher.subscribe_backfilled();
    match backfill {
        Some(state) => Ok(state),
        None => Ok(codelet_test_helpers::deadlock_guard("the first capture", rx.recv()).await??),
    }
}

/// Wait (bounded) for the watcher's broadcast channel to deliver a frame
/// whose `checkpoint_counts` equals `want`. Frames are full snapshots, so
/// the first match is the authoritative post-change state.
async fn wait_for_counts(
    watcher: &GitStateWatcher,
    want: CheckpointCounts,
) -> Result<CheckpointCounts, codelet_test_helpers::TestError> {
    let mut rx = watcher.subscribe();
    // The initial snapshot was broadcast before this receiver existed;
    // backfill so a no-change scenario still resolves.
    let mut got = watcher.snapshot().checkpoint_counts;
    if got == want {
        return Ok(got);
    }
    // TEST-042: wait for the frame under the deadlock guard, not a deadline.
    codelet_test_helpers::deadlock_guard("the watcher's checkpoint counts", async {
        loop {
            match rx.recv().await {
                Ok(GitState {
                    checkpoint_counts, ..
                }) => {
                    got = checkpoint_counts;
                    if got == want {
                        return Ok(got);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    return Err(format!(
                        "watcher channel closed while waiting for {want:?} (last {got:?})"
                    )
                    .into())
                }
            }
        }
    })
    .await?
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Manual checkpoint creation publishes incremented counts on
// the checkpoint-changed push
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn manual_checkpoint_creation_publishes_incremented_counts_on_the_checkpoint_changed_push(
) -> TestResult {
    // @step Given a GitStateWatcher watching a temp git repo that has no checkpoint refs
    let tmp = setup_test_repo()?;
    let repo = tmp.path();
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(250));
    assert_eq!(
        first_frame(&watcher).await?.checkpoint_counts,
        CheckpointCounts { manual: 0, auto: 0 },
        "initial snapshot must be zero counts"
    );

    // @step When a manual ghost-checkpoint ref AUTH-001/baseline is written into the repo (as fspec checkpoint AUTH-001 baseline does)
    make_checkpoint(repo, "AUTH-001", "baseline")?;

    // @step Then the watcher publishes a GitState frame with checkpoint_counts { manual: 1, auto: 0 } on its broadcast channel
    let counts = wait_for_counts(&watcher, CheckpointCounts { manual: 1, auto: 0 }).await?;
    assert_eq!(counts, CheckpointCounts { manual: 1, auto: 0 });
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Auto-checkpoint creation publishes incremented counts on the
// checkpoint-changed push
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn auto_checkpoint_creation_publishes_incremented_counts_on_the_checkpoint_changed_push(
) -> TestResult {
    // @step Given a GitStateWatcher watching a temp git repo that has one manual checkpoint (AUTH-001/baseline)
    let tmp = setup_test_repo()?;
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline")?;
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(250));
    assert_eq!(
        first_frame(&watcher).await?.checkpoint_counts,
        CheckpointCounts { manual: 1, auto: 0 },
        "initial snapshot must reflect the existing manual checkpoint"
    );

    // @step When an automatic checkpoint ref AUTH-001/AUTH-001-auto-testing is written into the repo (as update-work-unit-status does before the transition)
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-testing")?;

    // @step Then the watcher publishes a GitState frame with checkpoint_counts { manual: 1, auto: 1 } on its broadcast channel
    wait_for_counts(&watcher, CheckpointCounts { manual: 1, auto: 1 }).await?;
    assert_eq!(
        watcher.snapshot().checkpoint_counts,
        CheckpointCounts { manual: 1, auto: 1 }
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Checkpoint delete publishes decremented counts on the
// checkpoint-changed push
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn checkpoint_delete_publishes_decremented_counts_on_the_checkpoint_changed_push(
) -> TestResult {
    // @step Given a GitStateWatcher watching a temp git repo that has one manual + one auto checkpoint (AUTH-001/baseline, AUTH-001/AUTH-001-auto-testing)
    let tmp = setup_test_repo()?;
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline")?;
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-testing")?;
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(250));
    assert_eq!(
        first_frame(&watcher).await?.checkpoint_counts,
        CheckpointCounts { manual: 1, auto: 1 }
    );

    // @step When the manual checkpoint ref is deleted from the repo (as fspec cleanup-checkpoints / TUI delete do)
    delete_ghost_checkpoint(repo, "AUTH-001", "baseline").or_fail("delete")?;

    // @step Then the watcher publishes a GitState frame with checkpoint_counts { manual: 0, auto: 1 } on its broadcast channel
    wait_for_counts(&watcher, CheckpointCounts { manual: 0, auto: 1 }).await?;
    assert_eq!(
        watcher.snapshot().checkpoint_counts,
        CheckpointCounts { manual: 0, auto: 1 }
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Watcher degrades to an empty GitState when the watched
// directory is not a git repository
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn watcher_degrades_to_an_empty_git_state_when_the_watched_directory_is_not_a_git_repository(
) -> TestResult {
    // @step Given a GitStateWatcher watching a plain temp directory that has no .git
    let tmp = tempfile::TempDir::new().or_fail("tempdir")?;
    let dir = tmp.path();
    let watcher = GitStateWatcher::with_interval(dir, Duration::from_millis(250));

    // @step When the watcher's initial snapshot is read and a file is later created inside that directory
    let initial = first_frame(&watcher).await?;
    fs::write(dir.join("plain.txt"), "not a repo\n").or_fail("write file")?;

    // @step Then the initial snapshot is an empty GitState (zero checkpoint counts, empty changed_files and checkpoints lists)
    assert_eq!(
        initial.checkpoint_counts,
        CheckpointCounts { manual: 0, auto: 0 },
        "initial snapshot of a non-repo cwd must be zero counts with no error"
    );
    assert!(initial.changed_files.is_empty());
    assert!(initial.checkpoints.is_empty());
    assert_eq!(
        watcher.snapshot().checkpoint_counts,
        CheckpointCounts { manual: 0, auto: 0 }
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: FspecBackend exposes a transport-agnostic git-state push
// surface (source-shape)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn fspec_backend_exposes_a_transport_agnostic_git_state_push_surface() -> TestResult {
    // @step Given the rust/fspec-tui, rust/rpc, and rust/core source trees after BUG-182 lands
    let root = common::workspace_root()?;
    let transport_mod = codelet_test_helpers::source_composition::read_source(
        &root
            .join("fspec-tui")
            .join("src")
            .join("transport")
            .join("mod.rs"),
    )
    .or_fail("resolve composed transport source")?;
    let rpc_lib = codelet_test_helpers::source_composition::read_source(
        &root.join("rpc").join("src").join("lib.rs"),
    )
    .or_fail("resolve composed RPC source")?;
    let core_watcher =
        common::read_to_string_or_fail(&root.join("core").join("src").join("git_state.rs"))?;
    let bootstrap = common::read_to_string_or_fail(
        &root
            .join("fspec-tui")
            .join("src")
            .join("app")
            .join("bootstrap.rs"),
    )?;

    // @step When a developer scans the source trees
    // @step Then the FspecBackend trait in fspec-tui/src/transport/mod.rs contains the method git_state_changed_rx returning broadcast::Receiver<GitState>
    assert!(
        common::strip_rust_comments(&transport_mod)
            .contains("fn git_state_changed_rx(&self) -> broadcast::Receiver<GitState>"),
        "FspecBackend must declare the git-state push receiver"
    );

    // @step And SharedFspecService in rpc/src/lib.rs exposes git_state_changed_rx and git_state_snapshot
    assert!(
        rpc_lib.contains("pub fn git_state_changed_rx"),
        "SharedFspecService must expose git_state_changed_rx()"
    );
    assert!(
        rpc_lib.contains("pub fn git_state_snapshot"),
        "SharedFspecService must expose git_state_snapshot()"
    );

    // @step And the GitStateWatcher type exists in codelet-core and app/bootstrap.rs subscribes to git_state_changed_rx
    assert!(
        core_watcher.contains("pub struct GitStateWatcher"),
        "codelet-core must define the GitStateWatcher type"
    );
    assert!(
        bootstrap.contains("git_state_changed_rx"),
        "app/bootstrap.rs must subscribe to git_state_changed_rx"
    );
    Ok(())
}
