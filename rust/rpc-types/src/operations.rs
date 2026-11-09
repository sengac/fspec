//! Blocklist, merge-worktree, schedule and loop wire payloads (RPC-056..059).

use super::*;

// ============================================================================
// RPC-056: Blocklist rule wire payload.
// ============================================================================

/// One row in the `/blocklist` view, surfaced by the `blocklist_list`
/// RPC method. Mirrors the TS `BlocklistRule` interface
/// (`src/tui/components/BlocklistListView.tsx` lines 20-34) field-for-field
/// with the addition of the explicit `source` provenance tag — the TS
/// frontend stamps `'system' | 'project'` on each rule client-side after
/// loading from `blocklistLoad(cwd)`; the Rust pipeline carries the
/// provenance over the wire so the frontend does not need to know how to
/// split the two configs.
///
/// `action` is one of `"block" | "allow" | "prompt"` (matches
/// `BlocklistAction` lowercased serde tag).
/// `source` is one of `"system" | "project"`.
/// `guidance` is `None` when the rule has no educational follow-up.
///
/// Wire shape is a flat struct so `napi_derive::napi(object)` stays valid;
/// no discriminated enums.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlocklistRuleInfo {
    pub id: String,
    pub pattern: String,
    /// "block" | "allow" | "prompt"
    pub action: String,
    pub reason: String,
    pub guidance: Option<String>,
    /// "system" | "project"
    pub source: String,
}

// ============================================================================
// RPC-057: /merge-worktree wire payloads.
// ============================================================================

/// The merge algorithm to use. Currently the codelet-git layer only
/// supports a single fast-forward-style merge; `Squash` and `ThreeWay`
/// are reserved on the wire so future cards can add support without
/// breaking the trait surface. Serialised as a snake_case string so
/// napi consumers receive plain strings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(schemars::JsonSchema)]
pub enum MergeStrategy {
    #[default]
    FastForward,
    Squash,
    ThreeWay,
}

/// Result status of a merge attempt. `Success` and `NoChanges` are
/// terminal outcomes; `Conflict` carries the conflicting file paths in
/// the surrounding `MergeOutcome` so the LLM can be seeded with a
/// context message and asked to resolve them. Serialised as a
/// snake_case string for napi compatibility.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStatus {
    Success,
    Conflict,
    #[default]
    NoChanges,
}

/// Outcome of a `merge_session_worktree` RPC call.
///
/// * `status` — terminal state classification.
/// * `conflicts` — non-empty only when `status == Conflict`; lists the
///   relative paths whose merge produced a conflict.
/// * `merge_commit` — short SHA of the resulting merge commit if the
///   underlying codelet-git layer surfaces one; `None` otherwise.
/// * `worktree_path` — WT-009: absolute path of the session's worktree
///   (the resolved repo root + `.fspec/worktrees/<id>`), `None` when the
///   session has no resolvable worktree path. Lets the TUI print the
///   "Effective worktree: ..." conflict footer against a real path the
///   LLM can act on, instead of the session UUID.
///
/// Wire shape is a flat struct so `napi_derive::napi(object)` stays
/// valid; no discriminated enums.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeOutcome {
    pub status: MergeStatus,
    pub conflicts: Vec<String>,
    pub merge_commit: Option<String>,
    pub worktree_path: Option<String>,
}

/// One row in the `list_session_worktrees` RPC response. Mirrors
/// `codelet_git::WorktreeInfo` plus the `derive_session_status` +
/// dirty heuristic computed from a non-empty `get_session_diff`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionWorktreeInfo {
    pub session_id: SessionId,
    pub worktree_path: String,
    pub base_commit: String,
    pub head_commit: String,
    pub dirty: bool,
}

/// Summary payload returned by `inspect_session_changes`. The
/// MergeConfirmDialog renders the counts inline (e.g. "1 file changed,
/// +4 / -2, 1 commit") so the user has explicit feedback before
/// confirming a destructive merge.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionChangesSummary {
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
    /// Short SHAs (typically 7 chars) of every commit on the session
    /// branch that is not yet on the base branch.
    pub commits: Vec<String>,
    /// WT-006: count of gitignored untracked files in the session
    /// worktree. Never part of the tracked change set — surfaced so the
    /// TUI can stay honest when "nothing to merge" hides ignored
    /// build artifacts.
    pub files_ignored: u32,
}

// ============================================================================
// RPC-058: /schedule wire payloads.
// ============================================================================

/// Flat wire shape for a single scheduled job (agent or shell). Mirrors
/// `ScheduleEntry` from rust/napi/src/scheduler/types.rs but flattens
/// the nested agent/shell config into top-level `role`/`prompt`/`command`
/// so the struct stays `napi_derive::napi(object)`-compatible (no
/// discriminated enums on the wire).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ScheduledJob {
    pub name: String,
    pub cron: String,
    pub timezone: String,
    /// "agent" | "shell"
    pub job_type: String,
    /// "active" | "paused"
    pub status: String,
    pub created_at: Option<String>,
    pub last_run_at: Option<String>,
    pub last_run_status: Option<String>,
    /// Agent-only — role text injected on session spawn.
    pub role: Option<String>,
    /// Agent-only — initial prompt sent to the spawned session.
    pub prompt: Option<String>,
    /// Shell-only — the command line to spawn.
    pub command: Option<String>,
    /// "skip" | "queue" (when None, the engine treats as "skip")
    pub overlap_policy: Option<String>,
}

// ============================================================================
// RPC-059: /loop wire payloads.
// ============================================================================

/// Flat wire shape for a single registered session-scoped loop. Mirrors
/// the internal `codelet_core::loops::LoopEntry` but flattens the
/// `chrono::DateTime<Utc>` fields into RFC-3339 String timestamps and
/// wraps the session UUID in a `SessionId` so the struct stays
/// `napi_derive::napi(object)`-compatible (no chrono on the JS side).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredLoop {
    pub id: String,
    pub session_id: SessionId,
    pub prompt: String,
    pub interval_seconds: u32,
    /// RFC-3339 UTC timestamp.
    pub created_at: String,
    /// RFC-3339 UTC timestamp.
    pub expires_at: String,
    /// RFC-3339 UTC timestamp, or `None` if the loop has not fired yet.
    pub last_run_at: Option<String>,
}
