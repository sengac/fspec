//! Work-unit snapshot types shared by every transport (RPC-006/016).

use super::*;

/// Work unit information shared across all transports and the NAPI surface.
///
/// Field order and naming match the original NAPI definition so that the
/// `napi` feature gate can preserve the existing TypeScript shape without
/// breaking changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkUnitInfo {
    pub id: String,
    pub title: String,
    #[serde(rename = "workType")]
    pub work_type: String,
    pub status: String,
    pub description: Option<String>,
    pub estimate: Option<i32>,
    pub epic: Option<String>,
    /// RPC-014: attachment file paths (basenames are rendered in the
    /// BoardView details strip). Empty vec when the work unit has no
    /// attachments. The NAPI surface preserves the existing TS shape
    /// `attachments: string[]`.
    pub attachments: Vec<String>,
    /// RPC-016: ISO-8601 UTC timestamp of the latest entry in this work
    /// unit's `stateHistory` array (i.e. when the unit most recently
    /// changed status). `None` for legacy records without
    /// `stateHistory`. Drives the `⏩` last-changed indicator in the
    /// Rust BoardView. Additive — the TS Ink BoardView continues to
    /// derive its `lastChangedWorkUnit` from `stateHistory[last]`
    /// directly so this field is invisible on the TS side.
    #[serde(rename = "lastStateChangeAt")]
    pub last_state_change_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InterruptionReason {
    User,
    LoopDetector,
}

/// RPC-015: paired manual + automatic checkpoint counts across all work
/// units in a workspace.
///
/// Mirrors the TS interface `{ manual: number; auto: number }` from
/// `src/utils/checkpoint-index.ts` so both the existing Ink TUI (which
/// reads via the pure-JS `countCheckpoints` helper) and the new Rust
/// ratatui TUI (which reads via `FspecService::checkpoint_counts`)
/// converge on the same shape. The `napi` cfg-gate preserves the JS
/// shape so the additive `napi::count_checkpoints` export can return
/// the same type verbatim.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointCounts {
    /// Count of manual (user-created) checkpoints.
    pub manual: u32,
    /// Count of automatic (state-transition) checkpoints — those whose
    /// names contain the `-auto-` substring per
    /// `src/utils/checkpoint-index.ts::AUTO_CHECKPOINT_PATTERN`.
    pub auto: u32,
}
