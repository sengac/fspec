//! Cron-expression validator — Rust port of the TS `cron-validate` npm package
//! as configured by `src/utils/validators/cron.ts`.
//!
//! The TS path validates a 5-field standard cron expression with the `default`
//! preset and all extended-syntax features disabled
//! (`useBlankDay`/`useLastDayOfMonth`/`useLastDayOfWeek`/`useNearestWeekday`/
//! `useNthWeekdayOfMonth` = false). The original Rust port leaned on `croner`,
//! whose error strings (`Component error: Number out of bounds.`) diverged from
//! the TS `cron-validate` strings (`Number '99' of minutes field is bigger than
//! upper limit '59'.`). This module reproduces the `cron-validate` field-by-field
//! validation EXACTLY so error messages reach byte-parity.
//!
//! Field limits for the `default` preset (`option.js` lines 53-77):
//!   minutes 0-59, hours 0-23, daysOfMonth 0-31, months 0-12, daysOfWeek 0-7.
//! `lowerLimit` resolves to `minValue` (0 for every field) so — mirroring the
//! `if (lowerLimit && number < lowerLimit)` guard where `0` is falsy — the lower
//! bound check is never applied. Only the upper-limit check fires.

/// The five cron field types in positional order, with their human label
/// (used verbatim in error messages) and upper limit. The lower limit for the
/// `default` preset is `0` for every field, and because `cron-validate` guards
/// the lower-bound check with `if (lowerLimit && ...)` (where `0` is falsy),
/// the lower bound is never enforced — matched here by omitting it.
const FIELD_LABELS: [&str; 5] = ["minutes", "hours", "daysOfMonth", "months", "daysOfWeek"];
const FIELD_UPPER: [i64; 5] = [59, 23, 31, 12, 7];

/// Validate a single trimmed 5-field cron expression with `cron-validate`'s
/// `default` preset semantics. Returns `Ok(())` on success, or the FIRST
/// error message (without the `(Input cron: '...')` suffix, which the caller
/// appends) on failure.
///
/// Multi-field validation in `cron-validate` collects every field's errors and
/// joins them; the TS `validateCronExpression` then renders them as
/// `errors.join('; ')`. We reproduce the full collected list so the caller can
/// join identically.
pub fn validate_default_5field(trimmed: &str) -> Result<(), Vec<String>> {
    // `cron-validate` splits the (already-trimmed) string on a single space,
    // NOT on whitespace runs. The caller (cron.ts) enforces the 5-field count
    // with a whitespace-run split BEFORE calling cron-validate, so a value with
    // collapsed multiple spaces can pass the pre-check yet fail here with
    // "Expected 5 values, but got N." — parity preserved by splitting on ' '.
    let parts: Vec<&str> = trimmed.split(' ').collect();
    if parts.len() != 5 {
        return Err(vec![format!(
            "Expected 5 values, but got {}.",
            parts.len()
        )]);
    }

    let mut errors: Vec<String> = Vec::new();
    for (idx, field) in parts.iter().enumerate() {
        if let Err(field_errors) = check_field(field, idx) {
            errors.extend(field_errors);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Mirror of `dayOfMonthChecker`/`dayOfWeekChecker`/`checkField` for the
/// `default` preset (all `use*` extended features disabled). The blank-day
/// `?` short-circuit is handled here for the day fields and rejected for the
/// non-day fields, exactly as `checkField` does.
fn check_field(field: &str, idx: usize) -> Result<(), Vec<String>> {
    let label = FIELD_LABELS[idx];

    // `?` blank-day handling (checkField, helper.js:228-238).
    if field == "?" {
        if idx == 2 || idx == 4 {
            // daysOfMonth / daysOfWeek, useBlankDay disabled.
            return Err(vec![format!(
                "useBlankDay is not enabled, but is used in {label} field."
            )]);
        }
        return Err(vec![format!(
            "blank notation is not allowed in {label} field."
        )]);
    }

    // Lists: split on ',', validate each element, collect every error.
    let mut errors: Vec<String> = Vec::new();
    for list_element in field.split(',') {
        if let Err(e) = check_list_element(list_element, idx) {
            errors.push(e);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Mirror of `checkListElement` (helper.js:158-214) for the `default` preset
/// (`allowStepping` true). Returns the first error for the element.
fn check_list_element(list_element: &str, idx: usize) -> Result<(), String> {
    let step_array: Vec<&str> = list_element.split('/').collect();
    if step_array.len() > 2 {
        return Err(format!(
            "List element '{list_element}' is not valid. (More than one '/')."
        ));
    }

    // First step element (the part before '/').
    check_first_step_element(step_array[0], idx)?;

    if step_array.len() == 2 {
        let second = step_array[1];
        if second.is_empty() {
            return Err(format!(
                "Second step element '{second}' of '{list_element}' is not valid (doesnt exist)."
            ));
        }
        let n = js_number(second);
        match n {
            None => {
                return Err(format!(
                    "Second step element '{second}' of '{list_element}' is not valid (not a number)."
                ));
            }
            Some(num) => {
                if num == 0.0 {
                    return Err(format!(
                        "Second step element '{second}' of '{list_element}' cannot be zero."
                    ));
                }
                if num < 0.0 {
                    return Err(format!(
                        "Second step element '{second}' of '{list_element}' cannot be negative."
                    ));
                }
                if num.fract() != 0.0 {
                    return Err(format!(
                        "Second step element '{second}' of '{list_element}' is not an integer."
                    ));
                }
                let upper = FIELD_UPPER[idx] as f64;
                // upperLimit is always truthy (>0) for these fields.
                if num > upper {
                    return Err(format!(
                        "Second step element '{second}' of '{list_element}' is bigger than the upper limit '{}'.",
                        FIELD_UPPER[idx]
                    ));
                }
                // Range/step combination check (helper.js:198-211).
                let range_array: Vec<&str> = step_array[0].split('-').collect();
                if range_array.len() == 2 {
                    let range_start = js_number(range_array[0]);
                    let range_end = js_number(range_array[1]);
                    if let (Some(rs), Some(re)) = (range_start, range_end) {
                        // num <= 0 already handled above.
                        let custom_range = re - rs + 1.0;
                        if num >= custom_range {
                            return Err(format!(
                                "Step value '{second}' is too large for the range '{}-{}'.",
                                fmt_num(rs),
                                fmt_num(re)
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Mirror of `checkFirstStepElement` (helper.js:134-157): handles a single
/// element or an `a-b` range.
fn check_first_step_element(first: &str, idx: usize) -> Result<(), String> {
    let label = FIELD_LABELS[idx];
    let range_array: Vec<&str> = first.split('-').collect();
    if range_array.len() > 2 {
        return Err(format!(
            "List element '{first}' is not valid. (More than one '-')."
        ));
    }
    if range_array.len() == 1 {
        return check_single_element(range_array[0], idx);
    }
    // length == 2 → a range.
    check_range_element(range_array[0], idx)?;
    check_range_element(range_array[1], idx)?;
    let lo = js_number(range_array[0]);
    let hi = js_number(range_array[1]);
    if let (Some(l), Some(h)) = (lo, hi) {
        if l > h {
            return Err(format!(
                "Lower range end '{}' is bigger than upper range end '{}' of {label} field.",
                range_array[0], range_array[1]
            ));
        }
    }
    Ok(())
}

/// Mirror of `checkRangeElement` (helper.js:118-133) for the `default` preset.
fn check_range_element(element: &str, idx: usize) -> Result<(), String> {
    let label = FIELD_LABELS[idx];
    if element == "*" {
        return Err(format!("'*' can't be part of a range in {label} field."));
    }
    if element.is_empty() {
        return Err(format!(
            "One of the range elements is empty in {label} field."
        ));
    }
    check_single_element_within_limits(element, idx)
}

/// Mirror of `checkSingleElement` (helper.js:56-117) for the `default` preset.
fn check_single_element(element: &str, idx: usize) -> Result<(), String> {
    let label = FIELD_LABELS[idx];
    if element == "*" {
        // Wildcard always fits the default full range.
        return Ok(());
    }
    if element.is_empty() {
        return Err(format!("One of the elements is empty in {label} field."));
    }
    check_single_element_within_limits(element, idx)
}

/// Mirror of `checkSingleElementWithinLimits` (helper.js:27-55) for the
/// `default` preset. `useAliases` is false so month/day-of-week aliases are
/// rejected (they parse as NaN → invalid).
fn check_single_element_within_limits(element: &str, idx: usize) -> Result<(), String> {
    let label = FIELD_LABELS[idx];
    match js_number(element) {
        None => Err(format!("Element '{element}' of {label} field is invalid.")),
        Some(num) => {
            if num.fract() != 0.0 {
                return Err(format!(
                    "Element '{element}' of {label} field is not an integer."
                ));
            }
            // lowerLimit is 0 (falsy) → lower-bound check skipped (parity).
            let upper = FIELD_UPPER[idx];
            if num > upper as f64 {
                return Err(format!(
                    "Number '{}' of {label} field is bigger than upper limit '{upper}'.",
                    fmt_num(num)
                ));
            }
            Ok(())
        }
    }
}

/// JavaScript `Number(s)` semantics, restricted to the cases `cron-validate`
/// relies on. Returns `None` when JS `Number(s)` would be `NaN`.
///
/// JS specifics replicated:
///   * Leading/trailing ASCII whitespace is trimmed.
///   * Empty string → `0` (NOT NaN).
///   * Optional leading `+`/`-`.
///   * Hex (`0x`), binary (`0b`), octal (`0o`) integer literals.
///   * Decimal integers and floats, including exponent form (`1e2`).
///   * `Infinity` / `-Infinity`.
fn js_number(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        return Some(0.0);
    }
    // Handle sign for the special radix / Infinity forms.
    let (sign, body) = match t.strip_prefix('-') {
        Some(rest) => (-1.0_f64, rest),
        None => match t.strip_prefix('+') {
            Some(rest) => (1.0_f64, rest),
            None => (1.0_f64, t),
        },
    };

    if body == "Infinity" {
        return Some(sign * f64::INFINITY);
    }

    // Radix-prefixed integer literals (only valid without a sign in JS for
    // 0x/0o/0b, but JS `Number('-0x5')` is NaN — so only accept when no sign
    // was consumed, i.e. sign came from the unsigned branch). To match JS,
    // radix literals must be the full (unsigned) token.
    if t == body {
        if let Some(hex) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
            return i64::from_str_radix(hex, 16).ok().map(|v| v as f64);
        }
        if let Some(oct) = body.strip_prefix("0o").or_else(|| body.strip_prefix("0O")) {
            return i64::from_str_radix(oct, 8).ok().map(|v| v as f64);
        }
        if let Some(bin) = body.strip_prefix("0b").or_else(|| body.strip_prefix("0B")) {
            return i64::from_str_radix(bin, 2).ok().map(|v| v as f64);
        }
    }

    // Plain decimal / float / exponent. Rust's f64 parse is close enough to JS
    // for the numeric tokens cron expressions use; reject tokens Rust accepts
    // but JS would not (e.g. "inf", "nan").
    let lower = t.to_ascii_lowercase();
    if lower.contains("inf") || lower.contains("nan") {
        return None;
    }
    t.parse::<f64>().ok()
}

/// Format a JS number the way `String(number)` / template interpolation would
/// for the integer values used in cron error messages (no trailing `.0`).
fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn first_err(expr: &str) -> Option<String> {
        validate_default_5field(expr).err().map(|v| v[0].clone())
    }

    #[test]
    fn accepts_valid_expressions() {
        assert!(validate_default_5field("0 2 * * *").is_ok());
        assert!(validate_default_5field("30 6 * * 1-5").is_ok());
        assert!(validate_default_5field("* * * * *").is_ok());
        assert!(validate_default_5field("*/2 * * * *").is_ok());
        assert!(validate_default_5field("5,10,15 * * * *").is_ok());
        assert!(validate_default_5field("0 2 * * 7").is_ok());
        assert!(validate_default_5field("0 2 * * 0").is_ok());
    }

    #[test]
    fn upper_limit_messages_match_ts() {
        assert_eq!(
            first_err("99 2 * * *").unwrap(),
            "Number '99' of minutes field is bigger than upper limit '59'."
        );
        assert_eq!(
            first_err("0 99 * * *").unwrap(),
            "Number '99' of hours field is bigger than upper limit '23'."
        );
        assert_eq!(
            first_err("0 2 99 * *").unwrap(),
            "Number '99' of daysOfMonth field is bigger than upper limit '31'."
        );
        assert_eq!(
            first_err("0 2 * 99 *").unwrap(),
            "Number '99' of months field is bigger than upper limit '12'."
        );
        assert_eq!(
            first_err("0 2 * * 8").unwrap(),
            "Number '8' of daysOfWeek field is bigger than upper limit '7'."
        );
    }

    #[test]
    fn invalid_element_messages_match_ts() {
        assert_eq!(
            first_err("abc 2 * * *").unwrap(),
            "Element 'abc' of minutes field is invalid."
        );
        assert_eq!(
            first_err("0 2 * JAN *").unwrap(),
            "Element 'JAN' of months field is invalid."
        );
        assert_eq!(
            first_err("0 2 * * MON").unwrap(),
            "Element 'MON' of daysOfWeek field is invalid."
        );
        assert_eq!(
            first_err("0 2 L * *").unwrap(),
            "Element 'L' of daysOfMonth field is invalid."
        );
        assert_eq!(
            first_err("1.5 2 * * *").unwrap(),
            "Element '1.5' of minutes field is not an integer."
        );
    }

    #[test]
    fn range_and_step_messages_match_ts() {
        assert_eq!(
            first_err("5-3 2 * * *").unwrap(),
            "Lower range end '5' is bigger than upper range end '3' of minutes field."
        );
        assert_eq!(
            first_err("*/0 2 * * *").unwrap(),
            "Second step element '0' of '*/0' cannot be zero."
        );
        assert_eq!(
            first_err("*/70 2 * * *").unwrap(),
            "Second step element '70' of '*/70' is bigger than the upper limit '59'."
        );
        assert_eq!(
            first_err("5/2/3 * * * *").unwrap(),
            "List element '5/2/3' is not valid. (More than one '/')."
        );
        assert_eq!(
            first_err("1-2-3 * * * *").unwrap(),
            "List element '1-2-3' is not valid. (More than one '-')."
        );
        assert_eq!(
            first_err("10-20/11 * * * *").unwrap(),
            "Step value '11' is too large for the range '10-20'."
        );
        assert_eq!(
            first_err("5- 2 * * *").unwrap(),
            "One of the range elements is empty in minutes field."
        );
    }

    #[test]
    fn empty_elements_match_ts() {
        assert_eq!(
            first_err("5,, * * * *").unwrap(),
            "One of the elements is empty in minutes field."
        );
        assert_eq!(
            first_err(",5 * * * *").unwrap(),
            "One of the elements is empty in minutes field."
        );
        assert_eq!(
            first_err("/5 * * * *").unwrap(),
            "One of the elements is empty in minutes field."
        );
        assert_eq!(
            first_err("5/ * * * *").unwrap(),
            "Second step element '' of '5/' is not valid (doesnt exist)."
        );
    }

    #[test]
    fn blank_day_messages_match_ts() {
        assert_eq!(
            first_err("0 2 ? * *").unwrap(),
            "useBlankDay is not enabled, but is used in daysOfMonth field."
        );
        assert_eq!(
            first_err("? 2 * * *").unwrap(),
            "blank notation is not allowed in minutes field."
        );
    }

    #[test]
    fn collapsed_spaces_yield_expected_values_error() {
        // The caller's whitespace-run pre-check passes (5 fields), but the
        // single-space split here counts 9.
        let errs = validate_default_5field("0  2  *  *  *").unwrap_err();
        assert_eq!(errs[0], "Expected 5 values, but got 9.");
    }

    #[test]
    fn js_number_semantics() {
        assert_eq!(js_number("5"), Some(5.0));
        assert_eq!(js_number("+5"), Some(5.0));
        assert_eq!(js_number(""), Some(0.0));
        assert_eq!(js_number("0x5"), Some(5.0));
        assert_eq!(js_number("0b101"), Some(5.0));
        assert_eq!(js_number("1e2"), Some(100.0));
        assert_eq!(js_number("abc"), None);
        assert_eq!(js_number("1.5"), Some(1.5));
    }
}
