//! WT-013: the worktree merge commit reverts main's commits to files the
//! session never touched.
//!
//! Feature: spec/features/merge-commit-keeps-main-changes-the-session-never-touched.feature
//!
//! Each scenario builds a fresh git repository in a temp dir, creates an
//! isolated session worktree from its HEAD, moves main's HEAD on (or
//! changes a mode in the session), merges through the real
//! `codelet_git::merge_session_with_strategy`, and checks the merge commit
//! with the `git` CLI. HOME points at a per-process temp dir so the
//! session manifest cleanup never touches the user's home.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use codelet_test_helpers::{OrFail, TestError, TestResult};

/// Point HOME at one temp dir for the whole test process (the merge
/// deletes `~/.fspec/git-sessions/<id>.json`). Every test calls this
/// before anything else, so HOME is set once before any test reads it.
fn isolated_home() -> Result<(), TestError> {
    static HOME: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    let home = HOME.get_or_init(|| {
        let dir = codelet_test_helpers::shared_temp_directory()?.join("home");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        std::env::set_var("HOME", &dir);
        Ok(dir)
    });
    home.as_ref().map(|_| ()).map_err(|e| e.clone().into())
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

/// A fresh repo with a repo-local identity (HOME is a temp dir, so no
/// global git config exists; gix needs an identity for the reflog).
fn init_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init", "-q"])?;
    git(dir.path(), &["config", "user.name", "Test User"])?;
    git(dir.path(), &["config", "user.email", "test@example.com"])?;
    Ok(dir)
}

/// A fresh repo holding `files` (path, content) in one commit.
fn repo_with(files: &[(&str, &str)]) -> Result<tempfile::TempDir, TestError> {
    let dir = init_repo()?;
    for (path, content) in files {
        write(dir.path(), path, content)?;
    }
    git(dir.path(), &["add", "-A"])?;
    git(dir.path(), &["commit", "-q", "-m", "initial commit"])?;
    Ok(dir)
}

fn write(root: &Path, path: &str, content: &str) -> Result<(), TestError> {
    let full = root.join(path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(full, content)?;
    Ok(())
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

fn merge(repo: &Path, session_id: &str) -> Result<String, TestError> {
    let result = codelet_git::merge_session_with_strategy(
        repo,
        session_id,
        &codelet_rpc_types::MergeStrategy::FastForward,
    )?;
    Ok(result
        .merge_commit
        .or_fail("the merge must create a commit")?)
}

fn status(repo: &Path) -> Result<String, TestError> {
    git(repo, &["status", "--porcelain"])
}

#[cfg(unix)]
fn set_executable(path: &Path, executable: bool) -> Result<(), TestError> {
    use std::os::unix::fs::PermissionsExt;
    let mode = if executable { 0o755 } else { 0o644 };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[test]
fn main_commits_to_files_the_session_never_touched_survive_the_merge() -> TestResult {
    isolated_home()?;
    // @step Given a temp git repo with hello.txt, notes.txt and spec/work-units.json committed, and an isolated session created from that commit
    let repo = repo_with(&[
        // fspec projects ignore the spec write lock that merges take.
        (".gitignore", "spec/.fspec-write.lock\n"),
        ("hello.txt", "hello\n"),
        ("notes.txt", "notes v1\n"),
        ("spec/work-units.json", "{\"version\": 1}\n"),
    ])?;
    let (session_id, worktree) = start_session(repo.path())?;

    // @step And main then commits a change to notes.txt and to spec/work-units.json
    write(repo.path(), "notes.txt", "notes v2 from main\n")?;
    write(repo.path(), "spec/work-units.json", "{\"version\": 2}\n")?;
    git(repo.path(), &["commit", "-q", "-am", "main moves on"])?;

    // @step And the session edits only hello.txt
    write(&worktree, "hello.txt", "hello from the session\n")?;

    // @step When the session is merged
    merge(repo.path(), &session_id)?;

    // @step Then HEAD:notes.txt and HEAD:spec/work-units.json hold main's committed versions
    assert_eq!(
        git(repo.path(), &["show", "HEAD:notes.txt"])?,
        "notes v2 from main\n",
        "the merge commit must keep main's notes.txt"
    );
    assert_eq!(
        git(repo.path(), &["show", "HEAD:spec/work-units.json"])?,
        "{\"version\": 2}\n",
        "the merge commit must keep main's board"
    );

    // @step And HEAD:hello.txt holds the session's version
    assert_eq!(
        git(repo.path(), &["show", "HEAD:hello.txt"])?,
        "hello from the session\n"
    );

    // @step And git status in main is clean
    assert_eq!(status(repo.path())?, "", "git status must be clean");
    Ok(())
}

#[test]
fn a_file_main_deleted_after_the_session_started_stays_deleted() -> TestResult {
    isolated_home()?;
    // @step Given a temp git repo with hello.txt and old.txt committed, and an isolated session created from that commit
    let repo = repo_with(&[("hello.txt", "hello\n"), ("old.txt", "old\n")])?;
    let (session_id, worktree) = start_session(repo.path())?;

    // @step And main then commits the deletion of old.txt
    git(repo.path(), &["rm", "-q", "old.txt"])?;
    git(repo.path(), &["commit", "-q", "-m", "main deletes old.txt"])?;

    // @step And the session edits only hello.txt
    write(&worktree, "hello.txt", "hello from the session\n")?;

    // @step When the session is merged
    merge(repo.path(), &session_id)?;

    // @step Then old.txt is absent from HEAD and from main's working tree
    assert_eq!(
        git(repo.path(), &["ls-tree", "--name-only", "HEAD", "old.txt"])?,
        "",
        "the merge commit must not bring old.txt back"
    );
    assert!(
        !repo.path().join("old.txt").exists(),
        "old.txt must stay deleted in main's working tree"
    );

    // @step And git status in main is clean
    assert_eq!(status(repo.path())?, "", "git status must be clean");
    Ok(())
}

#[cfg(unix)]
#[test]
fn clearing_the_executable_bit_in_the_session_lands_in_the_merge_commit() -> TestResult {
    isolated_home()?;
    // @step Given a temp git repo with an executable run.sh committed, and an isolated session created from that commit
    let dir = init_repo()?;
    write(dir.path(), "run.sh", "#!/bin/sh\necho hi\n")?;
    set_executable(&dir.path().join("run.sh"), true)?;
    git(dir.path(), &["add", "-A"])?;
    git(dir.path(), &["commit", "-q", "-m", "initial commit"])?;
    assert!(git(dir.path(), &["ls-tree", "HEAD", "run.sh"])?.starts_with("100755"));
    let (session_id, worktree) = start_session(dir.path())?;

    // @step And the session clears the executable bit of run.sh
    set_executable(&worktree.join("run.sh"), false)?;

    // @step When the session is merged
    merge(dir.path(), &session_id)?;

    // @step Then HEAD:run.sh has mode 100644
    let entry = git(dir.path(), &["ls-tree", "HEAD", "run.sh"])?;
    assert!(
        entry.starts_with("100644"),
        "the merge commit must carry the cleared executable bit, got {entry:?}"
    );

    // @step And git status in main is clean
    assert_eq!(status(dir.path())?, "", "git status must be clean");
    Ok(())
}
