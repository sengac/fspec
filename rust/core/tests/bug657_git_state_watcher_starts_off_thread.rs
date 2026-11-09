//! BUG-657: the git-state watcher captures its first snapshot off the
//! constructing thread.
//!
//! Feature: spec/features/git-captures-read-each-tree-once-and-the-tui-draws-before-git-state.feature
//!
//! The watcher's constructor used to run a full capture inline, and the
//! service builds it before the TUI draws anything, so a slow capture kept
//! the TUI blank. The capture hook holds the first capture back (a channel,
//! never a clock), and tokio's paused clock is moved by hand, so "before
//! the first poll interval elapses" is exact.

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use codelet_core::git_state::GitStateWatcher;
use codelet_rpc_types::{ChangedFile, GitState};
use codelet_test_helpers::{deadlock_guard, wait_until, TestError, TestResult};

const POLL_INTERVAL: Duration = Duration::from_secs(60);

/// Run `git` in `cwd` without the user's global or system config.
fn git(cwd: &Path, args: &[&str]) -> Result<(), TestError> {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()?;
    if !out.status.success() {
        return Err(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )
        .into());
    }
    Ok(())
}

fn repo_with_a_staged_new_file() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::tempdir()?;
    let root = dir.path();
    git(root, &["init", "-q", "-b", "main"])?;
    git(root, &["config", "user.name", "Test User"])?;
    git(root, &["config", "user.email", "test@example.com"])?;
    std::fs::write(root.join("README.md"), "# fixture\n")?;
    git(root, &["add", "README.md"])?;
    git(root, &["commit", "-q", "-m", "initial"])?;
    std::fs::write(root.join("new.txt"), "new\n")?;
    git(root, &["add", "new.txt"])?;
    Ok(dir)
}

#[tokio::test(start_paused = true)]
async fn the_git_state_watcher_captures_its_first_snapshot_off_the_constructing_thread(
) -> TestResult {
    // @step Given a git repository with a staged new file
    let dir = repo_with_a_staged_new_file()?;

    // @step And a GitStateWatcher whose capture hook holds every capture back
    let captures = Arc::new(AtomicUsize::new(0));
    let (release, held) = mpsc::channel::<()>();
    let held = Mutex::new(held);
    let hook_captures = Arc::clone(&captures);
    let hook = move || {
        hook_captures.fetch_add(1, Ordering::SeqCst);
        // Held until the test releases it; once the sender is dropped,
        // later captures pass straight through.
        if let Ok(held) = held.lock() {
            let _ = held.recv();
        }
    };

    // @step When the watcher is constructed
    let watcher = GitStateWatcher::with_captures(dir.path(), POLL_INTERVAL, hook);

    // @step Then the constructor returns while the first capture is held back
    let counted = Arc::clone(&captures);
    wait_until("the first capture to reach the hook", move || {
        let counted = Arc::clone(&counted);
        async move { counted.load(Ordering::SeqCst) == 1 }
    })
    .await?;

    // @step And snapshot() is the empty GitState
    assert_eq!(watcher.snapshot(), GitState::default());
    let mut frames = watcher.subscribe();

    // @step When the held-back capture is released
    release.send(())?;
    drop(release);

    // @step Then the first frame is broadcast with the branch and the staged new file
    // The paused clock only moves when the runtime is idle; the guarded
    // recv keeps it busy polling, so no poll interval passes meanwhile.
    let frame = deadlock_guard("the first git-state frame", async {
        loop {
            match frames.try_recv() {
                Ok(frame) => return Ok::<GitState, TestError>(frame),
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                    codelet_test_helpers::yield_now().await
                }
                Err(other) => return Err(other.into()),
            }
        }
    })
    .await??;
    assert_eq!(frame.git_branch.as_deref(), Some("main"));
    assert_eq!(
        frame.changed_files,
        vec![ChangedFile {
            path: "new.txt".to_string(),
            change_type: "A".to_string(),
            staged: true,
        }]
    );
    assert_eq!(watcher.snapshot(), frame);

    // @step And exactly one capture has run before the first poll interval elapses
    tokio::time::advance(POLL_INTERVAL - Duration::from_millis(1)).await;
    for _ in 0..64 {
        tokio::task::yield_now().await;
    }
    assert_eq!(
        captures.load(Ordering::SeqCst),
        1,
        "no capture before the first poll"
    );
    tokio::time::advance(Duration::from_millis(1)).await;
    let counted = Arc::clone(&captures);
    wait_until("the first poll's capture", move || {
        let counted = Arc::clone(&counted);
        async move { counted.load(Ordering::SeqCst) == 2 }
    })
    .await?;
    Ok(())
}
