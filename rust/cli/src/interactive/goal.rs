//! CONT-003 — /goal mode: conditional done() acceptance against a user-set
//! goal.
//!
//! Feature: spec/features/goal-enforcement.feature
//! Feature: spec/features/goal-command-surface.feature
//!
//! `/goal` = `/continue` + an acceptance condition on `done()`. While a goal
//! is active the effective mode is Goal (implies auto-continue) and done()
//! must pass the Tier 1/2 acceptance pipeline in `codelet_tools::done`. This
//! module owns the derived mode, the effective budget resolution
//! (max(explicit, 15)), the `/goal` command grammar + apply semantics, the
//! escalation surfacing (HITL pause / prominent blocked message), and the
//! status-bar goal indicator.

use uuid::Uuid;

use crate::session::Session;
use codelet_tools::tool_pause::{pause_for_user, PauseKind, PauseRequest, PauseResponse};

/// Goal-mode default zero-progress nudge budget (doc §2).
pub const GOAL_DEFAULT_BUDGET: u32 = 15;

/// Derived effective mode (doc §1): mode is derived, never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectiveMode {
    /// No goal, continue toggle off.
    Off,
    /// No goal, continue toggle on (CONT-002 behavior unchanged).
    AutoContinue,
    /// Goal active — wins over the continue toggle.
    Goal,
}

/// Derive the effective mode from session state:
/// Goal if a goal is set, else AutoContinue if the toggle is on, else Off.
pub fn effective_mode(session: &Session) -> EffectiveMode {
    if session.goal.is_some() {
        EffectiveMode::Goal
    } else if session.continue_enabled {
        EffectiveMode::AutoContinue
    } else {
        EffectiveMode::Off
    }
}

/// Effective Goal-mode budget: the larger of the explicit `/continue <n>`
/// budget and the Goal default of 15 (doc §2). Computed where the budget is
/// read, never stored.
pub fn effective_goal_budget(session: &Session) -> u32 {
    session.continue_budget.max(GOAL_DEFAULT_BUDGET)
}

/// The prominent blocked message emitted in plain CLI repl mode when a goal
/// escalation is raised without a registered pause handler (doc §5).
pub fn build_goal_blocked_message() -> String {
    "🎯 goal: model repeatedly claims completion but verification fails — human review needed"
        .to_string()
}

/// Raise a goal escalation for the session via the per-session pause-handler
/// registry (doc §5). With a registered handler (TUI/NAPI surfaces) this
/// blocks until the pause resolves; in plain CLI repl mode no handler is
/// registered and `pause_for_user` returns `Resumed` immediately, so the
/// caller emits [`build_goal_blocked_message`] first — both surfaces get
/// correct behavior from this one code path. The goal stays active.
pub fn raise_goal_escalation(session_id: Uuid, message: &str) -> PauseResponse {
    pause_for_user(
        session_id,
        PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "goal".to_string(),
            message: message.to_string(),
            details: None,
        },
    )
}

/// Apply an accepted done() to the session (doc §4): announce
/// `🎯 goal satisfied: <summary>`, auto-clear the goal (falls back to the
/// continue toggle), and reset done_rejections. Returns the announcement.
pub fn apply_goal_acceptance(session: &mut Session, summary: &str) -> String {
    session.clear_goal();
    format!("🎯 goal satisfied: {summary}")
}

/// Parsed `/goal` command (doc §3 grammar). Shared by the CLI repl and the
/// NAPI session-state setter; the fspec-tui crate has its own mirror parser
/// (goal_parser.rs) since the crates do not share a parser crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalCommand {
    /// `/goal <text>` — set/replace the goal.
    Set(String),
    /// `/goal` — show the contract state.
    Show,
    /// `/goal verify <cmd>` — attach/replace the verify command.
    Verify(String),
    /// `/goal clear` — drop the goal.
    Clear,
}

/// Parse the `/goal [args]` body. Accepts input with or without the leading
/// `/goal`.
pub fn parse_goal_command(input: &str) -> GoalCommand {
    let trimmed = input.trim();
    let body = trimmed.strip_prefix("/goal").unwrap_or(trimmed).trim();
    if body.is_empty() {
        return GoalCommand::Show;
    }
    if body.eq_ignore_ascii_case("clear") {
        return GoalCommand::Clear;
    }
    if let Some(cmd) = body
        .strip_prefix("verify ")
        .or_else(|| body.strip_prefix("verify\t"))
    {
        return GoalCommand::Verify(cmd.trim().to_string());
    }
    GoalCommand::Set(body.to_string())
}

/// Result of applying a [`GoalCommand`] to the session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalApplyResult {
    /// User-facing state/error message (always printed).
    pub message: String,
    /// Whether the session state changed.
    pub changed: bool,
}

/// Render the full contract state line (goal text, verify command, effective
/// budget, nudges used, rejections).
fn goal_state_message(session: &Session) -> String {
    match &session.goal {
        Some(goal) => {
            let verify = goal.verify.as_deref().unwrap_or("none");
            format!(
                "🎯 goal: {}\nverify: {}\nbudget: {}, nudges used: {}, rejections: {}",
                goal.text,
                verify,
                effective_goal_budget(session),
                session.continue_nudges_used,
                session.done_rejections,
            )
        }
        None => "no goal set — use /goal <text> to set one".to_string(),
    }
}

/// Apply a parsed `/goal` command to the session (doc §3 table).
pub fn apply_goal_command(session: &mut Session, cmd: &GoalCommand) -> GoalApplyResult {
    match cmd {
        GoalCommand::Set(text) => {
            session.set_goal(text, None);
            GoalApplyResult {
                message: format!(
                    "🎯 goal set: {text}\nbudget: {}, nudges used: 0, rejections: 0",
                    effective_goal_budget(session)
                ),
                changed: true,
            }
        }
        GoalCommand::Show => GoalApplyResult {
            message: goal_state_message(session),
            changed: false,
        },
        GoalCommand::Verify(command) => {
            if session.goal.is_none() {
                return GoalApplyResult {
                    message: "goal: no active goal — set one first with /goal <text>".to_string(),
                    changed: false,
                };
            }
            if let Some(goal) = session.goal.as_mut() {
                goal.verify = Some(command.clone());
            }
            session.refresh_goal_reminder();
            GoalApplyResult {
                message: format!("🎯 goal verify command set: {command}"),
                changed: true,
            }
        }
        GoalCommand::Clear => {
            if session.goal.is_none() {
                return GoalApplyResult {
                    message: "no goal set — nothing to clear".to_string(),
                    changed: false,
                };
            }
            session.clear_goal();
            let fallback = if session.continue_enabled {
                format!("auto-continue (budget {})", session.continue_budget)
            } else {
                "off".to_string()
            };
            GoalApplyResult {
                message: format!("🎯 goal cleared — falling back to {fallback}"),
                changed: true,
            }
        }
    }
}

/// Status-bar indicator: `🎯 goal (n/N)` (N = effective Goal budget) replaces
/// the `⏩ auto-continue` indicator while a goal is active; falls back to the
/// CONT-002 indicator when the goal is cleared with auto-continue on.
pub fn goal_status_indicator(
    goal_active: bool,
    continue_enabled: bool,
    nudges_used: u32,
    explicit_budget: u32,
) -> Option<String> {
    if goal_active {
        let effective = explicit_budget.max(GOAL_DEFAULT_BUDGET);
        Some(format!("🎯 goal ({nudges_used}/{effective})"))
    } else if continue_enabled {
        Some(format!(
            "⏩ auto-continue ({nudges_used}/{explicit_budget})"
        ))
    } else {
        None
    }
}
