//! done Tool — CONT-002 auto-continue + CONT-003 goal completion contract
//!
//! Feature: spec/features/auto-continue-engine.feature
//! Feature: spec/features/goal-enforcement.feature
//!
//! `done(summary)` is the explicit completion signal for the auto-continue
//! Completion Contract. It follows the session-scoped registry pattern of
//! `inject_summary` (see `INJECT_SUMMARY_HANDLERS`, inject_summary.rs:90-98):
//! - a per-session contract registry (`CONTRACT_STATE`) gates conditional
//!   registration in the seven `create_rig_agent` builder chains AND carries
//!   the CONT-003 goal (text + optional verify command) synced from
//!   `Session.goal` at the same dispatch sites as the armed flag, and
//! - a per-session ACCEPTANCE registry records "done() was called and accepted
//!   this turn-sequence" so the stream loop can read it at the FinalResponse
//!   settle point (stream_loop.rs).
//!
//! CONT-002 (no goal): acceptance is Tier 0 (face value) — a non-empty
//! `summary` is accepted as-is. A stale `done()` arriving while auto-continue
//! is off is accepted inertly (never errors).
//!
//! CONT-003 (goal active): done() must pass Tier 1 (substantive
//! `goal_assessment` >= 20 chars trimmed AND >= 1 non-empty `evidence` entry)
//! and, when a verify command is configured, Tier 2 (the command must exit 0
//! within a bounded timeout). A rejected done() is just a failed tool result —
//! the loop continues; rejections are counted per-session for the escalation
//! threshold read at the settle point.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;

use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::ToolError;

/// Tool name as exposed to the model.
pub const DONE_TOOL_NAME: &str = "done";

/// Default bounded timeout for the Tier 2 verify command (doc §4).
const DEFAULT_VERIFY_TIMEOUT: Duration = Duration::from_secs(300);

/// Maximum bytes of verify-command output surfaced in a Tier 2 rejection.
const VERIFY_OUTPUT_TAIL_BYTES: usize = 4096;

/// Minimum trimmed length for a substantive `goal_assessment` (Tier 1).
const MIN_GOAL_ASSESSMENT_LEN: usize = 20;

/// CONT-003: the goal attached to a session's completion contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalSpec {
    /// The user-set goal text.
    pub text: String,
    /// Optional shell command; exit 0 = verified (Tier 2).
    pub verify: Option<String>,
}

/// Per-session completion-contract state (CONT-002 armed flag + CONT-003 goal).
#[derive(Debug, Clone, Default)]
struct ContractState {
    /// Auto-continue armed (done() registered while true).
    armed: bool,
    /// Active goal, if any (Tier 1/2 checks apply while Some).
    goal: Option<GoalSpec>,
    /// done() rejections recorded for the current goal.
    rejections: u32,
    /// Test override for the Tier 2 verify timeout.
    verify_timeout: Option<Duration>,
}

/// Per-session contract registry: session_id → contract state.
static CONTRACT_STATE: once_cell::sync::Lazy<RwLock<HashMap<Uuid, ContractState>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Per-session done() acceptance registry: session_id → accepted summary.
static DONE_ACCEPTANCE: once_cell::sync::Lazy<RwLock<HashMap<Uuid, String>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Mark a session as armed (auto-continue enabled) or disarmed.
///
/// Synced from `Session.continue_enabled` immediately before
/// `create_rig_agent` at the dispatch sites (agent_runner.rs / agent_loop.rs).
/// Disarming also clears any stale acceptance for the session.
pub fn set_continue_armed(session_id: Uuid, armed: bool) {
    if let Ok(mut guard) = CONTRACT_STATE.write() {
        let state = guard.entry(session_id).or_default();
        state.armed = armed;
    }
    if !armed {
        clear_done_acceptance(session_id);
    }
}

/// Whether the session is currently armed. Queried inside each of the seven
/// provider builder chains to conditionally register [`DoneTool`].
pub fn is_continue_armed(session_id: Uuid) -> bool {
    CONTRACT_STATE
        .read()
        .map(|guard| guard.get(&session_id).map(|s| s.armed).unwrap_or(false))
        .unwrap_or(false)
}

/// CONT-003: set or clear the active goal for a session's contract.
///
/// Synced from `Session.goal` at the same dispatch sites as
/// [`set_continue_armed`]. Setting/replacing/clearing the goal resets the
/// per-session rejection count (doc §2).
pub fn set_session_goal(session_id: Uuid, goal: Option<GoalSpec>) {
    if let Ok(mut guard) = CONTRACT_STATE.write() {
        let state = guard.entry(session_id).or_default();
        state.goal = goal;
        state.rejections = 0;
    }
}

/// CONT-003: read the active goal for a session, if any.
pub fn get_session_goal(session_id: Uuid) -> Option<GoalSpec> {
    CONTRACT_STATE
        .read()
        .ok()
        .and_then(|guard| guard.get(&session_id).and_then(|s| s.goal.clone()))
}

/// CONT-003: number of done() rejections recorded for the session's current
/// goal. Read by the stream loop at the settle point for the >= 4 escalation
/// threshold.
pub fn done_rejection_count(session_id: Uuid) -> u32 {
    CONTRACT_STATE
        .read()
        .map(|guard| guard.get(&session_id).map(|s| s.rejections).unwrap_or(0))
        .unwrap_or(0)
}

/// CONT-003 (test hook): override the Tier 2 verify-command timeout for a
/// session. Production uses [`DEFAULT_VERIFY_TIMEOUT`] (300s).
pub fn set_verify_timeout_for_tests(session_id: Uuid, timeout: Duration) {
    if let Ok(mut guard) = CONTRACT_STATE.write() {
        let state = guard.entry(session_id).or_default();
        state.verify_timeout = Some(timeout);
    }
}

fn record_rejection(session_id: Uuid) {
    if let Ok(mut guard) = CONTRACT_STATE.write() {
        let state = guard.entry(session_id).or_default();
        state.rejections = state.rejections.saturating_add(1);
    }
}

fn verify_timeout_for(session_id: Uuid) -> Duration {
    CONTRACT_STATE
        .read()
        .ok()
        .and_then(|guard| guard.get(&session_id).and_then(|s| s.verify_timeout))
        .unwrap_or(DEFAULT_VERIFY_TIMEOUT)
}

/// Take (read-and-clear) the done() acceptance recorded for this session
/// during the current turn-sequence, if any. Returns the accepted summary.
pub fn take_done_acceptance(session_id: Uuid) -> Option<String> {
    DONE_ACCEPTANCE
        .write()
        .ok()
        .and_then(|mut guard| guard.remove(&session_id))
}

/// Clear any recorded done() acceptance for this session (called at the start
/// of each real user turn).
pub fn clear_done_acceptance(session_id: Uuid) {
    if let Ok(mut guard) = DONE_ACCEPTANCE.write() {
        guard.remove(&session_id);
    }
}

/// Arguments for the done tool.
#[derive(Debug, Deserialize, Serialize)]
pub struct DoneArgs {
    /// Required, non-empty completion summary surfaced as the turn's closing line.
    pub summary: String,
    /// Supporting evidence (required while a goal is active — CONT-003 Tier 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Vec<String>>,
    /// Assessment against the active goal (required while a goal is active).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal_assessment: Option<String>,
}

/// DoneTool — explicit completion confirmation for the completion contract.
#[derive(Clone, Debug)]
pub struct DoneTool {
    session_id: Uuid,
}

impl DoneTool {
    /// Create a new DoneTool bound to a session.
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
    }

    /// The session this tool instance is bound to.
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    /// CONT-003 Tier 1: a goal-mode done() must carry >= 1 non-empty evidence
    /// string and a substantive (>= 20 chars trimmed) goal_assessment.
    fn tier1_ok(args: &DoneArgs) -> bool {
        let evidence_ok = args
            .evidence
            .as_ref()
            .is_some_and(|e| e.iter().any(|s| !s.trim().is_empty()));
        let assessment_ok = args
            .goal_assessment
            .as_ref()
            .is_some_and(|a| a.trim().len() >= MIN_GOAL_ASSESSMENT_LEN);
        evidence_ok && assessment_ok
    }

    /// CONT-003 Tier 2: run the verify command at the project root with a
    /// bounded timeout. Returns Ok(()) on exit 0; Err(rejection message)
    /// on non-zero exit or timeout.
    async fn run_verify(&self, command: &str) -> Result<(), String> {
        let timeout = verify_timeout_for(self.session_id);
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .output();
        match tokio::time::timeout(timeout, child).await {
            Err(_) => Err(format!(
                "done() rejected: verification command timed out after {}s",
                timeout.as_secs_f64()
            )),
            Ok(Err(e)) => Err(format!(
                "done() rejected: verification command failed to start: {e}"
            )),
            Ok(Ok(output)) => {
                if output.status.success() {
                    Ok(())
                } else {
                    let code = output
                        .status
                        .code()
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "signal".to_string());
                    let mut combined = output.stdout;
                    combined.extend_from_slice(&output.stderr);
                    let tail_start = combined.len().saturating_sub(VERIFY_OUTPUT_TAIL_BYTES);
                    let tail = String::from_utf8_lossy(&combined[tail_start..]);
                    Err(format!(
                        "done() rejected: verification command failed (exit {code}):\n{}",
                        tail.trim_end()
                    ))
                }
            }
        }
    }
}

impl Tool for DoneTool {
    const NAME: &'static str = DONE_TOOL_NAME;

    type Error = ToolError;
    type Args = DoneArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        let mut description = concat!(
            "Signal that the current task is complete. Call this when you have ",
            "finished the work the user asked for. The summary is shown to the ",
            "user as the closing line of the turn."
        )
        .to_string();
        // CONT-003: dynamic description while a goal is active (definition()
        // runs per-prompt — agents are rebuilt each user turn).
        if let Some(goal) = get_session_goal(self.session_id) {
            description.push_str(&format!(
                " The current goal is: {}. You must not call done() unless this \
                 goal is met; provide evidence and goal_assessment.",
                goal.text
            ));
        }
        rig::completion::ToolDefinition {
            name: DONE_TOOL_NAME.to_string(),
            description,
            parameters: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "summary": {
                        "type": "string",
                        "description": "Non-empty summary of what was completed"
                    },
                    "evidence": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Supporting evidence (required while a goal is active)"
                    },
                    "goal_assessment": {
                        "type": "string",
                        "description": "Assessment against the active goal (required while a goal is active)"
                    }
                },
                "required": ["summary"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Tier-0 validation: a non-empty summary is required.
        if args.summary.trim().is_empty() {
            return Err(ToolError::Validation {
                tool: "done",
                message: "done() requires a non-empty summary describing what was completed"
                    .to_string(),
            });
        }

        // Stale call after toggle-off: accept inertly, never error, and do
        // not record acceptance (the contract is no longer active).
        if !is_continue_armed(self.session_id) {
            return Ok("Acknowledged (auto-continue is off).".to_string());
        }

        // CONT-003: goal-mode acceptance pipeline.
        if let Some(goal) = get_session_goal(self.session_id) {
            // Tier 1 — schema: evidence + substantive goal_assessment.
            if !Self::tier1_ok(&args) {
                record_rejection(self.session_id);
                return Err(ToolError::Validation {
                    tool: "done",
                    message: format!(
                        "done() rejected: you must provide evidence and a \
                         goal_assessment for the active goal: {}",
                        goal.text
                    ),
                });
            }
            // Tier 2 — verify command (only after Tier 1 passes).
            if let Some(verify) = goal.verify.as_deref() {
                if let Err(message) = self.run_verify(verify).await {
                    record_rejection(self.session_id);
                    return Err(ToolError::Validation {
                        tool: "done",
                        message,
                    });
                }
            }
        }

        // Acceptance: record for the stream loop to read at the
        // FinalResponse settle point.
        if let Ok(mut guard) = DONE_ACCEPTANCE.write() {
            guard.insert(self.session_id, args.summary);
        }
        Ok("Completion recorded. The turn will finish with your summary.".to_string())
    }
}
