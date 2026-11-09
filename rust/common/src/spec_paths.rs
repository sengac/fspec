//! WT-015: the one module that builds spec paths, and the state-root
//! resolver isolated worktrees use to share the main checkout's board.
//!
//! Feature: spec/features/isolated-worktrees-share-the-main-checkouts-board-through-one-state-root-resolver.feature
//!
//! A project root has two roots ([`ProjectRoots`]):
//!
//! - the **content root**, the directory the command runs in (resolved by
//!   [`resolve_project_root`]). Files that describe or verify the code
//!   travel with the branch there: `spec/features/**` (features and their
//!   `.coverage` files), `spec/fspec-hooks.json` and hook scripts.
//! - the **state root**, where shared registries live
//!   ([`SHARED_STATE_PATHS`]): the board and its ID counters, epics,
//!   prefixes, tags, the foundation, attachments, project config,
//!   schedules, generated virtual-hook scripts, the spec write lock and
//!   `.fspec/stage-permissions.json`.
//!
//! The two differ only for an isolated copy holding a state-root pointer,
//! `<content>/.fspec/state-root` ([`STATE_ROOT_POINTER`]), a JSON file
//! `{"stateRoot": "<absolute main root>", "sessionId": "<id>"}` that
//! codelet-git writes when it creates a session worktree. The pointer is
//! a plain file, so any copy (a git worktree, a plain copy, another VCS's
//! workspace) behaves the same; nothing here asks git. A pointer that
//! cannot be read, is not valid JSON, names a relative path, points at a
//! root holding its own pointer, or points at a root without `spec/` is an
//! error naming the pointer file: there is no fallback to either root.
//!
//! Every spec path in the workspace is built here (a source-shape test in
//! `common/tests/wt015_spec_paths.rs` fails on `.join("spec")` anywhere
//! else), and [`is_shared_state_path`] is the definition WT-017's merge
//! uses to keep shared state out of a worktree merge.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use crate::project_root::resolve_project_root;

/// The spec directory's name inside a project root.
pub const SPEC_DIR: &str = "spec";

/// The state-root pointer, relative to a content root.
pub const STATE_ROOT_POINTER: &str = ".fspec/state-root";

/// Every path shared at the state root, relative to a project root. A
/// directory entry covers everything below it.
pub const SHARED_STATE_PATHS: &[&str] = &[
    "spec/work-units.json",
    "spec/epics.json",
    "spec/prefixes.json",
    "spec/tags.json",
    "spec/TAGS.md",
    "spec/foundation.json",
    "spec/foundation.json.draft",
    "spec/FOUNDATION.md",
    "spec/attachments",
    "spec/fspec-config.json",
    "spec/schedules.json",
    "spec/schedule-log.jsonl",
    "spec/hooks/.virtual",
    "spec/.fspec-write.lock",
    ".fspec/stage-permissions.json",
    ".fspec/blocklist.json",
];

/// True when `relative` (a path relative to a project root, `/`-separated
/// or native) is shared state: one of [`SHARED_STATE_PATHS`] or below one.
#[must_use]
pub fn is_shared_state_path(relative: &Path) -> bool {
    SHARED_STATE_PATHS
        .iter()
        .any(|shared| relative.starts_with(relative_path(shared)))
}

/// `relative` (`/`-separated) as a native relative path.
fn relative_path(relative: &str) -> PathBuf {
    relative.split('/').collect()
}

/// `<dir>/spec`.
#[must_use]
pub fn spec_dir_of(dir: &Path) -> PathBuf {
    dir.join(SPEC_DIR)
}

/// `<root>/spec/features`: feature files and their coverage files.
#[must_use]
pub fn features_dir_of(root: &Path) -> PathBuf {
    root.join(relative_path("spec/features"))
}

/// `<root>/spec/fspec-hooks.json`: command hooks.
#[must_use]
pub fn hooks_config_of(root: &Path) -> PathBuf {
    root.join(relative_path("spec/fspec-hooks.json"))
}

/// `<root>/spec/hooks`: hook scripts.
#[must_use]
pub fn hooks_dir_of(root: &Path) -> PathBuf {
    root.join(relative_path("spec/hooks"))
}

/// True when `dir` holds a `spec/` directory.
#[must_use]
pub fn has_spec_dir(dir: &Path) -> bool {
    spec_dir_of(dir).is_dir()
}

/// True when `dir` holds a state-root pointer file.
#[must_use]
pub fn has_state_root_pointer(dir: &Path) -> bool {
    dir.join(relative_path(STATE_ROOT_POINTER)).is_file()
}

/// The parsed state-root pointer of a content root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StateRootPointer {
    /// Absolute path of the project root holding the shared state.
    pub state_root: PathBuf,
    /// The isolated session the copy belongs to.
    pub session_id: String,
}

/// Write `<content_root>/.fspec/state-root` naming `state_root` (made
/// absolute) and `session_id`.
pub fn write_state_root_pointer(
    content_root: &Path,
    state_root: &Path,
    session_id: &str,
) -> io::Result<()> {
    let state_root = std::path::absolute(state_root)?;
    let pointer = StateRootPointer {
        state_root,
        session_id: session_id.to_string(),
    };
    let path = content_root.join(relative_path(STATE_ROOT_POINTER));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut bytes = serde_json::to_vec_pretty(&pointer).map_err(io::Error::other)?;
    bytes.push(b'\n');
    crate::spec_lock::replace_file_atomically(&path, &bytes)
}

fn pointer_error(path: &Path, reason: impl std::fmt::Display) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid state-root pointer {}: {reason}", path.display()),
    )
}

/// The validated pointer of `content_root`, or `None` when it has none.
pub fn read_state_root_pointer(content_root: &Path) -> io::Result<Option<StateRootPointer>> {
    let path = content_root.join(relative_path(STATE_ROOT_POINTER));
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(pointer_error(&path, error)),
    };
    let pointer: StateRootPointer =
        serde_json::from_str(&raw).map_err(|error| pointer_error(&path, error))?;
    let target = &pointer.state_root;
    if !target.is_absolute() {
        return Err(pointer_error(
            &path,
            format!("stateRoot {} is not an absolute path", target.display()),
        ));
    }
    if pointer.session_id.trim().is_empty() {
        return Err(pointer_error(&path, "sessionId is empty"));
    }
    if !has_spec_dir(target) {
        return Err(pointer_error(
            &path,
            format!("stateRoot {} has no spec directory", target.display()),
        ));
    }
    if has_state_root_pointer(target) {
        return Err(pointer_error(
            &path,
            format!(
                "stateRoot {} holds its own state-root pointer (chained pointers are not followed)",
                target.display()
            ),
        ));
    }
    Ok(Some(pointer))
}

/// The content and state roots of a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRoots {
    content: PathBuf,
    state: PathBuf,
    session_id: Option<String>,
}

impl ProjectRoots {
    /// The roots of the project a command run from `cwd` operates on: the
    /// content root is [`resolve_project_root`]`(cwd)`, the state root its
    /// pointer's target (or the content root when it has no pointer).
    pub fn resolve(cwd: &Path) -> io::Result<ProjectRoots> {
        ProjectRoots::of_content_root(&resolve_project_root(cwd))
    }

    /// The roots of the project whose content root is exactly `content`.
    pub fn of_content_root(content: &Path) -> io::Result<ProjectRoots> {
        Ok(match read_state_root_pointer(content)? {
            Some(pointer) => ProjectRoots {
                content: content.to_path_buf(),
                state: pointer.state_root,
                session_id: Some(pointer.session_id),
            },
            None => ProjectRoots {
                content: content.to_path_buf(),
                state: content.to_path_buf(),
                session_id: None,
            },
        })
    }

    /// The content root (branch files).
    #[must_use]
    pub fn content(&self) -> &Path {
        &self.content
    }

    /// The state root (shared files).
    #[must_use]
    pub fn state(&self) -> &Path {
        &self.state
    }

    /// The isolated session named by the pointer, if any.
    #[must_use]
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// True when the state lives in another root (a pointer was read).
    #[must_use]
    pub fn is_isolated_copy(&self) -> bool {
        self.session_id.is_some()
    }

    /// `<root>/<relative>` for a `/`-separated path relative to a project
    /// root: shared paths under the state root, every other path under
    /// the content root.
    #[must_use]
    pub fn path(&self, relative: &str) -> PathBuf {
        let relative = relative_path(relative);
        if is_shared_state_path(&relative) {
            self.state.join(relative)
        } else {
            self.content.join(relative)
        }
    }

    fn shared(&self, relative: &str) -> PathBuf {
        self.state.join(relative_path(relative))
    }

    /// The state root's `spec/` (shared files and the spec lock).
    #[must_use]
    pub fn state_spec_dir(&self) -> PathBuf {
        spec_dir_of(&self.state)
    }

    /// The content root's `spec/` (branch files).
    #[must_use]
    pub fn content_spec_dir(&self) -> PathBuf {
        spec_dir_of(&self.content)
    }

    /// `spec/work-units.json`: the board, with its ID counters.
    #[must_use]
    pub fn work_units(&self) -> PathBuf {
        self.shared("spec/work-units.json")
    }

    /// `spec/epics.json`.
    #[must_use]
    pub fn epics(&self) -> PathBuf {
        self.shared("spec/epics.json")
    }

    /// `spec/prefixes.json`.
    #[must_use]
    pub fn prefixes(&self) -> PathBuf {
        self.shared("spec/prefixes.json")
    }

    /// `spec/tags.json`.
    #[must_use]
    pub fn tags(&self) -> PathBuf {
        self.shared("spec/tags.json")
    }

    /// `spec/TAGS.md`.
    #[must_use]
    pub fn tags_md(&self) -> PathBuf {
        self.shared("spec/TAGS.md")
    }

    /// `spec/foundation.json`.
    #[must_use]
    pub fn foundation(&self) -> PathBuf {
        self.shared("spec/foundation.json")
    }

    /// `spec/foundation.json.draft`.
    #[must_use]
    pub fn foundation_draft(&self) -> PathBuf {
        self.shared("spec/foundation.json.draft")
    }

    /// `spec/FOUNDATION.md`.
    #[must_use]
    pub fn foundation_md(&self) -> PathBuf {
        self.shared("spec/FOUNDATION.md")
    }

    /// `spec/attachments/`.
    #[must_use]
    pub fn attachments_dir(&self) -> PathBuf {
        self.shared("spec/attachments")
    }

    /// `spec/fspec-config.json`: the project config.
    #[must_use]
    pub fn config(&self) -> PathBuf {
        self.shared("spec/fspec-config.json")
    }

    /// `spec/schedules.json`.
    #[must_use]
    pub fn schedules(&self) -> PathBuf {
        self.shared("spec/schedules.json")
    }

    /// `spec/schedule-log.jsonl`.
    #[must_use]
    pub fn schedule_log(&self) -> PathBuf {
        self.shared("spec/schedule-log.jsonl")
    }

    /// `spec/hooks/.virtual/`: scripts generated for cards' virtual hooks.
    #[must_use]
    pub fn virtual_hooks_dir(&self) -> PathBuf {
        self.shared("spec/hooks/.virtual")
    }

    /// `.fspec/stage-permissions.json`: the project's stage permissions.
    #[must_use]
    pub fn stage_permissions(&self) -> PathBuf {
        self.shared(".fspec/stage-permissions.json")
    }

    /// `.fspec/blocklist.json`: the project's tool blocklist.
    #[must_use]
    pub fn blocklist(&self) -> PathBuf {
        self.shared(".fspec/blocklist.json")
    }

    /// `spec/features/`: feature files and their coverage files.
    #[must_use]
    pub fn features_dir(&self) -> PathBuf {
        features_dir_of(&self.content)
    }

    /// `spec/fspec-hooks.json`: command hooks run against the branch.
    #[must_use]
    pub fn hooks_config(&self) -> PathBuf {
        hooks_config_of(&self.content)
    }

    /// `spec/hooks/`: hook scripts.
    #[must_use]
    pub fn hooks_dir(&self) -> PathBuf {
        hooks_dir_of(&self.content)
    }
}

/// The directory whose spec lock guards writes to `spec_dir`: the state
/// root's `spec/` when `spec_dir` is the `spec/` of a content root holding
/// a pointer, otherwise `spec_dir` itself. [`crate::spec_lock::SpecLock`]
/// locks this directory, so writers in the main checkout and in every
/// worktree contend on one lock, and a broken pointer fails the
/// acquisition naming the pointer file.
pub fn lock_dir_for(spec_dir: &Path) -> io::Result<PathBuf> {
    let Some(root) = spec_dir
        .file_name()
        .filter(|name| *name == SPEC_DIR)
        .and(spec_dir.parent())
    else {
        return Ok(spec_dir.to_path_buf());
    };
    Ok(match read_state_root_pointer(root)? {
        Some(pointer) => spec_dir_of(&pointer.state_root),
        None => spec_dir.to_path_buf(),
    })
}
