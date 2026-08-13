//! `restore-rule` shell-facing CLI bridge (RPC-291).
//!
//! Feature: spec/features/restore-rule-cli-subcommand.feature
//!
//! Two-front-doors pattern (RPC-003 §7/§11):
//!   - Shell argv         → clap → this module → fspec_core::commands::restore_rule::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::restore_rule::run
//!
//! Exit-code contract:
//!   - 0 on success; the rendered text is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `✗ Failed to restore rule:`
//!     (parity with TS `output.error('✗ Failed to restore rule:', error.message)`
//!     at `src/commands/restore-rule.ts:139`).
//!
//! ## CLI surface asymmetry vs. dispatcher
//!
//! The captured TS `--help` fixture advertises a `--ids` bulk flag, but
//! `src/commands/restore-rule.ts`'s `registerRestoreRuleCommand` does NOT
//! wire it as a clap option (lines 122-142 — only the two positionals).
//! The flag is purely documentation. We mirror that verbatim: the clap
//! layer exposes only `<workUnitId>` and `<index>`, so `--ids` falls
//! through to clap's unknown-flag error (exit code 2). The bulk branch
//! exists in `restore_rule`'s core impl and is dispatcher-only — see
//! `rust/fspec-core/src/commands/restore_rule.rs::run_bulk`.
//!
//! ## TS `parseInt` parity for the `<index>` positional
//!
//! TS uses `parseInt(index, 10)` to coerce the positional string to an
//! integer. For non-numeric input (`"abc"`, `""`, `"xyz1"`) TS returns
//! `NaN`, and the downstream `find(r => r.id === NaN)` always returns
//! `undefined`, so the canonical `Rule with ID NaN not found` error is
//! surfaced (unless the rules array is empty, in which case the
//! `Work unit X has no rules` guard fires first). To preserve byte parity
//! we accept the index as a raw `String` at the clap layer and marshal it
//! through [`parse_ts_int_radix10`] — finite integers go through as JSON
//! numbers, everything else becomes the literal JSON string `"NaN"` which
//! the core deserialiser treats as TS NaN.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::restore_rule;
use serde_json::{json, Value};

use crate::common::render_core_error;

#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    /// Raw positional arg as the user typed it. We forward to the core as a
    /// JSON Value (number when it parses as a finite i64, "NaN" string
    /// otherwise) so TS `parseInt('abc', 10) → NaN` semantics are preserved.
    pub index: String,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let index_value: Value = parse_ts_int_radix10(&args.index);

    let body = json!({
        "workUnitId": args.work_unit_id,
        "index": index_value,
    });
    let args_json = body.to_string();

    match restore_rule::run(&args_json, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to restore rule: {}", render_core_error(&err));
            Ok(1)
        }
    }
}

/// Mirror TS `parseInt(value, 10)` for the `index` positional. Returns the
/// parsed integer as a JSON number when the input matches `[-+]?\d+...`,
/// otherwise returns the JSON string `"NaN"`. Inlined here (not factored
/// into a shared helper) per the worker playbook — sharing would couple
/// unrelated bridges and pull `parse_ts_int_radix10` into a common module
/// that none of the existing remove-* bridges agreed to depend on.
fn parse_ts_int_radix10(raw: &str) -> Value {
    let trimmed = raw.trim_start();
    let (sign, rest) = match trimmed.chars().next() {
        Some('-') => (-1i64, &trimmed[1..]),
        Some('+') => (1i64, &trimmed[1..]),
        _ => (1i64, trimmed),
    };
    if rest.is_empty() || !rest.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return Value::String("NaN".into());
    }
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
        assert_eq!(parse_ts_int_radix10("42xyz"), Value::Number(42i64.into()));
    }
}
