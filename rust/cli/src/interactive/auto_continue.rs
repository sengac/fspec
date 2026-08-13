//! CONT-002 — Auto-continue engine: pure decision function, budget/refund
//! arithmetic, and /continue command grammar.
//!
//! Feature: spec/features/auto-continue-engine.feature
//! Feature: spec/features/continue-command-surface.feature
//!
//! The stream loop consults [`decide_continuation`] at the clean
//! `FinalResponse` settle point (stream_loop.rs, just before
//! `emit_done_with_stop_reason`). No other emit site (interrupt, stall
//! timeout, error) may nudge.
//!
//! Doc §5 decision table (spec/attachments/CONT-002/design-auto-continue.md):
//! - mode Off → Finish (today's behavior, zero change)
//! - done() accepted this turn-sequence → FinishWithSummary
//! - stop_reason in {stop, end_turn} (or None) without done(), used < budget → Nudge
//! - same, budget exhausted → FinishWithWarning
//! - interrupted → Finish (user interrupt ALWAYS wins)
//! - max_tokens/truncation → Finish (existing PROV-040/041 handling first)

use crate::session::Session;

/// Default zero-progress nudge budget per user-turn (`/continue` bare).
pub const DEFAULT_CONTINUE_BUDGET: u32 = 10;

/// One-shot nudge prompt injected as a plain user message (PROV-040/041 style).
pub const AUTO_CONTINUE_NUDGE_PROMPT: &str = "You stopped without calling done(). \
If the task is complete, call done(summary); otherwise continue working.";

/// Outcome of the auto-continue decision at the FinalResponse settle point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinueDecision {
    /// Finish the turn exactly as today (Off mode, interrupt, truncation).
    Finish,
    /// Inject the nudge prompt and continue the loop; count the nudge.
    Nudge,
    /// Finish and emit the visible budget-exhaustion warning line.
    FinishWithWarning(String),
    /// Finish and surface the accepted done() summary as the closing line.
    FinishWithSummary(String),
    /// CONT-003 Goal mode: stop the world for human review (HITL pause / the
    /// prominent blocked message in plain CLI repl). The goal stays active.
    Escalate(String),
}

/// Pure decision function for the auto-continue engine (doc §5 table).
///
/// * `armed` — derived mode is AutoContinue (`session.continue_enabled`).
/// * `done_summary` — accepted done() summary for this turn-sequence, if any.
/// * `stop_reason` — provider stop reason at the settle point (None allowed).
/// * `nudges_used` / `budget` — zero-progress nudge accounting.
/// * `interrupted` — user interrupt state (always wins).
pub fn decide_continuation(
    armed: bool,
    done_summary: Option<&str>,
    stop_reason: Option<&str>,
    nudges_used: u32,
    budget: u32,
    interrupted: bool,
) -> ContinueDecision {
    // Off mode: byte-for-byte today's behavior.
    if !armed {
        return ContinueDecision::Finish;
    }
    // Accepted done() ends the contract for this turn-sequence.
    if let Some(summary) = done_summary {
        if !summary.trim().is_empty() {
            return ContinueDecision::FinishWithSummary(summary.to_string());
        }
    }
    // User interrupt ALWAYS wins — never nudge an interrupted stream.
    if interrupted {
        return ContinueDecision::Finish;
    }
    // Only a clean stop ({stop, end_turn} or absent) may nudge; truncation
    // stops (max_tokens/length) are owned by PROV-040/041 recovery.
    let clean_stop = matches!(stop_reason, None | Some("stop") | Some("end_turn"));
    if !clean_stop {
        return ContinueDecision::Finish;
    }
    if nudges_used < budget {
        ContinueDecision::Nudge
    } else {
        ContinueDecision::FinishWithWarning(build_continue_exhaustion_warning(nudges_used))
    }
}

/// CONT-003: pure decision function for Goal mode at the same FinalResponse
/// settle point (doc §5). Differs from [`decide_continuation`] in the
/// exhaustion paths: Goal mode justifies stopping the world, so budget
/// exhaustion, `done_rejections >= 4`, and the stall fast-path (two
/// consecutive zero-activity nudges) all `Escalate` instead of finishing
/// with the AutoContinue warning.
///
/// * `done_summary` — accepted done() summary for this turn-sequence, if any
///   (rejections never reach here — they are tool-level errors).
/// * `stop_reason` — provider stop reason at the settle point (None allowed).
/// * `nudges_used` / `budget` — zero-progress nudge accounting (budget is the
///   effective Goal budget, max(explicit, 15)).
/// * `done_rejections` — per-session rejection count from the done() registry.
/// * `consecutive_zero_activity_nudges` — nudged segments in a row with no
///   tool calls and no done() (stall fast-path at 2).
/// * `interrupted` — user interrupt state (always wins).
pub fn decide_goal_continuation(
    done_summary: Option<&str>,
    stop_reason: Option<&str>,
    nudges_used: u32,
    budget: u32,
    done_rejections: u32,
    consecutive_zero_activity_nudges: u32,
    interrupted: bool,
) -> ContinueDecision {
    // Accepted done() ends the contract for this turn-sequence.
    if let Some(summary) = done_summary {
        if !summary.trim().is_empty() {
            return ContinueDecision::FinishWithSummary(summary.to_string());
        }
    }
    // User interrupt ALWAYS wins — never nudge or escalate an interrupted stream.
    if interrupted {
        return ContinueDecision::Finish;
    }
    // Escalation: 4+ done() rejections — the model repeatedly claims
    // completion but verification fails (doc §5).
    if done_rejections >= 4 {
        return ContinueDecision::Escalate(crate::interactive::goal::build_goal_blocked_message());
    }
    // Stall fast-path: two consecutive zero-activity nudges escalate
    // immediately without burning the remaining budget.
    if consecutive_zero_activity_nudges >= 2 {
        return ContinueDecision::Escalate(
            "🎯 goal: model stalled (two consecutive zero-activity nudges) — human review needed"
                .to_string(),
        );
    }
    // Only a clean stop ({stop, end_turn} or absent) may nudge; truncation
    // stops (max_tokens/length) are owned by PROV-040/041 recovery.
    let clean_stop = matches!(stop_reason, None | Some("stop") | Some("end_turn"));
    if !clean_stop {
        return ContinueDecision::Finish;
    }
    if nudges_used < budget {
        ContinueDecision::Nudge
    } else {
        // Budget exhaustion in Goal mode escalates — NOT the AutoContinue
        // silent-warning finish (doc §5).
        ContinueDecision::Escalate(
            "🎯 goal: zero-progress nudge budget exhausted — human review needed".to_string(),
        )
    }
}

/// Budget accounting for the segment that followed a nudge: if the segment
/// produced >= 1 tool call, the nudge is refunded (zero-progress budget).
/// Returns the new `nudges_used` value.
pub fn apply_segment_outcome(nudges_used: u32, tool_calls_in_segment: usize) -> u32 {
    if tool_calls_in_segment >= 1 {
        nudges_used.saturating_sub(1)
    } else {
        nudges_used
    }
}

/// Reset per-user-turn auto-continue accounting. Called at the start of every
/// real user message (NOT for synthetic nudge prompts).
pub fn reset_for_new_user_turn(session: &mut Session) {
    session.continue_nudges_used = 0;
}

/// Build the visible budget-exhaustion warning line (doc §2, option b).
pub fn build_continue_exhaustion_warning(retries: u32) -> String {
    format!("⚠ auto-continue: model never called done() after {retries} retries")
}

/// Parsed `/continue` command (doc §4 grammar). Shared by the CLI repl and
/// the NAPI session-state setter; the fspec-tui crate has its own mirror
/// parser (continue_parser.rs) since the crates do not share a parser crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinueCommand {
    /// `/continue` — toggle on/off with default budget.
    Toggle,
    /// `/continue on` — explicit on, default budget.
    On,
    /// `/continue off` — explicit off.
    Off,
    /// `/continue <n>` with n >= 1 — arm with budget n / update budget.
    SetBudget(u32),
    /// `/continue 0` — rejected with hint "use /continue off".
    RejectZero,
    /// Anything else — error, state unchanged.
    Invalid(String),
}

/// Parse the `/continue [arg]` body. Accepts input with or without the
/// leading `/continue`.
pub fn parse_continue_command(input: &str) -> ContinueCommand {
    let trimmed = input.trim();
    let body = trimmed.strip_prefix("/continue").unwrap_or(trimmed).trim();
    if body.is_empty() {
        return ContinueCommand::Toggle;
    }
    if body.eq_ignore_ascii_case("on") {
        return ContinueCommand::On;
    }
    if body.eq_ignore_ascii_case("off") {
        return ContinueCommand::Off;
    }
    match body.parse::<u32>() {
        Ok(0) => ContinueCommand::RejectZero,
        Ok(n) => ContinueCommand::SetBudget(n),
        Err(_) => ContinueCommand::Invalid(body.to_string()),
    }
}

/// Result of applying a [`ContinueCommand`] to the current session state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinueApplyResult {
    /// New continue_enabled value.
    pub enabled: bool,
    /// New continue_budget value.
    pub budget: u32,
    /// User-facing state/error message (always printed).
    pub message: String,
    /// Whether the state changed (false for RejectZero/Invalid).
    pub changed: bool,
}

fn on_message(budget: u32) -> String {
    format!("auto-continue: on (budget {budget})")
}

/// Apply a parsed `/continue` command to `(enabled, budget)` state.
/// Pure so the CLI repl and the NAPI setter agree byte-for-byte.
///
/// CONT-003: `goal_active` refuses `/continue off` while a goal is active —
/// the goal implies auto-continue, so the user must `/goal clear` first.
pub fn apply_continue_command(
    enabled: bool,
    budget: u32,
    goal_active: bool,
    cmd: &ContinueCommand,
) -> ContinueApplyResult {
    // CONT-003: refuse explicit off while a goal is active (doc §3).
    if goal_active && matches!(cmd, ContinueCommand::Off) {
        return ContinueApplyResult {
            enabled,
            budget,
            message: "auto-continue: a goal is active — clear the goal first (/goal clear)"
                .to_string(),
            changed: false,
        };
    }
    match cmd {
        ContinueCommand::Toggle => {
            if enabled {
                ContinueApplyResult {
                    enabled: false,
                    budget,
                    message: "auto-continue: off".to_string(),
                    changed: true,
                }
            } else {
                ContinueApplyResult {
                    enabled: true,
                    budget: DEFAULT_CONTINUE_BUDGET,
                    message: on_message(DEFAULT_CONTINUE_BUDGET),
                    changed: true,
                }
            }
        }
        ContinueCommand::On => ContinueApplyResult {
            enabled: true,
            budget: DEFAULT_CONTINUE_BUDGET,
            message: on_message(DEFAULT_CONTINUE_BUDGET),
            changed: true,
        },
        ContinueCommand::Off => ContinueApplyResult {
            enabled: false,
            budget,
            message: "auto-continue: off".to_string(),
            changed: true,
        },
        ContinueCommand::SetBudget(n) => ContinueApplyResult {
            enabled: true,
            budget: *n,
            message: on_message(*n),
            changed: true,
        },
        ContinueCommand::RejectZero => ContinueApplyResult {
            enabled,
            budget,
            message: "auto-continue: budget must be at least 1 — use /continue off".to_string(),
            changed: false,
        },
        ContinueCommand::Invalid(arg) => ContinueApplyResult {
            enabled,
            budget,
            message: format!(
                "auto-continue: invalid argument '{arg}' — usage: /continue [on|off|<n>]"
            ),
            changed: false,
        },
    }
}
