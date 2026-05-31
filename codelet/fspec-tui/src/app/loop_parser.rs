//! `/loop` subcommand parser — RPC-059.
//!
//! Mirrors `src/tui/utils/loopCommandParser.ts::parseLoopCommand` from
//! the TS Ink frontend.
//!
//! Supported syntaxes:
//! * `/loop` (bare) → [`LoopSubcommand::Help`]
//! * `/loop list` → [`LoopSubcommand::List`]
//! * `/loop cancel <id>` → [`LoopSubcommand::Cancel`]
//! * `/loop <N>s|m|h|d <prompt>` → leading-interval add
//! * `/loop <prompt> every <N> <unit>` → trailing-interval add
//! * `/loop <prompt>` (no interval) → default 600s add

use regex::Regex;
use std::sync::OnceLock;

/// Outcome of parsing a `/loop …` slash command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopSubcommand {
    /// `/loop [interval] <prompt>` — add a new loop.
    Add {
        interval_seconds: u32,
        prompt: String,
    },
    /// `/loop cancel <id>` — cancel an existing loop.
    Cancel { id: String },
    /// `/loop list` — list active loops.
    List,
    /// `/loop` (bare) or unknown subcommand — fall through to help.
    Help,
}

/// Default loop interval in seconds (10 minutes) used when no
/// interval qualifier is present.
const DEFAULT_INTERVAL_SECS: u32 = 600;

fn leading_interval_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^(\d+)([smhd])$").unwrap_or_else(|_| {
            // SAFETY: "^$" is a trivially valid regex — unwrap is infallible.
            #[allow(clippy::expect_used)]
            Regex::new("^$").expect("infallible fallback regex")
        })
    })
}

fn trailing_interval_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)\s+every\s+(\d+)\s*(s|sec|seconds?|m|min|minutes?|h|hrs?|hours?|d|days?)$",
        )
        .unwrap_or_else(|_| {
            // SAFETY: "^$" is a trivially valid regex — unwrap is infallible.
            #[allow(clippy::expect_used)]
            Regex::new("^$").expect("infallible fallback regex")
        })
    })
}

/// Convert a unit token (lowercase) + numeric magnitude into a number
/// of seconds. Matches TS `unitToSeconds`.
fn unit_to_seconds(value: u32, unit: &str) -> u32 {
    let u = unit.to_lowercase();
    match u.as_str() {
        "s" | "sec" | "second" | "seconds" => value.max(1),
        "m" | "min" | "minute" | "minutes" => value.saturating_mul(60),
        "h" | "hr" | "hrs" | "hour" | "hours" => value.saturating_mul(3600),
        "d" | "day" | "days" => value.saturating_mul(86_400),
        _ => value.max(1),
    }
}

/// Parse a `/loop …` slash-command input into a [`LoopSubcommand`].
/// `input` may begin with `/loop` (which is stripped) or already be
/// the body.
pub fn parse_loop_command(input: &str) -> LoopSubcommand {
    let trimmed = input.trim();
    let body = trimmed
        .strip_prefix("/loop")
        .map(str::trim_start)
        .unwrap_or(trimmed);
    let body = body.trim();

    if body.is_empty() {
        return LoopSubcommand::Help;
    }

    // List
    if body.eq_ignore_ascii_case("list") {
        return LoopSubcommand::List;
    }

    // Cancel <id>
    if let Some(rest) = body
        .strip_prefix("cancel ")
        .or_else(|| body.strip_prefix("CANCEL "))
    {
        let id = rest.trim();
        if !id.is_empty() {
            return LoopSubcommand::Cancel {
                id: id.to_string(),
            };
        }
    }
    if body.eq_ignore_ascii_case("cancel") {
        return LoopSubcommand::Help;
    }

    // Leading interval: first whitespace-split token matches `^(\d+)([smhd])$`
    let mut parts = body.split_whitespace();
    if let Some(first) = parts.next() {
        if let Some(caps) = leading_interval_re().captures(first) {
            let value: u32 = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(1);
            let unit = caps.get(2).map(|m| m.as_str()).unwrap_or("s");
            let interval_seconds = unit_to_seconds(value, unit);
            let remaining: Vec<&str> = parts.collect();
            let prompt = remaining.join(" ").trim().to_string();
            if !prompt.is_empty() {
                return LoopSubcommand::Add {
                    interval_seconds,
                    prompt,
                };
            }
        }
    }

    // Trailing interval: `<body> every N <unit>` at end of body
    if let Some(m) = trailing_interval_re().find(body) {
        let captured = m.as_str();
        if let Some(caps) = trailing_interval_re().captures(captured) {
            let value: u32 = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(1);
            let unit = caps.get(2).map(|m| m.as_str()).unwrap_or("s");
            let interval_seconds = unit_to_seconds(value, unit);
            let prompt = body[..m.start()].trim().to_string();
            if !prompt.is_empty() {
                return LoopSubcommand::Add {
                    interval_seconds,
                    prompt,
                };
            }
        }
    }

    // Default interval — full body is the prompt.
    LoopSubcommand::Add {
        interval_seconds: DEFAULT_INTERVAL_SECS,
        prompt: body.to_string(),
    }
}
