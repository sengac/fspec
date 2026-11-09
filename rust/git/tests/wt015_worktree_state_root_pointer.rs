//! WT-015: isolated worktrees share the main checkout's board through one
//! state-root resolver.
//!
//! Feature: spec/features/isolated-worktrees-share-the-main-checkouts-board-through-one-state-root-resolver.feature
//!
//! Creating an isolated session worktree writes the state-root pointer the
//! resolver reads. The repository is a fresh temp repo with a repo-local
//! identity; every git call ignores the user's global and system config.

use std::path::Path;
use std::process::Command;

use codelet_common::spec_paths::{read_state_root_pointer, STATE_ROOT_POINTER};
use codelet_test_helpers::{OrFail, TestError, TestResult};

/// Run `git` in `cwd` without the user's global or system config; fail the
/// test on a non-zero exit.
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
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

#[test]
fn creating_an_isolated_worktree_writes_a_state_root_pointer_to_the_main_checkout() -> TestResult {
    // @step Given a temp git repo with spec/work-units.json committed
    let dir = tempfile::tempdir()?;
    let main = dir.path().canonicalize()?;
    git(&main, &["init", "-q"])?;
    git(&main, &["config", "user.name", "Test User"])?;
    git(&main, &["config", "user.email", "test@example.com"])?;
    std::fs::write(main.join(".gitignore"), ".fspec/\nspec/.fspec-write.lock\n")?;
    std::fs::create_dir_all(main.join("spec"))?;
    std::fs::write(main.join("spec").join("work-units.json"), "{}\n")?;
    git(&main, &["add", "-A"])?;
    git(&main, &["commit", "-q", "-m", "initial commit"])?;

    // @step When an isolated worktree is created for session "s1"
    let created = codelet_git::create_worktree(&main, "s1")?;
    let worktree = created.info.path;
    assert_eq!(
        worktree,
        main.join(codelet_git::FSPEC_WORKTREES_DIR).join("s1")
    );

    // @step Then the worktree's .fspec/state-root names the main checkout as stateRoot and "s1" as sessionId
    let raw = std::fs::read_to_string(worktree.join(STATE_ROOT_POINTER))?;
    let json: serde_json::Value = serde_json::from_str(&raw)?;
    assert_eq!(
        json["stateRoot"].as_str(),
        Some(main.to_string_lossy().as_ref()),
        "{raw}"
    );
    assert_eq!(json["sessionId"].as_str(), Some("s1"), "{raw}");
    let pointer = read_state_root_pointer(&worktree)?.or_fail("a pointer")?;
    assert_eq!(pointer.state_root, main);
    assert_eq!(pointer.session_id, "s1");

    // @step And the main checkout's git status is clean
    let status = git(&main, &["status", "--porcelain"])?;
    assert_eq!(status, "", "main's git status: {status}");
    Ok(())
}
