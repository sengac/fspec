//! WT-014: applying a worktree merge writes spec files without the spec
//! lock or an atomic replace.
//!
//! Feature: spec/features/merge-apply-writes-spec-files-under-the-spec-lock-and-atomically.feature
//!
//! Each scenario builds a fresh git repository in a temp dir with a
//! committed `spec/`, creates an isolated session worktree from it and
//! merges through the real `codelet_git::merge_session_with_strategy`.
//! "Another writer" is another thread taking the same cross-process
//! `codelet_common::spec_lock::SpecLock` every fspec write takes. The lock
//! timeout is 0 ms for the whole process, so a held lock fails at once
//! and nothing waits on a clock. HOME points at a per-process temp dir so
//! the session manifest cleanup never touches the user's home.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{mpsc, OnceLock};

use codelet_common::spec_lock::{SpecLock, SPEC_LOCK_FILE_NAME, SPEC_LOCK_TIMEOUT_ENV};
use codelet_test_helpers::{deadlock_guard_blocking, OrFailErr, TestError, TestResult};

/// Process-wide test environment, set once before any test reads it:
/// HOME in a temp dir and a 0 ms spec lock timeout.
fn test_environment() -> Result<(), TestError> {
    static ENV: OnceLock<Result<(), String>> = OnceLock::new();
    let env = ENV.get_or_init(|| {
        let home = codelet_test_helpers::shared_temp_directory()?.join("home");
        std::fs::create_dir_all(&home).map_err(|e| e.to_string())?;
        std::env::set_var("HOME", &home);
        std::env::set_var(SPEC_LOCK_TIMEOUT_ENV, "0");
        Ok(())
    });
    env.clone().map_err(Into::into)
}

/// Run `git` in `cwd`; fail the test on a non-zero exit.
fn git(cwd: &Path, args: &[&str]) -> Result<String, TestError> {
    let out = Command::new("git")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(args)
        .current_dir(cwd)
        .output()?;
    if !out.status.success() {
        return Err(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )
        .into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn write(root: &Path, path: &str, content: &str) -> Result<(), TestError> {
    let full = root.join(path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(full, content)?;
    Ok(())
}

/// A fresh repo (repo-local identity, the spec lock ignored as fspec
/// projects do) holding `files` in one commit.
fn repo_with(files: &[(&str, &str)]) -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init", "-q"])?;
    git(dir.path(), &["config", "user.name", "Test User"])?;
    git(dir.path(), &["config", "user.email", "test@example.com"])?;
    write(dir.path(), ".gitignore", "spec/.fspec-write.lock\n")?;
    for (path, content) in files {
        write(dir.path(), path, content)?;
    }
    git(dir.path(), &["add", "-A"])?;
    git(dir.path(), &["commit", "-q", "-m", "initial commit"])?;
    Ok(dir)
}

/// Create an isolated session worktree from main's HEAD; returns
/// (session id, worktree path).
fn start_session(repo: &Path) -> Result<(String, PathBuf), TestError> {
    let session_id = uuid::Uuid::new_v4().to_string();
    codelet_git::create_worktree(repo, &session_id)?;
    let worktree = repo
        .join(codelet_git::FSPEC_WORKTREES_DIR)
        .join(&session_id);
    Ok((session_id, worktree))
}

fn merge(repo: &Path, session_id: &str) -> codelet_git::Result<codelet_git::MergeResult> {
    codelet_git::merge_session_with_strategy(
        repo,
        session_id,
        &codelet_rpc_types::MergeStrategy::FastForward,
    )
}

/// Another writer (its own thread) takes main's spec lock; with the 0 ms
/// timeout this succeeds only when nobody holds it.
fn another_writer_acquires(spec_dir: &Path) -> Result<(), TestError> {
    let dir = spec_dir.to_path_buf();
    deadlock_guard_blocking("another writer taking the spec lock", move || {
        SpecLock::acquire(&dir).map(drop).map_err(|e| e.to_string())
    })??;
    Ok(())
}

#[cfg(unix)]
fn inode(path: &Path) -> Result<u64, TestError> {
    use std::os::unix::fs::MetadataExt;
    Ok(std::fs::metadata(path)?.ino())
}

#[test]
fn a_spec_lock_held_past_the_timeout_fails_the_merge_before_anything_is_applied() -> TestResult {
    test_environment()?;
    // @step Given a temp git repo with spec/features/merge.feature committed, and an isolated session that changed spec/features/merge.feature
    let repo = repo_with(&[("spec/features/merge.feature", "{\"version\": 1}\n")])?;
    let (session_id, worktree) = start_session(repo.path())?;
    write(
        &worktree,
        "spec/features/merge.feature",
        "{\"version\": \"session\"}\n",
    )?;
    let head_before = git(repo.path(), &["rev-parse", "HEAD"])?;

    // @step And another writer holds main's spec lock and the lock timeout is 0 ms
    let spec_dir = repo.path().join("spec");
    let (acquired_tx, acquired_rx) = mpsc::channel::<Result<(), String>>();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let holder_dir = spec_dir.clone();
    let holder = std::thread::spawn(move || match SpecLock::acquire(&holder_dir) {
        Ok(guard) => {
            let _ = acquired_tx.send(Ok(()));
            let _ = release_rx.recv();
            drop(guard);
        }
        Err(e) => {
            let _ = acquired_tx.send(Err(e.to_string()));
        }
    });
    deadlock_guard_blocking("the holder taking the spec lock", move || {
        acquired_rx.recv().map_err(|e| e.to_string())
    })???;

    // @step When the session is merged
    let result = merge(repo.path(), &session_id);
    let _ = release_tx.send(());
    holder
        .join()
        .map_err(|_| "the lock holder thread panicked")?;

    // @step Then the merge fails with the standard lock message naming main's spec/.fspec-write.lock
    let error = result
        .or_fail_err("the merge must fail while the lock is held")?
        .to_string();
    let lock_file = spec_dir.canonicalize()?.join(SPEC_LOCK_FILE_NAME);
    assert!(
        error.contains(&format!(
            "Failed to acquire file lock on {}",
            lock_file.display()
        )) && error.contains("Check for stuck fspec processes."),
        "expected the standard lock message for {}, got {error:?}",
        lock_file.display()
    );

    // @step And main's spec/features/merge.feature is unchanged and nothing is committed
    assert_eq!(
        std::fs::read_to_string(spec_dir.join("features").join("merge.feature"))?,
        "{\"version\": 1}\n"
    );
    assert_eq!(git(repo.path(), &["rev-parse", "HEAD"])?, head_before);

    // @step And the session worktree still exists
    assert!(worktree.exists(), "the worktree must stay for a retry");
    Ok(())
}

#[cfg(unix)]
#[test]
fn merged_files_replace_main_files_atomically_and_the_lock_is_released() -> TestResult {
    test_environment()?;
    // @step Given a temp git repo with spec/features/merge.feature and hello.txt committed, and an isolated session that changed both
    let repo = repo_with(&[
        ("spec/features/merge.feature", "{\"version\": 1}\n"),
        ("hello.txt", "hello\n"),
    ])?;
    let (session_id, worktree) = start_session(repo.path())?;
    write(
        &worktree,
        "spec/features/merge.feature",
        "{\"version\": \"session\"}\n",
    )?;
    write(&worktree, "hello.txt", "hello from the session\n")?;
    let board = repo.path().join("spec/features/merge.feature");
    let hello = repo.path().join("hello.txt");
    let (board_inode, hello_inode) = (inode(&board)?, inode(&hello)?);

    // @step When the session is merged
    merge(repo.path(), &session_id)?;

    // @step Then main's spec/features/merge.feature and hello.txt are new files holding the session's content
    assert_ne!(
        inode(&board)?,
        board_inode,
        "the board must be replaced by rename, not rewritten in place"
    );
    assert_ne!(
        inode(&hello)?,
        hello_inode,
        "hello.txt must be replaced by rename, not rewritten in place"
    );
    assert_eq!(
        std::fs::read_to_string(&board)?,
        "{\"version\": \"session\"}\n"
    );
    assert_eq!(std::fs::read_to_string(&hello)?, "hello from the session\n");

    // @step And no temporary files are left beside them
    for dir in [repo.path().to_path_buf(), repo.path().join("spec")] {
        for entry in std::fs::read_dir(&dir)? {
            let name = entry?.file_name().to_string_lossy().to_string();
            assert!(
                !name.contains(".tmp."),
                "temp file {name} left in {}",
                dir.display()
            );
        }
    }

    // @step And another writer acquires main's spec lock at once
    another_writer_acquires(&repo.path().join("spec"))?;
    Ok(())
}

#[test]
fn a_merge_that_stops_on_a_conflict_releases_the_lock() -> TestResult {
    test_environment()?;
    // @step Given a temp git repo with spec/features/merge.feature committed, and an isolated session that changed its first line
    let repo = repo_with(&[("spec/features/merge.feature", "base line\nsecond line\n")])?;
    let (session_id, worktree) = start_session(repo.path())?;
    write(
        &worktree,
        "spec/features/merge.feature",
        "session line\nsecond line\n",
    )?;

    // @step And main changed the same line in its working tree
    write(
        repo.path(),
        "spec/features/merge.feature",
        "main line\nsecond line\n",
    )?;

    // @step When the session is merged
    let result = merge(repo.path(), &session_id);

    // @step Then the merge reports a conflict in spec/features/merge.feature
    match result {
        Err(codelet_git::GitError::ConflictError { files }) => {
            assert_eq!(files, vec!["spec/features/merge.feature".to_string()]);
        }
        other => return Err(format!("expected a conflict, got {other:?}").into()),
    }

    // @step And another writer acquires main's spec lock at once
    another_writer_acquires(&repo.path().join("spec"))?;
    Ok(())
}
