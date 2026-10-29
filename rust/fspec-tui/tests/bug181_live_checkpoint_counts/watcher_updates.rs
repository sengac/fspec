/// Wait (bounded) for the watcher's broadcast channel to deliver a frame
/// whose `checkpoint_counts` equals `want`. Frames are full snapshots, so
/// the first match is the authoritative post-change state.
async fn wait_for_counts(watcher: &GitStateWatcher, want: CheckpointCounts) -> CheckpointCounts {
    let mut rx = watcher.subscribe();
    // The initial snapshot was broadcast before this receiver existed;
    // backfill so a no-change scenario still resolves.
    let mut got = watcher.snapshot().checkpoint_counts;
    if got == want {
        return got;
    }
    let deadline = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(deadline);
    loop {
        let res = tokio::select! {
            _ = &mut deadline => panic!("timeout: watcher did not publish {want:?} (last {got:?})"),
            r = rx.recv() => r,
        };
        match res {
            Ok(GitState {
                checkpoint_counts, ..
            }) => {
                got = checkpoint_counts;
                if got == want {
                    return got;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                panic!("watcher channel closed while waiting for {want:?} (last {got:?})")
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Manual checkpoint creation publishes incremented counts on
// the checkpoint-changed push
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn manual_checkpoint_creation_publishes_incremented_counts_on_the_checkpoint_changed_push() {
    // @step Given a GitStateWatcher watching a temp git repo that has no checkpoint refs
    let tmp = setup_test_repo();
    let repo = tmp.path();
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(250));
    assert_eq!(
        watcher.snapshot().checkpoint_counts,
        CheckpointCounts { manual: 0, auto: 0 },
        "initial snapshot must be zero counts"
    );

    // @step When a manual ghost-checkpoint ref AUTH-001/baseline is written into the repo (as fspec checkpoint AUTH-001 baseline does)
    make_checkpoint(repo, "AUTH-001", "baseline");

    // @step Then the watcher publishes a GitState frame with checkpoint_counts { manual: 1, auto: 0 } on its broadcast channel
    let counts = wait_for_counts(&watcher, CheckpointCounts { manual: 1, auto: 0 }).await;
    assert_eq!(counts, CheckpointCounts { manual: 1, auto: 0 });
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Auto-checkpoint creation publishes incremented counts on the
// checkpoint-changed push
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn auto_checkpoint_creation_publishes_incremented_counts_on_the_checkpoint_changed_push() {
    // @step Given a GitStateWatcher watching a temp git repo that has one manual checkpoint (AUTH-001/baseline)
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline");
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(250));
    assert_eq!(
        watcher.snapshot().checkpoint_counts,
        CheckpointCounts { manual: 1, auto: 0 },
        "initial snapshot must reflect the existing manual checkpoint"
    );

    // @step When an automatic checkpoint ref AUTH-001/AUTH-001-auto-testing is written into the repo (as update-work-unit-status does before the transition)
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-testing");

    // @step Then the watcher publishes a GitState frame with checkpoint_counts { manual: 1, auto: 1 } on its broadcast channel
    wait_for_counts(&watcher, CheckpointCounts { manual: 1, auto: 1 }).await;
    assert_eq!(
        watcher.snapshot().checkpoint_counts,
        CheckpointCounts { manual: 1, auto: 1 }
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Checkpoint delete publishes decremented counts on the
// checkpoint-changed push
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn checkpoint_delete_publishes_decremented_counts_on_the_checkpoint_changed_push() {
    // @step Given a GitStateWatcher watching a temp git repo that has one manual + one auto checkpoint (AUTH-001/baseline, AUTH-001/AUTH-001-auto-testing)
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline");
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-testing");
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(250));
    assert_eq!(
        watcher.snapshot().checkpoint_counts,
        CheckpointCounts { manual: 1, auto: 1 }
    );

    // @step When the manual checkpoint ref is deleted from the repo (as fspec cleanup-checkpoints / TUI delete do)
    delete_ghost_checkpoint(repo, "AUTH-001", "baseline").expect("delete");

    // @step Then the watcher publishes a GitState frame with checkpoint_counts { manual: 0, auto: 1 } on its broadcast channel
    wait_for_counts(&watcher, CheckpointCounts { manual: 0, auto: 1 }).await;
    assert_eq!(
        watcher.snapshot().checkpoint_counts,
        CheckpointCounts { manual: 0, auto: 1 }
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Watcher degrades to an empty GitState when the watched
// directory is not a git repository
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn watcher_degrades_to_an_empty_git_state_when_the_watched_directory_is_not_a_git_repository()
{
    // @step Given a GitStateWatcher watching a plain temp directory that has no .git
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let dir = tmp.path();
    let watcher = GitStateWatcher::with_interval(dir, Duration::from_millis(250));

    // @step When the watcher's initial snapshot is read and a file is later created inside that directory
    let initial = watcher.snapshot();
    fs::write(dir.join("plain.txt"), "not a repo\n").expect("write file");

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
}
