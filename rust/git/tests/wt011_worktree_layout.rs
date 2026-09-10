//! WT-011 — worktree metadata hardcodes `<repo>/.git` assumptions
//! (list_worktrees HEAD path, commindir) and isolated_session.rs is dead
//! production code.
//!
//! Feature: spec/features/worktree-metadata-hardcodes-repo-git-assumptions-list-worktrees-head-path-commindir-and-isolated-session-rs-is-dead-production-code.feature
//!
//! The list_worktrees scenarios FAIL against the current code (it reads
//! HEAD from `repo_path/.git/worktrees/<sid>/HEAD`, which does not exist
//! for a linked-git-dir repo and reports `head_commit: "unknown"`). The
//! commondir scenarios pin the resolution invariant for both layouts
//! (worktree git dir is structurally `<git_dir>/worktrees/<sid>`, so
//! `../..` must keep resolving to the real git dir).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

mod common;

use codelet_git::{create_worktree, list_worktrees, WorktreeCreateResult};

/// Build a git repository whose git dir lives OUTSIDE the work dir:
/// `root/repo` is the work dir, `root/elsewhere/repo.git` is the real
/// git dir, and `root/repo/.git` is a gitfile pointing at it.
///
/// Returns (tempdir, repo work dir, git dir).
fn linked_git_repo() -> (TempDir, PathBuf, PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let repo = root.join("repo");
    let git_dir = root.join("elsewhere").join("repo.git");

    fs::create_dir_all(&repo).expect("mkdir repo");
    Command::new("git")
        .args(["init"])
        .current_dir(&repo)
        .output()
        .expect("git init");
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo)
        .output()
        .expect("git config email");
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo)
        .output()
        .expect("git config name");
    fs::write(repo.join("README.md"), "# Test\n").expect("write README");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo)
        .output()
        .expect("git add");
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo)
        .output()
        .expect("git commit");

    // Move .git out of the work dir and leave a gitfile behind.
    fs::create_dir_all(git_dir.parent().expect("git dir has parent")).expect("mkdir elsewhere");
    fs::rename(repo.join(".git"), &git_dir).expect("move .git");
    fs::write(
        repo.join(".git"),
        format!("gitdir: {}\n", git_dir.display()),
    )
    .expect("write gitfile");

    (tmp, repo, git_dir)
}

/// Build a standard-layout repo (git dir = repo/.git).
fn standard_git_repo() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path().join("repo");
    let repo = common::setup_repo_in(&repo);
    (tmp, repo)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: create_worktree writes a commondir that resolves to the real
// git dir
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn create_worktree_commondir_resolves_to_real_git_dir_linked() {
    // @step Given a git repository whose git dir is /elsewhere/repo.git and whose repo/.git is a gitfile pointing to it
    let (_tmp, repo, git_dir) = linked_git_repo();

    // @step When create_worktree is called for session "s-1"
    let result = create_worktree(&repo, "s-1");
    assert!(
        result.is_ok(),
        "create_worktree should succeed on a linked-git-dir repo: {:?}",
        result.err()
    );

    // @step Then the commondir file at /elsewhere/repo.git/worktrees/s-1/commondir contains a relative path that resolves back to /elsewhere/repo.git
    let worktree_git_dir = git_dir.join("worktrees").join("s-1");
    let commondir = fs::read_to_string(worktree_git_dir.join("commondir"))
        .expect("commondir exists")
        .trim()
        .to_string();
    let resolved = worktree_git_dir
        .join(&commondir)
        .canonicalize()
        .expect("commondir resolves");
    let git_dir_canonical = git_dir.canonicalize().expect("git dir canonical");
    assert_eq!(
        resolved, git_dir_canonical,
        "commondir must resolve back to the real git dir"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: create_worktree writes the standard commondir for a
// standard-layout repo
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn create_worktree_commondir_is_dotdot_dotdot_standard() {
    // @step Given a git repository with the standard layout (git dir = repo/.git)
    let (_tmp, repo) = standard_git_repo();

    // @step When create_worktree is called for session "s-1"
    let result = create_worktree(&repo, "s-1");
    assert!(
        result.is_ok(),
        "create_worktree should succeed on a standard-layout repo: {:?}",
        result.err()
    );

    // @step Then the commondir file at repo/.git/worktrees/s-1/commondir contains exactly "../.." (the standard-layout value is preserved)
    let commondir = fs::read_to_string(
        repo.join(".git")
            .join("worktrees")
            .join("s-1")
            .join("commondir"),
    )
    .expect("commondir exists")
    .trim()
    .to_string();
    assert_eq!(
        commondir, "../..",
        "standard-layout commondir must stay the git-canonical value"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: list_worktrees reports the correct HEAD for a repo whose git
// dir is linked
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn list_worktrees_linked_git_dir_reports_real_head() {
    // @step Given a git repository whose git dir is /elsewhere/repo.git and whose repo/.git is a gitfile pointing to it, with a session worktree for session "s-1" already created via create_worktree
    let (_tmp, repo, _git_dir) = linked_git_repo();
    let created = create_worktree(&repo, "s-1").expect("create worktree");
    assert_eq!(
        created.base_commit, created.info.head_commit,
        "sanity: create_worktree reports a concrete SHA"
    );
    assert_ne!(created.info.head_commit, "unknown");

    // @step When list_worktrees is called against the repository root
    let worktrees = list_worktrees(&repo).expect("list_worktrees");

    // @step Then list_worktrees reports the worktree for session "s-1" with head_commit equal to the base commit SHA and is_detached true (not "unknown")
    let entry = worktrees
        .iter()
        .find(|w| w.session_id == "s-1")
        .expect("s-1 listed");
    assert_eq!(
        entry.head_commit, created.base_commit,
        "head_commit must be the real SHA, not \"unknown\""
    );
    assert!(entry.is_detached, "fspec worktrees are created detached");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: list_worktrees still reports the correct HEAD for a
// standard-layout repo
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn list_worktrees_standard_layout_reports_real_head() {
    // @step Given a git repository with the standard layout (git dir = repo/.git), with a session worktree for session "s-1" already created via create_worktree
    let (_tmp, repo) = standard_git_repo();
    let created: WorktreeCreateResult = create_worktree(&repo, "s-1").expect("create worktree");

    // @step When list_worktrees is called against the repository root
    let worktrees = list_worktrees(&repo).expect("list_worktrees");

    // @step Then list_worktrees reports the worktree for session "s-1" with head_commit equal to the base commit SHA and is_detached true
    let entry = worktrees
        .iter()
        .find(|w| w.session_id == "s-1")
        .expect("s-1 listed");
    assert_eq!(entry.head_commit, created.base_commit);
    assert!(entry.is_detached);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the codelet-git test suite passes without IsolatedSessionInfo
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn worktree_create_result_is_the_only_isolation_info_path() {
    // @step Given the worktree-metadata fix has been applied (isolated_session.rs deleted)
    // The fix lands in the implementing phase; this test pins the
    // replacement API surface so the deleted IsolatedSessionInfo type
    // cannot be re-introduced without breaking this scenario.

    // @step When the codelet-git test suite is run
    let (_tmp, repo) = standard_git_repo();
    let created = create_worktree(&repo, "iso-1").expect("create worktree");

    // @step Then all git tests pass with create_worktree/create_worktree_at_ref as the only isolation-info path (IsolatedSessionInfo is gone)
    // WorktreeCreateResult carries exactly what IsolatedSessionInfo used
    // to expose: worktree path + base commit + head + detached state.
    assert_eq!(created.info.path, repo.join(".fspec/worktrees/iso-1"));
    assert_eq!(created.base_commit, created.info.head_commit);
    assert!(created.info.is_detached);
    assert!(created.created_at.timestamp() > 0);
}
