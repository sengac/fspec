//! Tree snapshots: a richer replacement for the old
//! `HashMap<String, Vec<u8>>` file maps (WT-006).
//!
//! Git's tree format distinguishes entries that a flat
//! path-to-bytes map silently destroys:
//! - **blobs** — regular files (the old map)
//! - **symlinks** — stored as the *target path* text, not a file
//! - **gitlinks** — submodule OIDs, opaque and unclonable
//! - **modes** — the executable bit (git's blob space is only
//!   100644 / 100755; non-executable modes normalize to 0o644)
//!
//! `TreeSnapshot` carries all four so the diff/merge/checkpoint
//! machinery can round-trip them faithfully instead of treating
//! symlinks as deletions and flattening every file to 0o644.
//!
//! WT-006 scope note: gitlinks are treated *atomically* — they are
//! never reported as deleted, never materialized, and never touched
//! by a merge (a worktree cannot clone submodule content). Full
//! submodule round-tripping is a follow-up.

use gix::object::tree::EntryKind;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// A snapshot of a git tree or working directory, rich enough for the
/// diff/merge/checkpoint machinery to preserve kind and mode (WT-006).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TreeSnapshot {
    /// Regular file blobs: repo-relative path (forward slashes) → content.
    pub blobs: HashMap<String, Vec<u8>>,
    /// Symlinks: repo-relative path → target path string.
    pub symlinks: HashMap<String, String>,
    /// Gitlinks (submodules): repo-relative path → submodule commit OID.
    pub gitlinks: HashMap<String, gix::ObjectId>,
    /// Paths whose blob carries the executable bit (100755).
    pub executables: HashSet<String>,
    /// Gitignored untracked paths found during a working-directory
    /// collection (WT-006): surfaced for honest reporting, never part
    /// of the tracked-change set, never diffed, and never merged.
    pub ignored: Vec<String>,
}

impl TreeSnapshot {
    /// Empty snapshot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the snapshot has any git-representable content.
    pub fn is_empty(&self) -> bool {
        self.blobs.is_empty() && self.symlinks.is_empty() && self.gitlinks.is_empty()
    }

    /// Whether a blob path is marked executable.
    pub fn is_executable(&self, path: &str) -> bool {
        self.executables.contains(path)
    }

    /// Whether a path is present in any git-representable kind
    /// (blob, symlink, gitlink).
    pub fn contains_path(&self, path: &str) -> bool {
        self.blobs.contains_key(path)
            || self.symlinks.contains_key(path)
            || self.gitlinks.contains_key(path)
    }

    /// The git-representable "content" of a path: the blob bytes for
    /// regular files, the target string for symlinks, the hex OID for
    /// gitlinks. `None` when the path is absent.
    pub fn entry_bytes(&self, path: &str) -> Option<Vec<u8>> {
        if let Some(b) = self.blobs.get(path) {
            return Some(b.clone());
        }
        if let Some(t) = self.symlinks.get(path) {
            return Some(t.as_bytes().to_vec());
        }
        if let Some(oid) = self.gitlinks.get(path) {
            return Some(oid.to_string().as_bytes().to_vec());
        }
        None
    }

    /// Whether the path's entry is a regular blob (not symlink/gitlink).
    pub fn entry_is_blob(&self, path: &str) -> bool {
        self.blobs.contains_key(path)
    }

    /// All git-representable paths (blobs ∪ symlinks ∪ gitlinks),
    /// sorted and de-duplicated.
    pub fn all_paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self
            .blobs
            .keys()
            .chain(self.symlinks.keys())
            .chain(self.gitlinks.keys())
            .cloned()
            .collect();
        paths.sort();
        paths.dedup();
        paths
    }

    /// Merge `other` into `self`, with `other` winning on any path
    /// overlap (used when folding a working directory over an index
    /// snapshot). The `ignored` list is unioned.
    pub fn extend(&mut self, other: TreeSnapshot) {
        let TreeSnapshot {
            blobs,
            symlinks,
            gitlinks,
            executables,
            ignored,
        } = other;
        self.blobs.extend(blobs);
        self.symlinks.extend(symlinks);
        self.gitlinks.extend(gitlinks);
        self.executables.extend(executables);
        for path in ignored {
            if !self.ignored.contains(&path) {
                self.ignored.push(path);
            }
        }
    }
}

/// The executable bit for a checked-out/merged file: `true` when the
/// source snapshot marks the path executable.
///
/// git's mode space for blobs is 100644 (0o644) and 100755 (0o755);
/// any other on-disk mode normalizes to 0o644 by convention, so the
/// bit is the full mode-relevant surface.
#[cfg(unix)]
fn file_mode(executable: bool) -> u32 {
    if executable {
        0o755
    } else {
        0o644
    }
}

/// Write a regular file to `workdir.join(path)`, preserving
/// parent-directory creation and the executable bit (WT-006).
///
/// On Unix the executable bit comes from `executable`; on Windows the
/// mode is a no-op.
pub fn write_file_with_mode(
    workdir: &Path,
    path: &str,
    content: &[u8],
    #[cfg_attr(not(unix), allow(unused_variables))] executable: bool,
) -> std::io::Result<()> {
    let full_path = workdir.join(path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&full_path, content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = file_mode(executable);
        std::fs::set_permissions(&full_path, std::fs::Permissions::from_mode(mode))?;
    }
    #[allow(clippy::let_and_return)]
    Ok(())
}

/// Read the executable bit of an on-disk file (false when absent or
/// non-unix).
#[cfg(unix)]
pub fn is_file_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
pub fn is_file_executable(_path: &Path) -> bool {
    false
}

/// Whether a tree entry's kind is a regular (possibly executable) blob.
pub fn is_blob_kind(kind: EntryKind) -> bool {
    matches!(kind, EntryKind::Blob | EntryKind::BlobExecutable)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Feature: spec/features/session-merge-diff-machinery-is-git-blind-ignored-files-symlinks-and-file-modes.feature
    // (unit-level pins for the snapshot type the scenarios exercise)

    #[test]
    fn snapshot_records_all_four_entry_kinds() {
        let mut snap = TreeSnapshot::new();
        snap.blobs.insert("a.txt".to_string(), b"hello".to_vec());
        snap.symlinks
            .insert("links/x".to_string(), "a.txt".to_string());
        snap.executables.insert("bin/tool".to_string());
        assert!(!snap.is_empty());
        assert!(snap.is_executable("bin/tool"));
        assert!(!snap.is_executable("a.txt"));
    }

    #[test]
    fn snapshot_extend_wins_on_overlap_and_unions_ignored() {
        let mut base = TreeSnapshot::new();
        base.blobs.insert("a.txt".to_string(), b"v1".to_vec());
        base.ignored.push("build/old".to_string());

        let mut other = TreeSnapshot::new();
        other.blobs.insert("a.txt".to_string(), b"v2".to_vec());
        other.ignored.push("build/new".to_string());

        base.extend(other);
        assert_eq!(base.blobs.get("a.txt"), Some(&b"v2".to_vec()));
        assert_eq!(
            base.ignored,
            vec!["build/old".to_string(), "build/new".to_string()]
        );
    }

    #[test]
    fn write_file_with_mode_sets_executable_bit() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = "bin/tool";
        write_file_with_mode(tmp.path(), path, b"#!/bin/sh\n", true).expect("write");
        let full = tmp.path().join(path);
        #[cfg(unix)]
        assert!(
            is_file_executable(&full),
            "executable bit must be set by write_file_with_mode"
        );
        #[cfg(not(unix))]
        assert!(full.exists());

        write_file_with_mode(tmp.path(), "plain.txt", b"data", false).expect("write");
        let plain = tmp.path().join("plain.txt");
        #[cfg(unix)]
        assert!(
            !is_file_executable(&plain),
            "non-executable files must land with 0o644"
        );
    }
}
