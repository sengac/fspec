//! `remove-architecture-note` shell-facing CLI bridge (RPC-267).
//!
//! Feature: spec/features/remove-architecture-note-cli-subcommand.feature
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::remove_architecture_note::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::remove_architecture_note::run
//!
//! Exit-code contract:
//!   - 0 on success (including the idempotent already-deleted path).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `Error:` mirroring TS at
//!     `src/commands/remove-architecture-note.ts:101-107`.
//!
//! The bridge converts the raw positional `<index>` string into a JSON
//! number (or the literal string `"NaN"` for non-numeric input) so the
//! core function can surface the canonical NaN-not-found error —
//! byte-parity with TS `parseInt(_, 10)` semantics.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::remove_architecture_note;
use serde_json::{json, Value};

use crate::common::render_core_error;

/// Strongly-typed args mirrored from the TypeScript Commander.js
/// registration at `src/commands/remove-architecture-note.ts:88-107`.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
    /// Raw positional `<index>` as the user typed it. Forwarded to the
    /// core as a JSON Value (number when it parses as a finite i64,
    /// `"NaN"` string otherwise) so TS `parseInt('abc', 10) → NaN`
    /// semantics are preserved.
    pub index: String,
}

/// Entry point invoked from `main.rs` for the `remove-architecture-note`
/// clap subcommand.
pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    let index_value: Value = parse_ts_int_radix10(&args.index);

    let mut obj = serde_json::Map::new();
    obj.insert("workUnitId".to_string(), json!(args.work_unit_id));
    obj.insert("index".to_string(), index_value);
    let args_json = Value::Object(obj).to_string();

    match remove_architecture_note::run(&args_json, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}

/// Mirror TS `parseInt(value, 10)` for the `<index>` positional.
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
    }
}
