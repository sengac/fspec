//! WT-006 — session merge/diff machinery is git-blind: ignored files,
//! symlinks, and file modes.
//!
//! Feature: spec/features/session-merge-diff-machinery-is-git-blind-ignored-files-symlinks-and-file-modes.feature
//!
//! Pins the three audit defects (spec/attachments/WT-006/wt-006.md):
//! (a) gitignored untracked files are invisible to the session diff and
//!     the merge sweep, (b) symlinks are dropped by the tree snapshot so
//!     the merge deletion sweep removes them from main, (c) the
//!     executable bit is lost across merge and ghost-checkpoint
//!     round-trips. Also pins the atomic gitlink treatment (submodules
//!     are never reported deleted and never deleted from main).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use codelet_git::ghost_commit;
use codelet_git::{create_worktree, inspect_session, merge_session, FSPEC_WORKTREES_DIR};

// =============================================================================
// Helpers
// =============================================================================

/// Run `git` in `dir`, panicking with the output on failure.
fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to spawn git");
    assert!(
        output.status.success(),
        "git {args:?} failed in {}: stderr: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Commit a real symlink `link_path -> target` at the repo root.
fn commit_symlink(repo_path: &Path, link_path: &str, target: &str) {
    let abs_link = repo_path.join(link_path);
    if let Some(parent) = abs_link.parent() {
        fs::create_dir_all(parent).expect("create symlink parent dir");
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, &abs_link).expect("create symlink");
    #[cfg(not(unix))]
    fs::write(&abs_link, target).expect("write link target as file (windows)");

    git(repo_path, &["add", link_path]);
    git(
        repo_path,
        &["commit", "-m", &format!("commit symlink {link_path}")],
    );
}

/// Commit an executable file (mode 100755) at the repo root.
fn commit_executable(repo_path: &Path, file_path: &str, content: &str) {
    let abs = repo_path.join(file_path);
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(&abs, content).expect("write file");
    let mut perms = fs::metadata(&abs).expect("stat file").permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
        fs::set_permissions(&abs, perms).expect("chmod +x");
    }
    git(repo_path, &["add", file_path]);
    git(
        repo_path,
        &["commit", "-m", &format!("commit executable {file_path}")],
    );
}

/// Whether a path on disk is a symlink (follows nothing — lstat check).
#[cfg(unix)]
fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_symlink(_path: &Path) -> bool {
    false
}

/// Read the raw symlink target (None if not a symlink).
#[cfg(unix)]
fn symlink_target(path: &Path) -> Option<String> {
    fs::read_link(path)
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

#[cfg(not(unix))]
fn symlink_target(_path: &Path) -> Option<String> {
    None
}

/// Whether a regular file on disk is executable (unix only).
#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    true // exec bit is a no-op on windows; treat as satisfied
}

/// Worktree path helper: `<repo>/.fspec/worktrees/<session_id>`.
fn worktree_path(repo_path: &Path, session_id: &str) -> std::path::PathBuf {
    repo_path.join(FSPEC_WORKTREES_DIR).join(session_id)
}

// =============================================================================
// Part A: Gitignored files are surfaced honestly
// =============================================================================

/// Feature scenario: Session diff reports build artifacts as ignored rather
/// than as changes.
#[test]
fn session_diff_reports_ignored_files_as_ignored_not_as_changes() {
    // @step Given a git repository whose .gitignore ignores "build/"
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();
    fs::write(repo_path.join(".gitignore"), "build/\n").expect("write .gitignore");
    git(repo_path, &["add", ".gitignore"]);
    git(repo_path, &["commit", "-m", "gitignore build/"]);

    // @step And a session worktree where tracked file "src/main.rs" is unchanged and "build/out.bin" was created
    let session_id = "wt006-ignored-diff";
    create_worktree(repo_path, session_id).expect("create worktree");
    let wt = worktree_path(repo_path, session_id);
    fs::create_dir_all(wt.join("build")).expect("create build dir");
    fs::write(wt.join("build/out.bin"), b"artifact").expect("write ignored artifact");

    // @step When I get the session diff for that session
    let diff = codelet_git::get_session_diff(repo_path, session_id).expect("get_session_diff");

    // @step Then files_changed, files_added and files_deleted are all empty
    assert!(
        diff.files_changed.is_empty(),
        "files_changed must be empty for an ignored-only delta, got {:?}",
        diff.files_changed
    );
    assert!(
        diff.files_added.is_empty(),
        "files_added must be empty for an ignored-only delta, got {:?}",
        diff.files_added
    );
    assert!(
        diff.files_deleted.is_empty(),
        "files_deleted must be empty for an ignored-only delta, got {:?}",
        diff.files_deleted
    );

    // @step And files_ignored contains "build/out.bin"
    assert!(
        diff.files_ignored.contains(&"build/out.bin".to_string()),
        "WT-006: gitignored untracked files must be surfaced in files_ignored, got {:?}",
        diff.files_ignored
    );

    // @step And the unified diff does not mention "build/out.bin"
    assert!(
        !diff.diff.contains("build/out.bin"),
        "WT-006: gitignored files must not appear in the unified diff:\n{}",
        diff.diff
    );
}

/// Feature scenario: Merging a session with only ignored artifacts does not
/// touch the main worktree.
#[test]
fn merging_session_with_only_ignored_artifacts_leaves_main_intact() {
    // @step Given a git repository whose .gitignore ignores "build/"
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();
    fs::write(repo_path.join(".gitignore"), "build/\n").expect("write .gitignore");
    git(repo_path, &["add", ".gitignore"]);
    git(repo_path, &["commit", "-m", "gitignore build/"]);

    // @step And a gitignored file "build/local.txt" created in the MAIN worktree after the session started
    let session_id = "wt006-ignored-merge";
    create_worktree(repo_path, session_id).expect("create worktree");
    fs::create_dir_all(repo_path.join("build")).expect("create main build dir");
    fs::write(repo_path.join("build/local.txt"), "main-local").expect("write main ignored file");

    // @step And a session worktree whose only change is the ignored file "build/session.bin"
    let wt = worktree_path(repo_path, session_id);
    fs::create_dir_all(wt.join("build")).expect("create wt build dir");
    fs::write(wt.join("build/session.bin"), b"session-artifact")
        .expect("write worktree ignored artifact");

    // @step When I merge that session into main
    let result = merge_session(repo_path, session_id);

    // @step Then the merge reports no modified, added or deleted files
    let result = result.expect("merge of an ignored-only session must succeed");
    assert!(
        result.files_modified.is_empty()
            && result.files_added.is_empty()
            && result.files_deleted.is_empty(),
        "WT-006: merge must report zero changes for ignored-only sessions, got {result:?}"
    );

    // @step And the session worktree is removed
    assert!(
        !worktree_path(repo_path, session_id).exists(),
        "worktree must be removed after a successful merge"
    );

    // @step And "build/local.txt" in main still exists with its content
    let local = fs::read_to_string(repo_path.join("build/local.txt"))
        .expect("main build/local.txt must survive the merge");
    assert_eq!(
        local, "main-local",
        "ignored main file content must be untouched"
    );
}

/// Feature scenario: inspect_session_changes reports zero changed files for a
/// worktree whose only delta is ignored.
#[test]
fn inspect_reports_zero_changed_files_when_only_delta_is_ignored() {
    // @step Given a session worktree whose only delta is gitignored content
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();
    fs::write(repo_path.join(".gitignore"), "logs/\n").expect("write .gitignore");
    git(repo_path, &["add", ".gitignore"]);
    git(repo_path, &["commit", "-m", "gitignore logs/"]);
    let session_id = "wt006-ignored-inspect";
    create_worktree(repo_path, session_id).expect("create worktree");
    let wt = worktree_path(repo_path, session_id);
    fs::create_dir_all(wt.join("logs")).expect("create logs dir");
    fs::write(wt.join("logs/session.log"), "noise\n").expect("write ignored log");

    // @step When I inspect the session's pending changes
    let result = inspect_session(repo_path, session_id).expect("inspect_session");

    // @step Then files_changed is 0
    assert!(
        result.files_changed.is_empty()
            && result.files_added.is_empty()
            && result.files_deleted.is_empty(),
        "WT-006: ignored-only delta must report zero tracked changes, got changed={:?} added={:?} deleted={:?}",
        result.files_changed,
        result.files_added,
        result.files_deleted
    );

    // @step And the ignored files are surfaced in the summary's files_ignored list
    assert!(
        result
            .files_ignored
            .contains(&"logs/session.log".to_string()),
        "WT-006: inspect must surface ignored files, got {:?}",
        result.files_ignored
    );
}

// =============================================================================
// Part B: Symlinks round-trip through worktree creation, diff, and merge
// =============================================================================

/// Feature scenario: Worktree checkout materializes committed symlinks as
/// real symlinks.
#[test]
fn worktree_checkout_materializes_committed_symlinks() {
    // @step Given a git repository with a committed symlink "links/config" pointing to "real/config.txt"
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();
    fs::create_dir_all(repo_path.join("real")).expect("create real dir");
    fs::write(repo_path.join("real/config.txt"), "config content\n").expect("write real config");
    git(repo_path, &["add", "real/config.txt"]);
    git(repo_path, &["commit", "-m", "real config"]);
    commit_symlink(repo_path, "links/config", "real/config.txt");

    // @step When I create a session worktree at HEAD
    let session_id = "wt006-symlink-create";
    create_worktree(repo_path, session_id).expect("create worktree");
    let wt = worktree_path(repo_path, session_id);

    // @step Then "links/config" in the worktree is a symlink
    let link = wt.join("links/config");
    #[cfg(unix)]
    {
        assert!(
            is_symlink(&link),
            "WT-006: committed symlink must be materialized as a real symlink in the worktree"
        );
    }

    // @step And the symlink target of "links/config" in the worktree is "real/config.txt"
    #[cfg(unix)]
    assert_eq!(
        symlink_target(&link).as_deref(),
        Some("real/config.txt"),
        "symlink target must round-trip"
    );
}

/// Feature scenario: Merging a session preserves committed symlinks in main.
#[test]
fn merging_session_preserves_committed_symlinks_in_main() {
    // @step Given a git repository with a committed symlink "links/config" pointing to "real/config.txt"
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();
    fs::create_dir_all(repo_path.join("real")).expect("create real dir");
    fs::write(repo_path.join("real/config.txt"), "config content\n").expect("write real config");
    git(repo_path, &["add", "real/config.txt"]);
    git(repo_path, &["commit", "-m", "real config"]);
    // Target is repo-relative from the link's own directory so the link
    // RESOLVES (a broken link would make `Path::exists` return false even
    // when the symlink entry is intact).
    commit_symlink(repo_path, "links/config", "../real/config.txt");

    // @step And a session worktree where tracked file "src/main.rs" has been modified
    let session_id = "wt006-symlink-merge";
    create_worktree(repo_path, session_id).expect("create worktree");
    let wt = worktree_path(repo_path, session_id);
    fs::write(wt.join("src/main.rs"), "fn main() { println!(\"wt\"); }\n")
        .expect("modify main.rs in worktree");

    // @step When I merge that session into main
    merge_session(repo_path, session_id)
        .expect("merge must succeed even though main has a committed symlink");

    // @step Then "links/config" in main still exists
    let link = repo_path.join("links/config");
    assert!(
        link.exists(),
        "WT-006: a committed symlink must not be deleted from main by a merge"
    );

    // @step And "links/config" in main is a symlink
    #[cfg(unix)]
    assert!(
        is_symlink(&link),
        "WT-006: links/config in main must remain a symlink after merge"
    );

    // @step And the symlink target of "links/config" in main is "real/config.txt"
    #[cfg(unix)]
    assert_eq!(
        symlink_target(&link).as_deref(),
        Some("../real/config.txt"),
        "symlink target must be preserved through the merge"
    );
}

/// Feature scenario: A symlink whose target changed in the session is
/// applied to main as a symlink.
#[test]
fn re_pointed_symlink_is_applied_to_main_as_symlink() {
    // @step Given a git repository with a committed symlink "links/config" pointing to "real/config.txt"
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();
    fs::create_dir_all(repo_path.join("real")).expect("create real dir");
    fs::write(repo_path.join("real/config.txt"), "config content\n").expect("write real config");
    git(repo_path, &["add", "real/config.txt"]);
    git(repo_path, &["commit", "-m", "real config"]);
    commit_symlink(repo_path, "links/config", "real/config.txt");

    // @step And a session worktree where "links/config" was re-pointed to "real/other.txt"
    let session_id = "wt006-symlink-repoint";
    create_worktree(repo_path, session_id).expect("create worktree");
    let wt = worktree_path(repo_path, session_id);
    fs::write(repo_path.join("real/other.txt"), "other content\n").expect("write other target");
    git(repo_path, &["add", "real/other.txt"]);
    git(repo_path, &["commit", "-m", "other target"]);
    let link = wt.join("links/config");
    #[cfg(unix)]
    {
        fs::remove_file(&link).expect("remove old symlink");
        std::os::unix::fs::symlink("real/other.txt", &link).expect("re-point symlink");
    }

    // @step When I merge that session into main
    merge_session(repo_path, session_id).expect("merge of a re-pointed symlink must succeed");

    // @step Then "links/config" in main is a symlink
    let main_link = repo_path.join("links/config");
    #[cfg(unix)]
    assert!(
        is_symlink(&main_link),
        "WT-006: re-pointed symlink must land in main as a real symlink"
    );

    // @step And the symlink target of "links/config" in main is "real/other.txt"
    #[cfg(unix)]
    assert_eq!(
        symlink_target(&main_link).as_deref(),
        Some("real/other.txt"),
        "new symlink target must be applied to main"
    );
}

/// Feature scenario: A symlink deleted in the session is deleted from main.
#[test]
fn deleted_symlink_is_removed_from_main() {
    // @step Given a git repository with a committed symlink "links/config" pointing to "real/config.txt"
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();
    fs::create_dir_all(repo_path.join("real")).expect("create real dir");
    fs::write(repo_path.join("real/config.txt"), "config content\n").expect("write real config");
    git(repo_path, &["add", "real/config.txt"]);
    git(repo_path, &["commit", "-m", "real config"]);
    commit_symlink(repo_path, "links/config", "real/config.txt");

    // @step And a session worktree where "links/config" has been deleted
    let session_id = "wt006-symlink-delete";
    create_worktree(repo_path, session_id).expect("create worktree");
    let wt = worktree_path(repo_path, session_id);
    fs::remove_file(wt.join("links/config")).expect("delete symlink in worktree");

    // @step When I merge that session into main
    merge_session(repo_path, session_id).expect("merge of a symlink deletion must succeed");

    // @step Then "links/config" no longer exists in main
    assert!(
        !repo_path.join("links/config").exists(),
        "deleted symlink must be removed from main on merge"
    );
}

/// Feature scenario: Session diff reports a re-pointed symlink as changed,
/// not deleted.
#[test]
fn diff_reports_re_pointed_symlink_as_changed_not_deleted() {
    // @step Given a git repository with a committed symlink "links/config" pointing to "real/config.txt"
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();
    fs::create_dir_all(repo_path.join("real")).expect("create real dir");
    fs::write(repo_path.join("real/config.txt"), "config content\n").expect("write real config");
    git(repo_path, &["add", "real/config.txt"]);
    git(repo_path, &["commit", "-m", "real config"]);
    commit_symlink(repo_path, "links/config", "real/config.txt");

    // @step And a session worktree where "links/config" was re-pointed to "real/other.txt"
    let session_id = "wt006-symlink-diff";
    create_worktree(repo_path, session_id).expect("create worktree");
    let wt = worktree_path(repo_path, session_id);
    let link = wt.join("links/config");
    #[cfg(unix)]
    {
        fs::remove_file(&link).expect("remove old symlink");
        std::os::unix::fs::symlink("real/other.txt", &link).expect("re-point symlink");
    }

    // @step When I get the session diff for that session
    let diff = codelet_git::get_session_diff(repo_path, session_id).expect("get_session_diff");

    // @step Then files_changed contains "links/config"
    assert!(
        diff.files_changed.contains(&"links/config".to_string()),
        "WT-006: a re-pointed symlink must be reported as changed, got changed={:?} deleted={:?}",
        diff.files_changed,
        diff.files_deleted
    );

    // @step And files_deleted does not contain "links/config"
    assert!(
        !diff.files_deleted.contains(&"links/config".to_string()),
        "WT-006: a re-pointed symlink must NOT be reported as deleted"
    );
}

// =============================================================================
// Part C: File modes (the executable bit) survive merge and checkpoints
// =============================================================================

/// Feature scenario: Merging a session preserves the executable bit of an
/// unchanged executable file.
#[test]
fn merging_session_preserves_executable_bit_of_unchanged_file() {
    // @step Given a git repository with an executable file "bin/tool" committed at mode 100755
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();
    commit_executable(repo_path, "bin/tool", "#!/bin/sh\necho tool\n");

    // @step And a session worktree where tracked file "src/main.rs" has been modified
    let session_id = "wt006-exec-unchanged";
    create_worktree(repo_path, session_id).expect("create worktree");
    let wt = worktree_path(repo_path, session_id);
    fs::write(wt.join("src/main.rs"), "fn main() { println!(\"wt\"); }\n")
        .expect("modify main.rs in worktree");

    // @step When I merge that session into main
    merge_session(repo_path, session_id).expect("merge must succeed");

    // @step Then "bin/tool" in main still exists
    let tool = repo_path.join("bin/tool");
    assert!(tool.exists(), "bin/tool must survive the merge");

    // @step And "bin/tool" in main is executable
    assert!(
        is_executable(&tool),
        "WT-006: the executable bit must be preserved through the merge"
    );
}

/// Feature scenario: The executable bit added in a session is applied to
/// main.
#[test]
fn executable_bit_added_in_session_is_applied_to_main() {
    // @step Given a git repository with a regular (non-executable) file "bin/tool"
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();
    fs::create_dir_all(repo_path.join("bin")).expect("create bin dir");
    fs::write(repo_path.join("bin/tool"), "#!/bin/sh\necho tool\n")
        .expect("write non-executable tool");
    git(repo_path, &["add", "bin/tool"]);
    git(repo_path, &["commit", "-m", "tool (non-exec)"]);

    // @step And a session worktree where "bin/tool" has been made executable
    let session_id = "wt006-exec-added";
    create_worktree(repo_path, session_id).expect("create worktree");
    let wt = worktree_path(repo_path, session_id);
    let mut perms = fs::metadata(wt.join("bin/tool"))
        .expect("stat worktree tool")
        .permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
        fs::set_permissions(wt.join("bin/tool"), perms).expect("chmod +x in worktree");
    }

    // @step When I merge that session into main
    merge_session(repo_path, session_id).expect("merge must succeed");

    // @step Then "bin/tool" in main is executable
    assert!(
        is_executable(&repo_path.join("bin/tool")),
        "WT-006: an executable bit added in the session must be applied to main"
    );
}

/// Feature scenario: A ghost checkpoint round-trip preserves the executable
/// bit and content.
#[test]
fn ghost_checkpoint_round_trip_preserves_executable_bit() {
    // @step Given a git repository with an executable file "bin/tool"
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();
    commit_executable(repo_path, "bin/tool", "#!/bin/sh\necho v1\n");

    // @step When I create a ghost checkpoint and then modify "bin/tool" content and remove its executable bit
    ghost_commit::create_ghost_commit(repo_path, "WT006", "snap").expect("create checkpoint");
    fs::write(repo_path.join("bin/tool"), "#!/bin/sh\necho v2\n").expect("modify tool");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(repo_path.join("bin/tool"))
            .expect("stat tool")
            .permissions();
        perms.set_mode(0o644);
        fs::set_permissions(repo_path.join("bin/tool"), perms).expect("chmod -x tool");
    }

    // @step And I restore the checkpoint with force
    ghost_commit::restore_ghost_commit(repo_path, "WT006", "snap", true)
        .expect("force restore must succeed");

    // @step Then "bin/tool" has its original content
    let content = fs::read_to_string(repo_path.join("bin/tool")).expect("read restored tool");
    assert_eq!(
        content, "#!/bin/sh\necho v1\n",
        "restored content must match"
    );

    // @step And "bin/tool" in the working tree is executable again
    assert!(
        is_executable(&repo_path.join("bin/tool")),
        "WT-006: the executable bit must survive a checkpoint round-trip"
    );
}

// =============================================================================
// Part D: Submodules (gitlinks) are atomic
// =============================================================================

/// Feature scenario: A committed submodule is reported as present (not
/// deleted) and survives a merge.
#[test]
fn committed_submodule_is_not_deleted_by_diff_or_merge() {
    // @step Given a git repository with a committed submodule (gitlink) "vendor/lib"
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();

    // Build the inner repo and clone it as a "submodule" (gitlink entry).
    let inner = tempfile::tempdir().expect("tempdir for inner repo");
    git(inner.path(), &["init", "-b", "main"]);
    git(inner.path(), &["config", "user.email", "t@example.com"]);
    git(inner.path(), &["config", "user.name", "T"]);
    fs::write(inner.path().join("lib.txt"), "inner\n").expect("write inner file");
    git(inner.path(), &["add", "lib.txt"]);
    git(inner.path(), &["commit", "-m", "inner"]);

    let vendor = repo_path.join("vendor");
    fs::create_dir_all(&vendor).expect("create vendor dir");
    let dest = vendor.join("lib");
    Command::new("git")
        .args([
            "clone",
            inner.path().to_str().expect("str"),
            dest.to_str().expect("str"),
        ])
        .current_dir(repo_path)
        .output()
        .expect("git clone inner");
    git(
        repo_path,
        &[
            "config",
            "submodule.vendor/lib.url",
            inner.path().to_str().expect("str"),
        ],
    );
    git(repo_path, &["add", "vendor/lib"]);
    git(repo_path, &["commit", "-m", "add submodule vendor/lib"]);

    // @step And a session worktree where tracked file "src/main.rs" has been modified
    let session_id = "wt006-gitlink";
    create_worktree(repo_path, session_id).expect("create worktree");
    let wt = worktree_path(repo_path, session_id);
    fs::write(wt.join("src/main.rs"), "fn main() { println!(\"wt\"); }\n")
        .expect("modify main.rs in worktree");

    // @step When I get the session diff for that session
    let diff = codelet_git::get_session_diff(repo_path, session_id).expect("get_session_diff");

    // @step Then files_deleted does not contain "vendor/lib"
    assert!(
        !diff.files_deleted.contains(&"vendor/lib".to_string()),
        "WT-006: a base gitlink must never be reported as deleted (the worktree cannot clone it), got {:?}",
        diff.files_deleted
    );

    // @step When I merge that session into main
    merge_session(repo_path, session_id).expect("merge must not fail or clobber the submodule");

    // @step Then "vendor/lib" in main still exists with its content
    assert!(
        repo_path.join("vendor/lib/lib.txt").exists(),
        "WT-006: the main submodule directory must be untouched by the merge"
    );
}
