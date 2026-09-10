//! WT-008 — /merge-worktree does not commit anything and the
//! 'FastForward' strategy is silently ignored.
//!
//! Feature: spec/features/merge-worktree-does-not-commit-anything-and-the-fastforward-strategy-is-silently-ignored.feature
//!
//! Drives the real `codelet_git::merge_session_with_strategy` against
//! fresh git repositories (temp dirs, `git` CLI for assertions) so the
//! commit, HEAD movement, and strategy gate are observed on-disk.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;

use uuid::Uuid;

/// Run a `git` CLI command in `cwd` and fail the test if it exits non-zero.
fn run_git(cwd: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to spawn git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Run a `git` CLI command in `cwd` and return its stdout (must succeed).
fn git_out(cwd: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to spawn git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Create a fresh git repository (temp dir) with one committed file.
///
/// The initial commit uses `-c` flags so NO user.name/user.email is
/// stored in the repo config — the merge-commit code must fall back to
/// its fspec identity for such repos.
fn fresh_git_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir for git repo");
    let path = dir.path();
    run_git(path, &["init"]);
    std::fs::write(path.join("hello.txt"), "hello\n").expect("write hello.txt");
    run_git(path, &["add", "."]);
    run_git(
        path,
        &[
            "-c",
            "user.name=Test User",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            "initial commit",
        ],
    );
    dir
}

/// Read the repo's current HEAD SHA.
fn resolve_head(repo: &Path) -> String {
    git_out(repo, &["rev-parse", "HEAD"])
}

// =============================================================================
// Scenario: Merging a session with tracked changes commits them to main
// =============================================================================

#[test]
fn scenario_merging_a_session_with_tracked_changes_commits_them_to_main() {
    // @step Given a temp git repo with one committed file, and an isolated session that has committed a modification to that file in its worktree
    let repo = fresh_git_repo();
    let session_id = Uuid::new_v4().to_string();
    codelet_git::create_worktree(repo.path(), &session_id)
        .expect("create worktree");
    let head_before = resolve_head(repo.path());
    let worktree = repo.path().join(codelet_git::FSPEC_WORKTREES_DIR).join(&session_id);
    std::fs::write(worktree.join("hello.txt"), "hello (merged)\n").expect("write session edit");

    // @step When the session is merged with the default FastForward strategy
    let result = codelet_git::merge_session_with_strategy(
        repo.path(),
        &session_id,
        &codelet_rpc_types::MergeStrategy::FastForward,
    )
    .expect("merge must succeed");

    // @step Then the merge reports success with a real merge commit SHA and the main repo's HEAD points at a new commit whose message is 'fspec: merge session <id>' and whose tree contains the session's change, with the main working tree clean
    let sha = result
        .merge_commit
        .clone()
        .expect("merge_commit must be Some");
    assert_ne!(sha, head_before, "the merge must create a NEW commit");
    assert_eq!(
        resolve_head(repo.path()),
        sha,
        "main repo HEAD must point at the merge commit"
    );
    assert_eq!(
        git_out(repo.path(), &["log", "-1", "--format=%s"]),
        format!("fspec: merge session {session_id}"),
        "the merge commit message must name the session"
    );
    // No repo user config was set → the fallback fspec identity.
    assert_eq!(
        git_out(repo.path(), &["log", "-1", "--format=%an"]),
        "fspec",
        "without repo config the commit falls back to the fspec identity"
    );
    assert_eq!(
        git_out(repo.path(), &["log", "-1", "--format=%ae"]),
        "fspec@local",
        "without repo config the commit falls back to the fspec identity email"
    );
    // The committed tree contains the session's change.
    assert_eq!(
        git_out(repo.path(), &["show", "HEAD:hello.txt"]),
        "hello (merged)",
        "the committed tree must contain the session's change"
    );
    assert!(
        git_out(repo.path(), &["status", "--porcelain"]).is_empty(),
        "the main working tree must be clean after the merge"
    );
}

// =============================================================================
// Scenario: Requesting an unsupported merge strategy is rejected before
// anything is applied
// =============================================================================

#[test]
fn scenario_requesting_an_unsupported_merge_strategy_is_rejected_before_anything_is_applied() {
    // @step Given a temp git repo with one committed file, and an isolated session with a committed change in its worktree
    let repo = fresh_git_repo();
    let session_id = Uuid::new_v4().to_string();
    codelet_git::create_worktree(repo.path(), &session_id)
        .expect("create worktree");
    let head_before = resolve_head(repo.path());
    let worktree = repo.path().join(codelet_git::FSPEC_WORKTREES_DIR).join(&session_id);
    std::fs::write(worktree.join("hello.txt"), "hello (squashed)\n").expect("write session edit");

    // @step When the session is merged requesting the Squash strategy
    let result = codelet_git::merge_session_with_strategy(
        repo.path(),
        &session_id,
        &codelet_rpc_types::MergeStrategy::Squash,
    );

    // @step Then the merge returns an explicit unsupported-strategy error and nothing is committed, copied, or merged (the main repo HEAD is unchanged and the session worktree is intact)
    match result {
        Err(codelet_git::GitError::UnsupportedMergeStrategy { .. }) => {}
        other => panic!(
            "expected UnsupportedMergeStrategy, got {other:?}"
        ),
    }
    assert_eq!(
        resolve_head(repo.path()),
        head_before,
        "nothing must be committed"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("hello.txt")).expect("read main hello.txt"),
        "hello\n",
        "nothing must be copied into main"
    );
    assert!(
        worktree.exists(),
        "the session worktree must stay intact for retry"
    );
}

// =============================================================================
// Scenario: Merging a session with only ignored changes creates no commit
// =============================================================================

#[test]
fn scenario_merging_a_session_with_only_ignored_changes_creates_no_commit() {
    // @step Given a temp git repo with one committed file, and an isolated session whose worktree only contains gitignored build artifacts (no tracked changes)
    let repo = fresh_git_repo();
    std::fs::write(repo.path().join(".gitignore"), "target/\n").expect("write .gitignore");
    run_git(repo.path(), &["add", ".gitignore"]);
    run_git(
        repo.path(),
        &[
            "-c",
            "user.name=Test User",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            "add .gitignore",
        ],
    );
    let session_id = Uuid::new_v4().to_string();
    codelet_git::create_worktree(repo.path(), &session_id)
        .expect("create worktree");
    let head_before = resolve_head(repo.path());
    let worktree = repo.path().join(codelet_git::FSPEC_WORKTREES_DIR).join(&session_id);
    std::fs::create_dir_all(worktree.join("target")).expect("mkdir target in worktree");
    std::fs::write(worktree.join("target").join("out.bin"), b"build output").expect("write artifact");

    // @step When the session is merged with the default FastForward strategy
    let result = codelet_git::merge_session_with_strategy(
        repo.path(),
        &session_id,
        &codelet_rpc_types::MergeStrategy::FastForward,
    )
    .expect("merge must succeed");

    // @step Then the merge reports nothing-to-merge (NoChanges), creates no commit (the main repo HEAD is unchanged), and surfaces no merge commit SHA
    assert!(
        result.files_modified.is_empty()
            && result.files_added.is_empty()
            && result.files_deleted.is_empty(),
        "no tracked changes: {result:?}"
    );
    assert!(
        result.merge_commit.is_none(),
        "no tracked changes must produce no commit"
    );
    assert_eq!(
        resolve_head(repo.path()),
        head_before,
        "the main repo HEAD must be unchanged"
    );
}

// =============================================================================
// Scenario: The merge commit uses the repo's configured identity
// =============================================================================

#[test]
fn scenario_the_merge_commit_uses_the_repo_configured_identity() {
    // @step Given a temp git repo with user.name and user.email set in its git config, and an isolated session with a committed change in its worktree
    let repo = fresh_git_repo();
    run_git(repo.path(), &["config", "user.name", "Repo Owner"]);
    run_git(repo.path(), &["config", "user.email", "owner@example.com"]);
    let session_id = Uuid::new_v4().to_string();
    codelet_git::create_worktree(repo.path(), &session_id)
        .expect("create worktree");
    let worktree = repo.path().join(codelet_git::FSPEC_WORKTREES_DIR).join(&session_id);
    std::fs::write(worktree.join("hello.txt"), "hello (owned)\n").expect("write session edit");

    // @step When the session is merged with the default FastForward strategy
    codelet_git::merge_session_with_strategy(
        repo.path(),
        &session_id,
        &codelet_rpc_types::MergeStrategy::FastForward,
    )
    .expect("merge must succeed");

    // @step Then the merge commit is authored by the configured user.name/user.email
    assert_eq!(
        git_out(repo.path(), &["log", "-1", "--format=%an"]),
        "Repo Owner",
        "the commit must use the repo's configured user.name"
    );
    assert_eq!(
        git_out(repo.path(), &["log", "-1", "--format=%ae"]),
        "owner@example.com",
        "the commit must use the repo's configured user.email"
    );
}
