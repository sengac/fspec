//! BUG-497: project root resolution shared by fspec-core (BUG-523), the
//! common config code and codelet-core, so every reader and writer of
//! `<root>/spec` resolves the same root from a working directory.
//!
//! Rust port of `src/utils/project-root-detection.ts`: walk upward from a
//! starting directory looking for project-boundary markers.

use std::path::{Path, PathBuf};

use crate::spec_paths::{has_spec_dir, has_state_root_pointer, spec_dir_of};

/// Files / directories that mark a project root. Mirrors the TS
/// `BOUNDARY_MARKERS` constant.
const BOUNDARY_MARKERS: &[&str] = &[
    ".git",
    "package.json",
    ".gitignore",
    "Cargo.toml",
    "pyproject.toml",
];

/// Maximum directories to walk upward. Matches TS `MAX_SEARCH_DEPTH = 10`.
const MAX_SEARCH_DEPTH: u32 = 10;

/// The project root a command run from `cwd` operates on: the directory
/// whose `spec/` [`find_or_create_spec_directory`] uses. Creates nothing.
///
/// 1. `cwd` itself when `cwd/spec` is a directory (test isolation).
/// 2. Otherwise the parent of the nearest existing `spec/` inside a project
///    boundary, walking upward.
/// 3. Otherwise the nearest directory holding a boundary marker (where
///    `spec/` would be created), or `cwd` when there is none.
///
/// BUG-523: commands build their write paths from `<root>/spec` while the
/// loaders and the spec lock resolve the spec directory by walking up, so
/// every front door (the fspec CLI bridges, `dispatch_command`) hands
/// commands this root rather than the raw working directory. Run from
/// `rust/`, `update-work-unit-status` otherwise read and locked the
/// repository's `spec/` and wrote a new `rust/spec/work-units.json`.
///
/// WT-015: a directory holding a state-root pointer (an isolated copy of a
/// project, [`crate::spec_paths::STATE_ROOT_POINTER`]) is a project root
/// of its own: the walk stops there, so a copy without a `spec/` of its
/// own never resolves to the `spec/` of a project it sits inside (the main
/// checkout holding `.fspec/worktrees/`).
pub fn resolve_project_root(cwd: &Path) -> PathBuf {
    if has_spec_dir(cwd) {
        return cwd.to_path_buf();
    }
    if let Some(root) = find_spec_or_pointer_root(cwd) {
        return root;
    }
    find_project_root(cwd)
}

/// The nearest directory, walking upward from `cwd`, that holds an existing
/// `spec/` inside a project boundary or a state-root pointer.
fn find_spec_or_pointer_root(cwd: &Path) -> Option<PathBuf> {
    let mut current = cwd.to_path_buf();
    let mut depth = 0u32;

    while depth < MAX_SEARCH_DEPTH {
        if has_spec_dir(&current) && has_project_boundary_marker(&current) {
            return Some(current);
        }
        if has_state_root_pointer(&current) {
            return Some(current);
        }
        match current.parent() {
            Some(parent) if parent != current => {
                current = parent.to_path_buf();
                depth += 1;
            }
            _ => break,
        }
    }

    None
}

/// Searches upward for an existing `spec/` directory inside a project
/// boundary, stopping at a directory holding a state-root pointer (WT-015).
pub fn find_existing_spec_directory(cwd: &Path) -> Option<PathBuf> {
    let mut current = cwd.to_path_buf();
    let mut depth = 0u32;

    while depth < MAX_SEARCH_DEPTH {
        let candidate = spec_dir_of(&current);
        if candidate.is_dir() && has_project_boundary_marker(&current) {
            return Some(candidate);
        }
        if has_state_root_pointer(&current) {
            return None;
        }

        match current.parent() {
            Some(parent) if parent != current => {
                current = parent.to_path_buf();
                depth += 1;
            }
            _ => break,
        }
    }

    None
}

/// Walks upward for the nearest directory containing any boundary marker.
/// Returns `cwd` if nothing found (matches TS fallback).
pub fn find_project_root(cwd: &Path) -> PathBuf {
    let mut current = cwd.to_path_buf();
    let mut depth = 0u32;

    while depth < MAX_SEARCH_DEPTH {
        if has_project_boundary_marker(&current) {
            return current;
        }

        match current.parent() {
            Some(parent) if parent != current => {
                current = parent.to_path_buf();
                depth += 1;
            }
            _ => break,
        }
    }

    cwd.to_path_buf()
}

/// Returns true if `dir` contains any boundary marker file/directory.
pub fn has_project_boundary_marker(dir: &Path) -> bool {
    BOUNDARY_MARKERS
        .iter()
        .any(|marker| dir.join(marker).exists())
}
