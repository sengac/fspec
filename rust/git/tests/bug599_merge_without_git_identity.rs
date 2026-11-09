//! BUG-599: a session merge needs no git identity, and its reflog names the
//! identity the merge commit was written with.
//!
//! Feature: spec/features/a-merge-commits-without-a-git-identity-and-git-tests-ignore-the-users-git-config.feature
//!
//! Advancing HEAD wrote the reflog with the committer from git config, so on
//! a machine with no git identity a merge failed with "The reflog could not
//! be created or updated" although the commit itself had an identity. This
//! test binary runs as such a machine: before any git call, the process's
//! HOME and XDG_CONFIG_HOME are empty temp directories and no system or
//! global git config is read. Every test first calls `no_git_identity()`,
//! whose `Once` finishes before any test reads the environment.

use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use codelet_test_helpers::{OrFail, TestError, TestResult};

/// The process-wide "no git identity" environment, set once.
fn no_git_identity() -> Result<(), TestError> {
    static HOME: OnceLock<Result<tempfile::TempDir, String>> = OnceLock::new();
    let home = HOME.get_or_init(|| {
        let home = tempfile::tempdir().map_err(|e| e.to_string())?;
        std::env::set_var("HOME", home.path());
        std::env::set_var("XDG_CONFIG_HOME", home.path().join("config"));
        std::env::set_var("GIT_CONFIG_GLOBAL", "/dev/null");
        std::env::set_var("GIT_CONFIG_NOSYSTEM", "1");
        Ok(home)
    });
    home.as_ref().map(|_| ()).map_err(|e| e.clone().into())
}

/// Run git in `cwd` (ignoring the user's and the system's git config) and
/// return its trimmed stdout; a failing git is a test error.
fn git(cwd: &Path, args: &[&str]) -> Result<String, TestError> {
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
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// A repository with one committed file whose commit identity is passed on
/// the command line, so the repository's own config holds no identity.
fn repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init"])?;
    std::fs::write(dir.path().join("hello.txt"), "hello\n")?;
    git(dir.path(), &["add", "."])?;
    git(
        dir.path(),
        &[
            "-c",
            "user.name=Base",
            "-c",
            "user.email=base@example.com",
            "commit",
            "-m",
            "base",
        ],
    )?;
    Ok(dir)
}

/// A session worktree of `repo` with a tracked change to hello.txt.
fn session_with_a_tracked_change(repo: &Path) -> Result<String, TestError> {
    let session = uuid::Uuid::new_v4().to_string();
    codelet_git::create_worktree(repo, &session)?;
    let worktree = repo.join(codelet_git::FSPEC_WORKTREES_DIR).join(&session);
    std::fs::write(worktree.join("hello.txt"), "hello (merged)\n")?;
    Ok(session)
}

fn merge(repo: &Path, session: &str) -> Result<String, TestError> {
    let outcome = codelet_git::merge_session_with_strategy(
        repo,
        session,
        &codelet_rpc_types::MergeStrategy::FastForward,
    )
    .map_err(|e| format!("the merge failed: {e:?}"))?;
    Ok(outcome.merge_commit.or_fail("the merge made a commit")?)
}

/// Scenario: A merge commits on a machine with no git identity
#[test]
fn a_merge_commits_on_a_machine_with_no_git_identity() -> TestResult {
    // @step Given a process whose HOME and XDG_CONFIG_HOME are empty and which reads no system git config
    no_git_identity()?;

    // @step And a session worktree with a tracked change and a repository with no user identity
    let repo = repo()?;
    assert!(
        git(repo.path(), &["config", "--get", "user.name"]).is_err(),
        "the repository must hold no identity"
    );
    let session = session_with_a_tracked_change(repo.path())?;

    // @step When the session is merged into main
    let commit = merge(repo.path(), &session)?;

    // @step Then the merge succeeds and main's HEAD is the merge commit
    assert_eq!(git(repo.path(), &["rev-parse", "HEAD"])?, commit);

    // @step And HEAD's newest reflog entry names the merge commit's committer
    let committer = git(repo.path(), &["log", "-1", "--format=%cn <%ce>", &commit])?;
    let reflog = git(
        repo.path(),
        &["log", "-g", "-1", "--format=%gn <%ge>", "HEAD"],
    )?;
    assert_eq!(reflog, committer);
    Ok(())
}

/// Scenario: The reflog names the repository's own identity
#[test]
fn the_reflog_names_the_repositorys_own_identity() -> TestResult {
    no_git_identity()?;
    // @step Given a repository whose .git/config sets user.name "Ada" and user.email "ada@example.com"
    let repo = repo()?;
    git(repo.path(), &["config", "user.name", "Ada"])?;
    git(repo.path(), &["config", "user.email", "ada@example.com"])?;

    // @step And a session worktree with a tracked change
    let session = session_with_a_tracked_change(repo.path())?;

    // @step When the session is merged into main
    let commit = merge(repo.path(), &session)?;

    // @step Then the merge commit's committer is Ada
    assert_eq!(
        git(repo.path(), &["log", "-1", "--format=%cn <%ce>", &commit])?,
        "Ada <ada@example.com>"
    );

    // @step And HEAD's newest reflog entry names Ada
    assert_eq!(
        git(
            repo.path(),
            &["log", "-g", "-1", "--format=%gn <%ge>", "HEAD"]
        )?,
        "Ada <ada@example.com>"
    );
    Ok(())
}
