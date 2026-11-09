//! Git state, changed-file, checkpoint and checkpoint-progress wire types (BUG-182, RPC-355, RPC-362, TUI-109).

use super::*;

// ============================================================================
// BUG-182: GitState wire type (the ONE centralized git polling frame)
// ============================================================================

/// BUG-182: one `GitState` snapshot broadcast by codelet-core's
/// `GitStateWatcher` on every debounced `.git` fs-event AND every 10-second
/// poll tick — ONLY when the snapshot differs from the last published one
/// (dedup). Replaces the BUG-181 `CheckpointCounts`-only push: the
/// checkpoint counts now ride this frame, alongside the chrome-bar branch,
/// the changed-files list, and the checkpoint list.
///
/// `git_branch` is `None` on a non-repo cwd (the SessionFooter degrades to
/// a bare-cwd render, mirroring RPC-018's degraded contract).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitState {
    /// Manual + auto checkpoint counts (BUG-181 contract).
    pub checkpoint_counts: CheckpointCounts,
    /// Current branch name, or `None` when detached / not a git repo.
    pub git_branch: Option<String>,
    /// Changed working-tree files (staged + unstaged + untracked).
    pub changed_files: Vec<ChangedFile>,
    /// Checkpoint list, sorted most-recent-first, capped at 200.
    pub checkpoints: Vec<CheckpointInfo>,
}

// ============================================================================
// RPC-355: Changed-file wire type
// ============================================================================

/// One changed file in the working tree (RPC-355).
///
/// `change_type` is a single-letter `String` ("A" | "M" | "D" | "R") to stay
/// serde/tarpc-friendly; the UI maps the letter to a color. Mirrors the TS
/// `FileItem` shape consumed by `ChangedFilesViewer`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangedFile {
    /// Repo-relative path.
    pub path: String,
    /// Single-letter change type: "A" | "M" | "D" | "R".
    #[serde(rename = "changeType")]
    pub change_type: String,
    /// true = staged (index), false = unstaged/untracked working-tree change.
    pub staged: bool,
}

// ============================================================================
// RPC-362: Checkpoint info wire type
// ============================================================================

/// One ghost-commit checkpoint surfaced to the Rust TUI CheckpointsView
/// (RPC-362).
///
/// Carries the `work_unit_id` + `name` pair needed to resolve the
/// `refs/fspec-checkpoints/<work_unit_id>/<name>` ref for diff/restore/delete,
/// the original creation `timestamp` recovered from the metadata index, and
/// the derived `is_automatic` flag (`name` contains the `-auto-` substring).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointInfo {
    /// Work unit identifier the checkpoint belongs to (ref namespace).
    #[serde(rename = "workUnitId")]
    pub work_unit_id: String,
    /// Checkpoint name (the final ref path segment).
    pub name: String,
    /// ISO-8601 creation timestamp recovered from the metadata index,
    /// or a fallback "now" when the index is missing/malformed.
    pub timestamp: String,
    /// true when `name` contains the `-auto-` automatic-checkpoint pattern.
    #[serde(rename = "isAutomatic")]
    pub is_automatic: bool,
}

// ============================================================================
// TUI-109: Checkpoint enumeration progress wire type
// ============================================================================

/// One per-item progress frame emitted while `FspecService::list_checkpoints`
/// enumerates checkpoints (TUI-109).
///
/// The total is only known after enumeration completes, so intermediate
/// frames carry `total: 0` (the TUI renders a pending total as
/// `"(loaded/…)"`); the final `done: true` frame carries the full
/// enumeration count. The final `Vec<CheckpointInfo>` still returns via
/// the same RPC — this type only feeds the loading dialog's counter row.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointsProgress {
    /// Checkpoints collected so far (capped at the TUI's display limit).
    /// `u32` (not `usize`) so the napi-gated wire shape compiles —
    /// checkpoint counts are small and this mirrors `CheckpointCounts`.
    pub loaded: u32,
    /// Full enumeration count (0 until enumeration completes).
    pub total: u32,
    /// true on the final frame of the enumeration.
    pub done: bool,
}
