//! `remove-rule` shell-facing CLI bridge (RPC-279).
//!
//! Feature: spec/features/remove-rule-cli-subcommand.feature
//!
//! Two-front-doors: marshals positional args to JSON {workUnitId, index}
//! and delegates to codelet_fspec_core::commands::remove_rule::run.
//!
//! Exit-code contract: 0 on success, 1 on FspecCoreError. The success
//! line mirrors TS `output.log('✓ Removed rule: "<text>"')`. To produce
//! that line the bridge parses the JSON payload returned by the core
//! function — extracting `removedRule` is JSON marshalling, NOT domain
//! logic.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::remove_rule;
use serde_json::{json, Value};

use crate::common::render_core_error;

#[derive(Debug)]
pub struct CliArgs {
    pub work_unit_id: String,
    /// Raw positional arg as the user typed it. We forward to the core as a
    /// JSON Value (number when it parses as a finite i64, "NaN" string
    /// otherwise) so TS `parseInt('abc', 10) → NaN` semantics are preserved.
    pub index: String,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // TS `parseInt(index, 10)` parses a leading integer (with optional sign)
    // and returns `NaN` when no leading digits are present. Mirror that:
    //   - "0", "-3", "42"     → integer
    //   - "abc", "", "0.5x"   → NaN  (TS parseInt is permissive: "0.5x"→0,
    //     but "abc"→NaN). We use the simple leading-int parse which matches
    //     the canonical CLI usage; the NaN path is what the parity test
    //     exercises.
    let index_value: serde_json::Value = parse_ts_int_radix10(&args.index);

    let body = json!({
        "workUnitId": args.work_unit_id,
        "index": index_value,
    });
    let args_json = body.to_string();

    match remove_rule::run(&args_json, &project_root).await {
        Ok(data_json) => {
            // The core returns JSON {success, removedRule, remainingCount, [message]}.
            // The TS CLI prints `✓ Removed rule: "<removedRule>"` so we extract
            // removedRule from the JSON payload (marshalling, not domain logic).
            let removed_text = serde_json::from_str::<Value>(&data_json)
                .ok()
                .and_then(|v| {
                    v.get("removedRule")
                        .and_then(|s| s.as_str())
                        .map(str::to_string)
                })
                .unwrap_or_default();
            println!("✓ Removed rule: \"{removed_text}\"");
            Ok(0)
        }
        Err(err) => {
            // Mirror TS `output.error('✗ Failed to remove rule:', error.message)`
            // at src/commands/remove-rule.ts:102.
            eprintln!("✗ Failed to remove rule: {}", render_core_error(&err));
            Ok(1)
        }
    }
}

/// Mirror TS `parseInt(value, 10)` for the `index` positional. Returns the
/// parsed integer as a JSON number when the input matches `[-+]?\d+`,
/// otherwise returns the JSON string `"NaN"`. The core function compares
/// rule ids using strict equality, so a `"NaN"` value never matches an
/// integer id and the canonical not-found error is surfaced — byte-parity
/// with the TS CLI's behaviour on non-numeric input.
fn parse_ts_int_radix10(raw: &str) -> Value {
    // Drop a single leading sign if present; otherwise treat as positional.
    let trimmed = raw.trim_start();
    let (sign, rest) = match trimmed.chars().next() {
        Some('-') => (-1i64, &trimmed[1..]),
        Some('+') => (1i64, &trimmed[1..]),
        _ => (1i64, trimmed),
    };
    if rest.is_empty() || !rest.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return Value::String("NaN".into());
    }
    // Take the leading-digit run (TS parseInt('42xyz', 10) → 42).
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    match digits.parse::<i64>() {
        Ok(n) => Value::Number((sign * n).into()),
        Err(_) => Value::String("NaN".into()),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn parse_ts_int_radix10_returns_number_for_integer_strings() {
        assert_eq!(parse_ts_int_radix10("0"), Value::Number(0i64.into()));
        assert_eq!(parse_ts_int_radix10("42"), Value::Number(42i64.into()));
        assert_eq!(parse_ts_int_radix10("-3"), Value::Number((-3i64).into()));
    }

    #[test]
    fn parse_ts_int_radix10_returns_nan_string_for_non_numeric() {
        assert_eq!(parse_ts_int_radix10("abc"), Value::String("NaN".into()));
        assert_eq!(parse_ts_int_radix10(""), Value::String("NaN".into()));
        assert_eq!(parse_ts_int_radix10("xyz1"), Value::String("NaN".into()));
    }

    #[test]
    fn parse_ts_int_radix10_takes_leading_digit_run_like_ts() {
        // TS parseInt('42xyz', 10) === 42
        assert_eq!(parse_ts_int_radix10("42xyz"), Value::Number(42i64.into()));
    }
}
