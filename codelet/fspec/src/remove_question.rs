//! `remove-question` shell-facing CLI bridge (RPC-278).
//!
//! Feature: spec/features/remove-question-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4
//! derive as the Commander.js equivalent. This module is the thin
//! façade that parses argv (the `Mode::RemoveQuestion` clap variant in
//! [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::remove_question::run`] — the SAME
//! function the LLM-facing dispatcher invokes.
//!
//! NO logic in the bridge — JSON marshalling + a fixed success line
//! sourced from the dispatcher result.
//!
//! Exit-code contract: 0 on success, 1 on FspecCoreError. Errors are
//! written to stderr prefixed with '✗ Failed to remove question:'
//! mirroring TS at `src/commands/remove-question.ts:106` —
//! `output.error('✗ Failed to remove question:', error.message)`.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::remove_question;
use serde_json::{json, Value};

use crate::common::render_core_error;

#[derive(Debug)]
pub struct CliArgs {
    pub work_unit_id: String,
    /// Raw `--index` flag as the user typed it. We forward to the core as
    /// a JSON Value (number when it parses as a finite i64, "NaN" string
    /// otherwise) so TS `parseInt('abc', 10) → NaN` semantics are
    /// preserved through to the canonical "Question with ID NaN not
    /// found" message.
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

    match remove_question::run(&args_json, &project_root).await {
        Ok(data) => {
            // Parse the dispatcher JSON to extract `removedQuestion`,
            // mirroring TS at `src/commands/remove-question.ts:100-102`
            // which prints `✓ Removed question: "${result.removedQuestion}"`.
            let parsed: Value = serde_json::from_str(&data).unwrap_or(Value::Null);
            let text = parsed
                .get("removedQuestion")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            println!("✓ Removed question: \"{text}\"");
            Ok(0)
        }
        Err(err) => {
            eprintln!("✗ Failed to remove question: {}", render_core_error(&err));
            Ok(1)
        }
    }
}

/// Mirror TS `parseInt(value, 10)` for the `--index` flag. Returns the
/// parsed integer as a JSON number when the input matches `[-+]?\d+`,
/// otherwise returns the JSON string `"NaN"`. The core function compares
/// question ids using strict equality, so a `"NaN"` value never matches
/// an integer id and the canonical not-found error is surfaced —
/// byte-parity with the TS CLI's behaviour on non-numeric input.
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
}
