//! Snapshot-aware session diff, conflict detection, and apply helpers
//! (WT-006).
//!
//! All three-way session operations now compare [`TreeSnapshot`]s
//! instead of flat `HashMap<String, Vec<u8>>` maps so that symlinks
//! (by target), gitlinks (atomically — never deleted, never applied),
//! and the executable bit round-trip faithfully.
//!
//! WT-006 invariants enforced here:
//! - gitignored files (`TreeSnapshot::ignored`) are never compared,
//!   never diffed, and never applied to main;
//! - base gitlinks are never reported as deleted and never applied;
//! - symlinks are compared/applied by their target string;
//! - applied blobs preserve the executable bit from the worktree
//!   snapshot (falling back to the base tree's bit when unchanged).

use crate::error::Result;
use crate::tree_snapshot::{write_file_with_mode, TreeSnapshot};
use crate::utils::is_binary_content;
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::path::Path;

/// Compute the tracked-change lists and unified diff between a base
/// tree and a worktree snapshot (WT-006).
///
/// Returns `(files_changed, files_added, files_deleted, diff_text)`.
/// Gitignored paths in `worktree.ignored` and base gitlinks are
/// excluded from all lists and from the diff.
pub fn compute_session_diff(
    base: &TreeSnapshot,
    worktree: &TreeSnapshot,
) -> (Vec<String>, Vec<String>, Vec<String>, String) {
    let mut files_changed = Vec::new();
    let mut files_added = Vec::new();
    let mut files_deleted = Vec::new();
    let mut diff_parts = Vec::new();

    // Modified and deleted entries (base is authoritative for kind).
    for path in base.all_paths() {
        if base.gitlinks.contains_key(&path) {
            continue; // gitlinks are atomic (WT-006)
        }
        let base_content = base
            .entry_bytes(&path)
            .expect("path iterated from base snapshot");
        let base_is_blob = base.entry_is_blob(&path);
        match worktree.entry_bytes(&path) {
            Some(work_content) if work_content != base_content.as_slice() => {
                files_changed.push(path.clone());
                diff_parts.push(generate_file_diff(
                    &path,
                    &base_content,
                    &work_content,
                    base_is_blob,
                    worktree.entry_is_blob(&path),
                ));
            }
            Some(_work_content) => {
                // Content identical — but the executable bit is part of
                // blob identity in git (WT-006): a mode-only change is a
                // real change that must be reported and applied.
                let mode_changed = base_is_blob
                    && worktree.entry_is_blob(&path)
                    && base.is_executable(&path) != worktree.is_executable(&path);
                if mode_changed {
                    files_changed.push(path.clone());
                    diff_parts.push(format!(
                        "mode change {}: {} -> {}\n",
                        path,
                        mode_str(base.is_executable(&path)),
                        mode_str(worktree.is_executable(&path))
                    ));
                }
            }
            None => {
                files_deleted.push(path.clone());
                diff_parts.push(generate_delete_diff(&path, &base_content, base_is_blob));
            }
        }
    }

    // Added entries (worktree side only; never in base by kind).
    for path in worktree.all_paths() {
        if base.contains_path(&path) {
            continue;
        }
        let content = worktree
            .entry_bytes(&path)
            .expect("path iterated from worktree snapshot");
        if worktree.entry_is_blob(&path) || worktree.symlinks.contains_key(&path) {
            files_added.push(path.clone());
            diff_parts.push(generate_add_diff(
                &path,
                &content,
                worktree.entry_is_blob(&path),
            ));
        } else {
            // A gitlink the worktree cannot have cloned: report it as a
            // changed, unmergeable entry so it does not vanish silently.
            files_changed.push(path.clone());
            diff_parts.push(format!(
                "submodule {path} changed (gitlink) — not merged atomically (WT-006)"
            ));
        }
    }

    files_changed.sort();
    files_added.sort();
    files_deleted.sort();

    (
        files_changed,
        files_added,
        files_deleted,
        diff_parts.join("\n"),
    )
}

/// Apply a worktree snapshot to the main working directory (WT-006).
///
/// Copies modified/added blobs with the executable bit, recreates
/// symlinks, and removes deleted blobs/symlinks. Gitlinks and
/// gitignored files are never touched.
pub fn apply_snapshot_to_main(
    base: &TreeSnapshot,
    worktree: &TreeSnapshot,
    main_workdir: &Path,
) -> Result<()> {
    // Copy modified/added blobs (content change OR executable-bit change —
    // git's blob identity includes the mode, WT-006).
    for (path, worktree_content) in &worktree.blobs {
        let is_changed = base
            .blobs
            .get(path)
            .map(|b| b != worktree_content)
            .unwrap_or(true)
            || (base.blobs.contains_key(path)
                && base.is_executable(path) != worktree.is_executable(path));

        if is_changed {
            // Preserve the executable bit: the worktree's bit wins when
            // present in either snapshot (an unchanged 100755 file from
            // the base tree must stay 100755 in main).
            let executable = worktree.is_executable(path) || base.is_executable(path);
            write_file_with_mode(main_workdir, path, worktree_content, executable)?;
        }
    }

    // Copy modified/added symlinks
    for (path, worktree_target) in &worktree.symlinks {
        let is_changed = base
            .symlinks
            .get(path)
            .map(|t| t != worktree_target)
            .unwrap_or(true);

        if is_changed {
            let full_path = main_workdir.join(path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }
            #[cfg(unix)]
            {
                if full_path.symlink_metadata().is_ok() {
                    fs::remove_file(&full_path)?;
                }
                std::os::unix::fs::symlink(worktree_target, &full_path)?;
            }
            #[cfg(windows)]
            fs::write(&full_path, worktree_target)?;
        }
    }

    // Remove deleted blobs and symlinks (gitlinks never — atomic)
    for path in base.all_paths() {
        if base.gitlinks.contains_key(&path) {
            continue;
        }
        if !worktree.contains_path(&path) {
            remove_path_quiet(main_workdir, &path);
        }
    }

    Ok(())
}

/// Detect conflicts between session and main worktree changes
/// (WT-006 snapshot version of the byte-map helper).
///
/// A conflict is: a base path changed in both the worktree and main
/// since the base, or a file added in the worktree that exists in
/// main with different content. Gitlinks and gitignored files are
/// excluded.
pub fn detect_conflicts(
    base: &TreeSnapshot,
    worktree: &TreeSnapshot,
    main: &TreeSnapshot,
) -> Vec<String> {
    let mut conflicts = Vec::new();

    // Modified-in-both check for every base entry.
    for path in base.all_paths() {
        if base.gitlinks.contains_key(&path) {
            continue;
        }
        let base_content = base
            .entry_bytes(&path)
            .expect("path iterated from base snapshot");
        let session_changed = match worktree.entry_bytes(&path) {
            Some(c) => c.as_slice() != base_content.as_slice(),
            None => true, // deleted counts as changed
        };
        let main_changed = match main.entry_bytes(&path) {
            Some(c) => c.as_slice() != base_content.as_slice(),
            None => false, // main deleted it — not a conflict (session's delete wins)
        };

        if session_changed && main_changed {
            conflicts.push(path);
        }
    }

    // Added files that exist in main with different content.
    for path in worktree.all_paths() {
        if base.contains_path(&path) {
            continue;
        }
        if worktree.gitlinks.contains_key(&path) {
            continue; // atomic — never conflict, never apply
        }
        let session_content = worktree
            .entry_bytes(&path)
            .expect("path iterated from worktree snapshot");
        let main_content = main.entry_bytes(&path);
        if main_content.is_some() && main_content.as_deref() != Some(session_content.as_slice()) {
            conflicts.push(path);
        }
    }

    conflicts.sort();
    conflicts.dedup();
    conflicts
}

/// Render a blob's git mode (100644 / 100755) for mode-change diff lines.
fn mode_str(executable: bool) -> &'static str {
    if executable {
        "100755"
    } else {
        "100644"
    }
}

/// Best-effort remove of `main_workdir.join(path)` (blob or symlink).
fn remove_path_quiet(main_workdir: &Path, path: &str) {
    let dest = main_workdir.join(path);
    if dest.symlink_metadata().is_ok() {
        let _ = fs::remove_file(&dest);
    }
}

// =============================================================================
// Diff generation helpers
// =============================================================================

/// Generate unified diff for a modified file (WT-006: kind-aware).
fn generate_file_diff(
    path: &str,
    old_content: &[u8],
    new_content: &[u8],
    old_is_blob: bool,
    new_is_blob: bool,
) -> String {
    // Symlinks: diff the target strings (they are text by definition).
    if !old_is_blob || !new_is_blob {
        let old_str = String::from_utf8_lossy(old_content);
        let new_str = String::from_utf8_lossy(new_content);
        let diff = TextDiff::from_lines(old_str.as_ref(), new_str.as_ref());
        let mut lines = vec![format!("--- a/{}", path), format!("+++ b/{}", path)];
        for change in diff.iter_all_changes() {
            let prefix = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            lines.push(format!(
                "{}{}",
                prefix,
                change.value().trim_end_matches('\n')
            ));
        }
        return lines.join("\n");
    }

    if is_binary_content(old_content) || is_binary_content(new_content) {
        return format!("Binary file {} changed\n", path);
    }

    let old_str = String::from_utf8_lossy(old_content);
    let new_str = String::from_utf8_lossy(new_content);
    let diff = TextDiff::from_lines(old_str.as_ref(), new_str.as_ref());

    let mut lines = vec![format!("--- a/{}", path), format!("+++ b/{}", path)];
    for change in diff.iter_all_changes() {
        let prefix = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        lines.push(format!(
            "{}{}",
            prefix,
            change.value().trim_end_matches('\n')
        ));
    }

    lines.join("\n")
}

/// Generate diff for a deleted file (WT-006: kind-aware).
fn generate_delete_diff(path: &str, content: &[u8], is_blob: bool) -> String {
    if !is_blob {
        let target = String::from_utf8_lossy(content);
        return format!(
            "deleted symlink {} -> {}\n",
            path,
            target.trim_end_matches('\n')
        );
    }

    if is_binary_content(content) {
        return format!("Binary file {} deleted\n", path);
    }

    let content_str = String::from_utf8_lossy(content);
    let mut lines = vec![format!("--- a/{}", path), "+++ /dev/null".to_string()];

    for line in content_str.lines() {
        lines.push(format!("-{}", line));
    }

    lines.join("\n")
}

/// Generate diff for an added file (WT-006: kind-aware).
fn generate_add_diff(path: &str, content: &[u8], is_blob: bool) -> String {
    if !is_blob {
        let target = String::from_utf8_lossy(content);
        return format!(
            "new symlink {} -> {}\n",
            path,
            target.trim_end_matches('\n')
        );
    }

    if is_binary_content(content) {
        return format!("Binary file {} added\n", path);
    }

    let content_str = String::from_utf8_lossy(content);
    let mut lines = vec!["--- /dev/null".to_string(), format!("+++ b/{}", path)];

    for line in content_str.lines() {
        lines.push(format!("+{}", line));
    }

    lines.join("\n")
}
