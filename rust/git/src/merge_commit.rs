//! WT-008: commit the merged session into the MAIN repository.
//!
//! `merge_session` used to copy files into the main working tree and
//! stop there — nothing was ever committed. After the copy succeeds,
//! `commit_merged_session` creates a real commit:
//!
//! - the tree is built from the MAIN repo's HEAD tree with the
//!   session's tracked delta applied (added/modified blobs, symlinks;
//!   deleted paths removed), so the commit contains exactly the
//!   session's change and never sweeps in unrelated
//!   main-working-tree content;
//! - the parent is the current HEAD (fast-forward on the checked-out
//!   branch);
//! - author/committer identity comes from the repo-local git config
//!   (`user.name`/`user.email`) with a fixed `fspec` fallback;
//! - the commit message is `fspec: merge session <session-id>`;
//! - the main repo's index is synced for the delta paths only (added /
//!   modified / deleted), so `git status` is clean afterwards and the
//!   user's own staged/unstaged unrelated work is left untouched.

use crate::error::Result;
use crate::ghost_commit::build_tree_from_snapshot;
use crate::git_commit_signature;
use crate::open_repo;
use crate::session_result::SessionResult;
use crate::tree_snapshot::TreeSnapshot;
use crate::tree_utils::get_tree_files;
use crate::GitError;
use gix::bstr::BStr;
use std::path::{Path, PathBuf};

/// Commit the session's merged state into the main repository.
///
/// Called from `merge_session` AFTER `apply_session_changes` has
/// copied the session's changes into the main working tree.
/// `applied` is the post-copy worktree snapshot (the exact state now
/// present in main for the session's paths).
///
/// # Returns
/// The hex SHA of the new commit.
pub fn commit_merged_session(
    repo_path: impl AsRef<Path>,
    session_id: &str,
    applied: &TreeSnapshot,
    diff: &SessionResult,
) -> Result<String> {
    let repo_path = repo_path.as_ref();
    let repo = open_repo(repo_path)?;

    // Parent: current HEAD (fast-forward on the checked-out branch).
    let parent = repo
        .head_commit()
        .map_err(|e| GitError::Head(format!("No HEAD commit: {e}")))?;

    let head_tree = get_tree_files(&repo, &parent.id.to_string())?;
    let final_tree = overlay_session_delta(head_tree, applied, diff);
    let tree_id = build_tree_from_snapshot(&repo, &final_tree)?;

    // Identity: repo-local git config, fixed fspec fallback.
    let (name, email) = resolve_identity(repo_path);
    let message = format!("fspec: merge session {session_id}");

    let commit_id = git_commit_signature(&repo, tree_id, &parent.id, &name, &email, &message)?;

    // Keep `git status` honest: sync the index for the delta paths
    // only (added/modified → new entries, deleted → removed).
    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?
        .to_path_buf();
    sync_index_delta(&repo, &workdir, applied, diff)?;

    Ok(commit_id)
}

/// WT-013: main's HEAD tree with only the session's delta applied.
///
/// `applied` holds every tracked path of the worktree, the untouched
/// ones at the session's BASE content, so overlaying all of it would
/// revert every file main committed after the base. Only the paths the
/// session added, modified or deleted are taken from it; each one is
/// first removed from every entry kind and then re-added with its full
/// applied entry (blob content and executable bit, or symlink target),
/// so kind and mode changes round-trip. Gitlinks keep HEAD's entry: a
/// session can never change a submodule OID from a worktree (WT-006).
fn overlay_session_delta(
    head_tree: TreeSnapshot,
    applied: &TreeSnapshot,
    diff: &SessionResult,
) -> TreeSnapshot {
    let mut tree = head_tree;
    let touched = diff
        .files_added
        .iter()
        .chain(diff.files_changed.iter())
        .chain(diff.files_deleted.iter());
    for path in touched {
        if tree.gitlinks.contains_key(path) || applied.gitlinks.contains_key(path) {
            continue;
        }
        tree.blobs.remove(path);
        tree.symlinks.remove(path);
        tree.executables.remove(path);
        if let Some(content) = applied.blobs.get(path) {
            tree.blobs.insert(path.clone(), content.clone());
            if applied.is_executable(path) {
                tree.executables.insert(path.clone());
            }
        } else if let Some(target) = applied.symlinks.get(path) {
            tree.symlinks.insert(path.clone(), target.clone());
        }
    }
    tree
}

/// Read `user.name` / `user.email` from the repo-local git config,
/// following linked-worktree `.git` gitdir pointers. A missing or
/// malformed config never fails a merge — the fspec fallback is used.
fn resolve_identity(repo_path: &Path) -> (String, String) {
    const FALLBACK_NAME: &str = "fspec";
    const FALLBACK_EMAIL: &str = "fspec@local";

    let cfg_path = local_config_path(repo_path);
    let Ok(content) = std::fs::read_to_string(&cfg_path) else {
        return (FALLBACK_NAME.into(), FALLBACK_EMAIL.into());
    };

    let mut in_user = false;
    let mut name = None;
    let mut email = None;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_user = line[1..line.len() - 1].trim() == "user";
            continue;
        }
        if in_user {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = unquote(value.trim());
                match key {
                    "name" => name = Some(value.to_string()),
                    "email" => email = Some(value.to_string()),
                    _ => {}
                }
            }
        }
    }

    (
        name.filter(|s| !s.is_empty())
            .unwrap_or_else(|| FALLBACK_NAME.into()),
        email
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| FALLBACK_EMAIL.into()),
    )
}

/// Path of the repo-local config file, following `.git` → gitdir
/// pointers for linked worktrees.
fn local_config_path(repo_path: &Path) -> PathBuf {
    let git = repo_path.join(".git");
    if git.is_file() {
        if let Ok(content) = std::fs::read_to_string(&git) {
            if let Some(line) = content.lines().find(|l| l.starts_with("gitdir:")) {
                let commdir = line["gitdir:".len()..].trim();
                return Path::new(commdir).join("config");
            }
        }
        // Unreadable pointer: fall through — the read will fail and
        // the fspec identity is used.
    }
    git.join("config")
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or(value)
}

/// Update the main repo's index for the merge delta paths only, so the
/// post-merge `git status` is clean for the merged files while the
/// user's own staged/unstaged unrelated work is left exactly as it
/// was.
fn sync_index_delta(
    repo: &gix::Repository,
    workdir: &Path,
    applied: &TreeSnapshot,
    diff: &SessionResult,
) -> Result<()> {
    use crate::GitError;
    use gix::index::entry::Mode;

    let index_path = workdir.join(".git").join("index");
    let mut index = if index_path.exists() {
        crate::index_guard::ensure_index_not_truncated(&index_path, repo.object_hash())?;
        gix::index::File::at(&index_path, repo.object_hash(), false, Default::default()).map_err(
            |e| GitError::CorruptedIndex {
                message: format!("Failed to read index: {e}"),
            },
        )?
    } else {
        gix::index::File::from_state(
            gix::index::State::new(repo.object_hash()),
            index_path.clone(),
        )
    };

    // Added / modified: (re)stage the post-copy content.
    for path in diff.files_added.iter().chain(diff.files_changed.iter()) {
        let bstr_path = BStr::new(path.as_bytes());
        // Remove a stale entry first so the entry count stays exact.
        if let Ok(pos) = index.entry_index_by_path(bstr_path) {
            index.remove_entries(|idx, _, _| idx == pos);
        }
        if applied.entry_is_blob(path) {
            let content = applied.blobs.get(path).expect("delta path is a blob");
            let blob_id = repo
                .write_blob(content)
                .map_err(|e| GitError::Other(format!("Failed to write blob: {e}")))?;
            let mode = if applied.is_executable(path) {
                Mode::FILE_EXECUTABLE
            } else {
                Mode::FILE
            };
            push_index_entry(&mut index, workdir, path, blob_id.into(), mode)?;
        } else if let Some(target) = applied.symlinks.get(path) {
            let blob_id = repo
                .write_blob(target.as_bytes())
                .map_err(|e| GitError::Other(format!("Failed to write blob: {e}")))?;
            push_index_entry(&mut index, workdir, path, blob_id.into(), Mode::SYMLINK)?;
        }
        // gitlinks are atomic (WT-006) — a session can never change a
        // submodule OID from a worktree, so nothing to stage here.
    }

    // Deleted: drop the index entries.
    for path in &diff.files_deleted {
        let bstr_path = BStr::new(path.as_bytes());
        if let Ok(pos) = index.entry_index_by_path(bstr_path) {
            index.remove_entries(|idx, _, _| idx == pos);
        }
    }

    index.sort_entries();
    let options = gix::index::write::Options::default();
    let mut index_file = std::fs::File::create(index_path)?;
    index
        .write_to(&mut index_file, options)
        .map_err(|e| GitError::Other(format!("Failed to write index: {e}")))?;

    Ok(())
}

fn push_index_entry(
    index: &mut gix::index::File,
    workdir: &Path,
    path: &str,
    blob_id: gix::ObjectId,
    mode: gix::index::entry::Mode,
) -> Result<()> {
    use crate::GitError;
    use gix::index::fs::Metadata;

    let full_path = workdir.join(path);
    let bstr_path = BStr::new(path.as_bytes());
    let metadata = Metadata::from_path_no_follow(&full_path)
        .map_err(|e| GitError::Other(format!("Failed to stat {path} for index update: {e}")))?;
    let stat = gix::index::entry::Stat::from_fs(&metadata)
        .map_err(|e| GitError::Other(format!("Failed to build index stat for {path}: {e}")))?;
    index.dangerously_push_entry(
        stat,
        blob_id,
        gix::index::entry::Flags::empty(),
        mode,
        bstr_path,
    );
    Ok(())
}
