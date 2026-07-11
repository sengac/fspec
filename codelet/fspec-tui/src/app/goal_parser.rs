//! `/goal` subcommand parser + status-bar indicator — CONT-003.
//!
//! Feature: spec/features/goal-command-surface.feature
//!
//! Mirrors the grammar of `codelet_cli::interactive::goal`
//! (doc §3, spec/attachments/CONT-003/design-goal-mode.md):
//! * `/goal <text>` → [`GoalSubcommand::Set`] (set/replace the goal)
//! * `/goal` (bare) → [`GoalSubcommand::Show`] (contract state)
//! * `/goal verify <cmd>` → [`GoalSubcommand::Verify`] (needs active goal)
//! * `/goal clear` → [`GoalSubcommand::Clear`] (fall back to the toggle)
//!
//! Modeled on `continue_parser.rs` (`ContinueSubcommand`), routed from
//! `slash_parser.rs::parse_slash_command` like the `/continue` family. The
//! grammar is a deliberate small mirror of the CLI's shared pure parser
//! (the crates do not share a parser crate — arch note [3]).

/// Goal-mode default zero-progress nudge budget (mirrors
/// `codelet_cli::interactive::goal::GOAL_DEFAULT_BUDGET`).
pub const GOAL_DEFAULT_BUDGET: u32 = 15;

/// Outcome of parsing a `/goal …` slash command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalSubcommand {
    /// `/goal <text>` — set/replace the goal.
    Set(String),
    /// `/goal` — show the contract state.
    Show,
    /// `/goal verify <cmd>` — attach/replace the verify command.
    Verify(String),
    /// `/goal clear` — drop the goal.
    Clear,
}

/// Parse a `/goal …` slash-command input into a [`GoalSubcommand`].
/// `input` may begin with `/goal` (which is stripped) or already be the
/// body.
pub fn parse_goal_command(input: &str) -> GoalSubcommand {
    let trimmed = input.trim();
    let body = trimmed.strip_prefix("/goal").unwrap_or(trimmed).trim();
    if body.is_empty() {
        return GoalSubcommand::Show;
    }
    if body.eq_ignore_ascii_case("clear") {
        return GoalSubcommand::Clear;
    }
    if let Some(cmd) = body
        .strip_prefix("verify ")
        .or_else(|| body.strip_prefix("verify\t"))
    {
        return GoalSubcommand::Verify(cmd.trim().to_string());
    }
    GoalSubcommand::Set(body.to_string())
}

/// Result of applying a [`GoalSubcommand`] to the cached
/// `(goal, continue_enabled, continue_budget)` chrome state. Mirrors the
/// CLI's `apply_goal_command` semantics so both surfaces print matching
/// state lines. The goal is `(text, verify)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalApplyOutcome {
    /// New goal chrome state `(text, verify)`.
    pub goal: Option<(String, Option<String>)>,
    /// User-facing state/error notice (always shown).
    pub message: String,
    /// Whether the state changed (false for Show / refused Verify /
    /// no-op Clear).
    pub changed: bool,
}

/// Effective Goal-mode budget: max(explicit continue budget, 15).
fn effective_budget(continue_budget: u32) -> u32 {
    continue_budget.max(GOAL_DEFAULT_BUDGET)
}

/// Apply a parsed `/goal` subcommand to the cached chrome state.
///
/// CONT-008: the bare `/goal` Show display renders the REAL per-turn
/// counters from the CONT-007 live snapshot cache (`live_counters` =
/// `(nudges_used, done_rejections)`, `(0, 0)` when no live snapshot has
/// arrived — truthful, since any goal change resets both counters at the
/// next dispatch sync). The Set acknowledgement keeps its literal zeros:
/// `Session::set_goal` resets both counters, byte parity with the CLI's
/// `apply_goal_command`.
pub fn apply_goal_subcommand(
    goal: Option<(String, Option<String>)>,
    continue_enabled: bool,
    continue_budget: u32,
    live_counters: (u32, u32),
    sub: &GoalSubcommand,
) -> GoalApplyOutcome {
    match sub {
        GoalSubcommand::Set(text) => GoalApplyOutcome {
            goal: Some((text.clone(), None)),
            message: format!(
                "🎯 goal set: {text}\nbudget: {}, nudges used: 0, rejections: 0",
                effective_budget(continue_budget)
            ),
            changed: true,
        },
        GoalSubcommand::Show => match &goal {
            Some((text, verify)) => {
                let verify_str = verify.as_deref().unwrap_or("none");
                let (nudges_used, rejections) = live_counters;
                GoalApplyOutcome {
                    message: format!(
                        "🎯 goal: {text}\nverify: {verify_str}\nbudget: {}, nudges used: {nudges_used}, rejections: {rejections}",
                        effective_budget(continue_budget)
                    ),
                    goal,
                    changed: false,
                }
            }
            None => GoalApplyOutcome {
                goal,
                message: "no goal set — use /goal <text> to set one".to_string(),
                changed: false,
            },
        },
        GoalSubcommand::Verify(command) => match goal {
            Some((text, _)) => GoalApplyOutcome {
                goal: Some((text, Some(command.clone()))),
                message: format!("🎯 goal verify command set: {command}"),
                changed: true,
            },
            None => GoalApplyOutcome {
                goal: None,
                message: "goal: no active goal — set one first with /goal <text>".to_string(),
                changed: false,
            },
        },
        GoalSubcommand::Clear => match goal {
            Some(_) => {
                let fallback = if continue_enabled {
                    format!("auto-continue (budget {continue_budget})")
                } else {
                    "off".to_string()
                };
                GoalApplyOutcome {
                    goal: None,
                    message: format!("🎯 goal cleared — falling back to {fallback}"),
                    changed: true,
                }
            }
            None => GoalApplyOutcome {
                goal: None,
                message: "no goal set — nothing to clear".to_string(),
                changed: false,
            },
        },
    }
}

/// Status-bar indicator: `🎯 goal (n/N)` (N = effective Goal budget)
/// replaces the `⏩ auto-continue` indicator while a goal is active; falls
/// back to the CONT-002 indicator when the goal is cleared with
/// auto-continue on. Mirrors `codelet_cli::interactive::goal::goal_status_indicator`.
pub fn goal_status_indicator(
    goal_active: bool,
    continue_enabled: bool,
    nudges_used: u32,
    explicit_budget: u32,
) -> Option<String> {
    if goal_active {
        let effective = effective_budget(explicit_budget);
        Some(format!("🎯 goal ({nudges_used}/{effective})"))
    } else if continue_enabled {
        Some(format!(
            "⏩ auto-continue ({nudges_used}/{explicit_budget})"
        ))
    } else {
        None
    }
}
