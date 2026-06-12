//! JavaScript-compatibility shims for byte-for-byte parity with the
//! TypeScript fspec CLI.
//!
//! ## Why this module exists — Commander.js `parseInt` coercion
//!
//! Every Event-Storm `add-*` command registers its `--timestamp` option with
//! the global `parseInt` function as the coercion callback, e.g.
//! `.option('--timestamp <ms>', '…', parseInt)` at
//! `src/commands/add-domain-event.ts:166` (identically in `add-aggregate.ts`,
//! `add-command.ts`, `add-hotspot.ts`, `add-bounded-context.ts`,
//! `add-external-system.ts`, `add-policy.ts`). Commander invokes the parser
//! as `parseInt(value, previousValue)`; on the sole occurrence
//! `previousValue` is `undefined`, so the radix defaults to 10.
//!
//! The command body then persists the result with
//! `if (options.timestamp !== undefined) item.timestamp = options.timestamp;`
//! — so a `NaN` result (e.g. `--timestamp abc`) is STILL assigned to the
//! field, and `JSON.stringify(NaN)` serialises it as `null`. The Rust port
//! must therefore write a `Number` for a parseable value and `null` for an
//! unparseable one, NOT reject the input (clap's default `i64` parser exits
//! with code 2, which the TS CLI never does).
//!
//! [`parse_js_int`] reproduces `parseInt(value, 10)` exactly. It is invoked by
//! the CLI bridges (the Commander analog) to coerce the raw `--timestamp`
//! string into a persisted JSON `Number`/`null` before it reaches the core
//! command. The LLM dispatch path passes a JSON `Number` directly and is left
//! untouched, so no double-coercion occurs.

use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// Reproduce JavaScript `parseInt(raw, 10)` and map the result onto a
/// [`serde_json::Value`]:
///
/// * A successfully parsed integer → [`Value::Number`].
/// * `NaN` (no leading decimal digit after the optional sign) → [`Value::Null`]
///   — matching `JSON.stringify(NaN) === "null"`.
///
/// Algorithm (mirrors the ECMAScript `parseInt` spec for radix 10): strip
/// leading whitespace, accept an optional `+`/`-` sign, then consume the
/// leading run of ASCII decimal digits; any trailing non-digit characters
/// are ignored (`"12abc"` → `12`).
pub fn parse_js_int(raw: &str) -> Value {
    // JS `parseInt` strips leading StrWhiteSpace before scanning.
    let s = raw.trim_start();
    let bytes = s.as_bytes();
    let mut idx = 0;
    let mut negative = false;
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        negative = bytes[idx] == b'-';
        idx += 1;
    }
    let digits_start = idx;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }
    if idx == digits_start {
        // No digits consumed → NaN → JSON `null`.
        return Value::Null;
    }
    let digits = &s[digits_start..idx];
    // Fast path: fits in i64 (covers every realistic millisecond timestamp).
    if let Ok(n) = digits.parse::<i64>() {
        let signed = if negative { -n } else { n };
        return Value::from(signed);
    }
    // Overflow: JS `parseInt` yields a (lossy) float for very large inputs.
    // Mirror the Number result rather than collapsing to null.
    if let Ok(f) = digits.parse::<f64>() {
        let signed = if negative { -f } else { f };
        return Value::from(signed);
    }
    Value::Null
}

/// Deserialize a `timestamp`-style field that must preserve the distinction
/// between an ABSENT key (→ `None`) and a key PRESENT with an explicit JSON
/// `null` (→ `Some(Value::Null)`).
///
/// A plain `Option<Value>` collapses a present `null` to `None`, which would
/// make the core omit the field entirely. The TS CLI instead writes
/// `"timestamp": null` whenever Commander's `parseInt` produced `NaN`, so the
/// present-`null` state must survive to the on-disk item.
///
/// Pair with `#[serde(default, deserialize_with = "…")]`: serde only invokes
/// this function when the key is present, so an absent key falls through to
/// the `Default` (`None`) while a present value — including `null` — is
/// captured verbatim as `Some(value)`.
pub fn deserialize_present_value<'de, D>(deserializer: D) -> Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    Value::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn parses_plain_integer() {
        assert_eq!(parse_js_int("12"), Value::from(12));
    }

    #[test]
    fn parses_leading_digits_ignoring_trailing() {
        assert_eq!(parse_js_int("12abc"), Value::from(12));
    }

    #[test]
    fn non_numeric_yields_null() {
        assert_eq!(parse_js_int("abc"), Value::Null);
    }

    #[test]
    fn empty_string_yields_null() {
        assert_eq!(parse_js_int(""), Value::Null);
    }

    #[test]
    fn parses_negative_integer() {
        assert_eq!(parse_js_int("-5"), Value::from(-5));
    }

    #[test]
    fn trims_leading_whitespace() {
        assert_eq!(parse_js_int("  7"), Value::from(7));
    }

    #[test]
    fn parses_explicit_positive_sign() {
        assert_eq!(parse_js_int("+42"), Value::from(42));
    }

    #[test]
    fn stops_at_decimal_point() {
        assert_eq!(parse_js_int("3.9"), Value::from(3));
    }

    #[test]
    fn sign_only_yields_null() {
        assert_eq!(parse_js_int("-"), Value::Null);
    }

    #[derive(Deserialize)]
    struct Holder {
        #[serde(default, deserialize_with = "deserialize_present_value")]
        timestamp: Option<Value>,
    }

    #[test]
    fn present_value_distinguishes_absent_null_and_number() {
        let absent: Holder = serde_json::from_str("{}").unwrap();
        assert_eq!(absent.timestamp, None);

        let null: Holder = serde_json::from_str(r#"{"timestamp":null}"#).unwrap();
        assert_eq!(null.timestamp, Some(Value::Null));

        let number: Holder = serde_json::from_str(r#"{"timestamp":1000}"#).unwrap();
        assert_eq!(number.timestamp, Some(Value::from(1000)));
    }
}
