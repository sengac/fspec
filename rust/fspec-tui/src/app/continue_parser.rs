//! `/continue` subcommand parser + status-bar indicator — CONT-002.
//!
//! Feature: spec/features/continue-command-surface.feature
//!
//! Mirrors the grammar of `codelet_cli::interactive::auto_continue`
//! (doc §4, spec/attachments/CONT-002/design-auto-continue.md):
//! * `/continue` (bare) → [`ContinueSubcommand::Toggle`]
//! * `/continue <n>` (n >= 1) → [`ContinueSubcommand::SetBudget`]
//! * `/continue on` / `/continue off` → explicit set
//! * `/continue 0` → [`ContinueSubcommand::RejectZero`] (hint: use /continue off)
//! * anything else → [`ContinueSubcommand::Invalid`], state unchanged
//!
//! Modeled on `loop_parser.rs` (`LoopSubcommand`), routed from
//! `slash_parser.rs::parse_slash_command` like the `/loop` family. The
//! grammar is a deliberate small mirror of the CLI's shared pure parser
//! (the crates do not share a parser crate — arch note [3]).

/// Default zero-progress nudge budget (mirrors
/// `codelet_cli::interactive::auto_continue::DEFAULT_CONTINUE_BUDGET`).
pub const DEFAULT_CONTINUE_BUDGET: u32 = 10;

/// Outcome of parsing a `/continue …` slash command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinueSubcommand {
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
    /// Anything else — error notice, state unchanged.
    Invalid(String),
}

/// Parse a `/continue …` slash-command input into a [`ContinueSubcommand`].
/// `input` may begin with `/continue` (which is stripped) or already be the
/// body.
pub fn parse_continue_command(input: &str) -> ContinueSubcommand {
    let trimmed = input.trim();
    let body = trimmed.strip_prefix("/continue").unwrap_or(trimmed).trim();
    if body.is_empty() {
        return ContinueSubcommand::Toggle;
    }
    if body.eq_ignore_ascii_case("on") {
        return ContinueSubcommand::On;
    }
    if body.eq_ignore_ascii_case("off") {
        return ContinueSubcommand::Off;
    }
    match body.parse::<u32>() {
        Ok(0) => ContinueSubcommand::RejectZero,
        Ok(n) => ContinueSubcommand::SetBudget(n),
        Err(_) => ContinueSubcommand::Invalid(body.to_string()),
    }
}

/// Result of applying a [`ContinueSubcommand`] to `(enabled, budget)` state.
/// Byte-for-byte mirror of the CLI's `apply_continue_command` semantics so
/// both surfaces print identical state lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinueApplyOutcome {
    /// New continue_enabled value.
    pub enabled: bool,
    /// New continue_budget value.
    pub budget: u32,
    /// User-facing state/error notice (always shown).
    pub message: String,
    /// Whether the state changed (false for RejectZero/Invalid).
    pub changed: bool,
}

fn on_message(budget: u32) -> String {
    format!("auto-continue: on (budget {budget})")
}

/// Apply a parsed `/continue` subcommand to the current state.
///
/// CONT-003: `goal_active` refuses `/continue off` while a goal is active —
/// the goal implies auto-continue, so the user must `/goal clear` first
/// (byte-for-byte mirror of the CLI's `apply_continue_command`).
pub fn apply_continue_subcommand(
    enabled: bool,
    budget: u32,
    goal_active: bool,
    sub: &ContinueSubcommand,
) -> ContinueApplyOutcome {
    // CONT-003: refuse explicit off while a goal is active (doc §3).
    if goal_active && matches!(sub, ContinueSubcommand::Off) {
        return ContinueApplyOutcome {
            enabled,
            budget,
            message: "auto-continue: a goal is active — clear the goal first (/goal clear)"
                .to_string(),
            changed: false,
        };
    }
    match sub {
        ContinueSubcommand::Toggle => {
            if enabled {
                ContinueApplyOutcome {
                    enabled: false,
                    budget,
                    message: "auto-continue: off".to_string(),
                    changed: true,
                }
            } else {
                ContinueApplyOutcome {
                    enabled: true,
                    budget: DEFAULT_CONTINUE_BUDGET,
                    message: on_message(DEFAULT_CONTINUE_BUDGET),
                    changed: true,
                }
            }
        }
        ContinueSubcommand::On => ContinueApplyOutcome {
            enabled: true,
            budget: DEFAULT_CONTINUE_BUDGET,
            message: on_message(DEFAULT_CONTINUE_BUDGET),
            changed: true,
        },
        ContinueSubcommand::Off => ContinueApplyOutcome {
            enabled: false,
            budget,
            message: "auto-continue: off".to_string(),
            changed: true,
        },
        ContinueSubcommand::SetBudget(n) => ContinueApplyOutcome {
            enabled: true,
            budget: *n,
            message: on_message(*n),
            changed: true,
        },
        ContinueSubcommand::RejectZero => ContinueApplyOutcome {
            enabled,
            budget,
            message: "auto-continue: budget must be at least 1 — use /continue off".to_string(),
            changed: false,
        },
        ContinueSubcommand::Invalid(arg) => ContinueApplyOutcome {
            enabled,
            budget,
            message: format!(
                "auto-continue: invalid argument '{arg}' — usage: /continue [on|off|<n>]"
            ),
            changed: false,
        },
    }
}

/// Status-bar indicator: `⏩ auto-continue (n/N)` while armed, nothing while
/// off.
pub fn continue_status_indicator(enabled: bool, nudges_used: u32, budget: u32) -> Option<String> {
    if enabled {
        Some(format!("⏩ auto-continue ({nudges_used}/{budget})"))
    } else {
        None
    }
}
