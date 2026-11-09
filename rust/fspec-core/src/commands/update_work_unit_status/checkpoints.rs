//! Auto-checkpoints for `update-work-unit-status` (system `git` binary — blocking, poll_sync_future-safe).

use std::path::Path;

use crate::error::FspecCoreError;

pub(super) fn git(project_root: &Path, args: &[&str]) -> Option<std::process::Output> {
    std::process::Command::new("git")
        .args(args)
        .current_dir(project_root)
        .output()
        .ok()
}

pub(super) fn working_dir_dirty(project_root: &Path) -> bool {
    git(project_root, &["status", "--porcelain"])
        .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false)
}

/// Create an auto-checkpoint ref pointing at a commit of the current tree when
/// the working directory is dirty. Mirrors the ghost-commit approach using the
/// system git binary so it is poll_sync_future-safe.
pub(super) fn maybe_auto_checkpoint(
    project_root: &Path,
    id: &str,
    checkpoint_name: &str,
) -> Result<(), FspecCoreError> {
    if !working_dir_dirty(project_root) {
        return Ok(());
    }
    // Stash-free ghost commit: write a tree from the index+worktree via a
    // temporary add, capture the commit, then point a checkpoint ref at it
    // WITHOUT moving HEAD or mutating the user's index permanently.
    // Simplest robust approach: `git stash create` produces a commit object
    // for the dirty state without touching the working tree.
    let stash = git(project_root, &["stash", "create", "fspec-auto-checkpoint"]);
    let commit = stash
        .as_ref()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    let target = match commit {
        Some(c) => c,
        None => {
            // Nothing stashable resolved — fall back to HEAD.
            git(project_root, &["rev-parse", "HEAD"])
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_default()
        }
    };
    if target.is_empty() {
        return Ok(());
    }
    let ref_name = format!("refs/fspec-checkpoints/{id}/{checkpoint_name}");
    git(project_root, &["update-ref", &ref_name, &target]);
    Ok(())
}

/// Remove auto-checkpoint refs (containing `-auto-`) for `id`, preserving manual.
pub(super) fn cleanup_done_checkpoints(project_root: &Path, id: &str) {
    let prefix = format!("refs/fspec-checkpoints/{id}/");
    if let Some(out) = git(
        project_root,
        &["for-each-ref", "--format=%(refname)", &prefix],
    ) {
        let refs: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.contains("-auto-"))
            .map(|l| l.trim().to_string())
            .collect();
        for r in refs {
            git(project_root, &["update-ref", "-d", &r]);
        }
    }
}
