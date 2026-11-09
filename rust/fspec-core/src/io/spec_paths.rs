//! WT-015: spec paths for fspec commands.
//!
//! Feature: spec/features/isolated-worktrees-share-the-main-checkouts-board-through-one-state-root-resolver.feature
//!
//! Thin wrappers over [`codelet_common::spec_paths`], the one module that
//! builds spec paths: shared files (the board, epics, prefixes, tags, the
//! foundation, attachments, config, schedules, virtual-hook scripts) come
//! from the project's state root, which is the main checkout when the
//! command runs in an isolated copy holding a `.fspec/state-root` pointer;
//! branch files (features, coverage, command hooks) come from the project
//! root the command was handed. A broken pointer is an error naming the
//! pointer file, never a fallback to either root.

use std::path::{Path, PathBuf};

pub use codelet_common::spec_paths::{
    features_dir_of, hooks_config_of, hooks_dir_of, is_shared_state_path, spec_dir_of,
    ProjectRoots, STATE_ROOT_POINTER,
};

use crate::error::FspecCoreError;

/// The label a state-root resolution failure is reported under.
const RESOLVE: &str = "resolve-state-root";

/// The content and state roots of the project rooted at `project_root`.
pub fn roots(project_root: &Path) -> Result<ProjectRoots, FspecCoreError> {
    ProjectRoots::resolve(project_root).map_err(|source| FspecCoreError::Io {
        command: RESOLVE,
        source,
    })
}

macro_rules! shared_paths {
    ($($(#[$doc:meta])* $name:ident;)*) => {
        $(
            $(#[$doc])*
            pub fn $name(project_root: &Path) -> Result<PathBuf, FspecCoreError> {
                Ok(roots(project_root)?.$name())
            }
        )*
    };
}

shared_paths! {
    /// The state root's `spec/` (shared files and the spec lock).
    state_spec_dir;
    /// `spec/work-units.json` at the state root.
    work_units;
    /// `spec/epics.json` at the state root.
    epics;
    /// `spec/prefixes.json` at the state root.
    prefixes;
    /// `spec/tags.json` at the state root.
    tags;
    /// `spec/TAGS.md` at the state root.
    tags_md;
    /// `spec/foundation.json` at the state root.
    foundation;
    /// `spec/foundation.json.draft` at the state root.
    foundation_draft;
    /// `spec/FOUNDATION.md` at the state root.
    foundation_md;
    /// `spec/attachments/` at the state root.
    attachments_dir;
    /// `spec/fspec-config.json` at the state root.
    config;
    /// `spec/schedules.json` at the state root.
    schedules;
    /// `spec/hooks/.virtual/` at the state root.
    virtual_hooks_dir;
}
