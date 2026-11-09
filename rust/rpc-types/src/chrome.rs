//! AgentView chrome types: model info, thinking level, workspace info and entries (RPC-018, RPC-443, RPC-445).

use super::*;

// ============================================================================
// RPC-018: AgentView chrome shared types (ModelInfo, ThinkingLevel,
// WorkspaceInfo) consumed by the new `get_model_info`,
// `get_thinking_level`, `get_workspace_info` RPC methods.
// ============================================================================

/// Capability + display metadata for the model attached to a session.
///
/// Mirrors the TS `useModelStore.getCapabilities` shape so the SessionHeader
/// in the Rust ratatui TUI can paint `[R]` / `[V]` / `[Nk]` badges using
/// the SAME source of truth that the Ink SessionHeader.tsx consumes.
///
/// `Default` returns an "unknown model" sentinel — `display_name` is empty,
/// every capability is false, and both limits are `None`. The SessionHeader
/// widget hides empty badges so the UI degrades gracefully when no
/// session manager is attached (the RPC-018 default-impl path).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelInfo {
    pub display_name: String,
    pub supports_reasoning: bool,
    pub supports_vision: bool,
    /// PROV-166: `None` when the context window is unavailable.
    pub context_window: Option<u32>,
    /// Compaction threshold (tokens) — the point at which context compaction
    /// triggers. The SessionHeader size badge prefers this over
    /// `context_window` (mirrors TS `compactionThreshold ?? contextWindow` in
    /// `SessionHeader.tsx`). PROV-166: `None` when unavailable.
    pub compaction_threshold: Option<u32>,
}

/// Per-session thinking/reasoning level, as chosen (MODEL-011): any level
/// the session's model offers, with nothing remapped. A named effort is the
/// model's own name for it (`low`, `xhigh`, ...); the closed
/// Off/Low/Medium/High set is gone (no legacy mapping).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum ThinkingLevel {
    /// Thinking off, in the model's own off form.
    #[default]
    Off,
    /// BUG-455: no level chosen — no thinking parameter is sent and the
    /// model's own default applies.
    ModelDefault,
    /// MODEL-016: a toggle-only model's thinking switched on.
    On,
    /// A named effort the model advertises.
    Effort(String),
    /// MODEL-016: a numeric effort, sent as a JSON number; it crosses the
    /// binary wire as a tagged number (BUG-594).
    Numeric(
        #[serde(with = "wire_number")]
        #[schemars(with = "serde_json::Number")]
        serde_json::Number,
    ),
    /// A token budget.
    Budget(u64),
}

/// Workspace snapshot — cwd + git branch — returned by
/// `FspecService::get_workspace_info`. Mirrors the data the TS
/// `SessionFooter.tsx` derives from a Rust-side `FooterStateUpdate`
/// poller; the new RPC method is a single-shot pull alternative that the
/// Rust ratatui SessionFooter widget consumes via `Action::WorkspaceInfoLoaded`.
///
/// `cwd` is returned RAW (not `~`-shortened) — the SessionFooter widget
/// performs the `home::home_dir()` substitution at render time so the
/// wire shape stays portable across hosts whose `$HOME` differs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub cwd: String,
    pub git_branch: Option<String>,
}

/// RPC-443: one workspace a daemon serves (canonical path, folder-name
/// display name, and whether it is the daemon's start workspace).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    pub path: String,
    pub name: String,
    pub start: bool,
}

/// RPC-445: a work-units snapshot for a workspace opened besides the
/// daemon's start workspace, tagged with its canonical path.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceWorkUnits {
    pub workspace: String,
    pub units: Vec<WorkUnitInfo>,
}

/// RPC-454: a git-state snapshot for a workspace opened besides the
/// daemon's start workspace, tagged with its canonical path.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceGitState {
    pub workspace: String,
    pub state: crate::GitState,
}
