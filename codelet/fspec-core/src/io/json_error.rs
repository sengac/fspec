//! Shared `serde_json` parse-error diagnostics (RPC-334).
//!
//! Every canonical fspec state file (`work-units.json`, `foundation.json`,
//! `prefixes.json`, `epics.json`, `tags.json`, …) is JSON. When one of them is
//! corrupted, a bare `serde_json::Error::to_string()` (e.g. `key must be a
//! string at line 4 column 37`) tells the user *what* went wrong but not
//! *where* in the file to look. This module routes those errors through the
//! vendored [`codelet_fspec_json_error::SerdeError`] formatter, which renders
//! the offending line(s) with a caret under the exact error column.
//!
//! Two entry points:
//!
//! * [`parse_json_diagnostic`] — wraps the rendered snippet as
//!   [`FspecCoreError::ParseJson`] for the ~80 sites that name a state file.
//! * [`parse_json_reason`] — returns just the rendered snippet, for the
//!   `InvalidArgs`/`String`-wrapping call sites (Groups 1, 3, 4 of the
//!   RPC-334 inventory) that embed a command-specific outer prefix.
//!
//! This is a deliberate, documented divergence from the TypeScript frontend
//! (which surfaces V8/`JSON.parse` wording pinned to whatever Node version it
//! runs on). Diagnostic quality, not byte-for-byte V8 parity, is the goal.

use codelet_fspec_json_error::SerdeError;

use crate::error::FspecCoreError;

/// Build a [`FspecCoreError::ParseJson`] whose `reason` is the caret-pointed
/// snippet rendered from `input` + `err`.
///
/// `file_label` MUST match the TS file name (e.g. `"work-units.json"`) so the
/// `"Failed to parse <file>"` framing keeps cross-frontend assertions working.
#[must_use]
pub fn parse_json_diagnostic(
    file_label: &str,
    input: &str,
    err: &serde_json::Error,
) -> FspecCoreError {
    FspecCoreError::ParseJson {
        file: file_label.to_string(),
        reason: parse_json_reason(input, err),
    }
}

/// Render just the caret-pointed snippet (no file framing) for call sites that
/// embed the body inside a command-specific wrapper.
#[must_use]
pub fn parse_json_reason(input: &str, err: &serde_json::Error) -> String {
    SerdeError::from_json(input.to_string(), err).to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn json_err(input: &str) -> serde_json::Error {
        serde_json::from_str::<serde_json::Value>(input).unwrap_err()
    }

    #[test]
    fn diagnostic_names_the_file_and_keeps_corruption_guidance() {
        let input = "{ bad";
        let err = json_err(input);
        let msg = parse_json_diagnostic("work-units.json", input, &err).to_string();
        assert!(
            msg.contains("Failed to parse work-units.json"),
            "missing file framing: {msg}"
        );
        assert!(
            msg.contains("The file may be corrupted or contain invalid JSON."),
            "missing corruption guidance: {msg}"
        );
    }

    #[test]
    fn diagnostic_includes_caret_snippet() {
        let input = "{ bad";
        let err = json_err(input);
        let msg = parse_json_diagnostic("work-units.json", input, &err).to_string();
        assert!(msg.contains("1 | { bad"), "missing source line: {msg}");
        assert!(msg.contains('^'), "missing caret: {msg}");
        assert!(
            msg.contains("line 1 column 3"),
            "missing serde position: {msg}"
        );
    }

    #[test]
    fn reason_is_just_the_snippet_without_file_framing() {
        let input = "{ bad";
        let err = json_err(input);
        let reason = parse_json_reason(input, &err);
        assert!(
            !reason.contains("Failed to parse"),
            "reason must not carry file framing: {reason}"
        );
        assert!(reason.contains('^'), "missing caret: {reason}");
        assert!(
            reason.contains("line 1 column 3"),
            "missing serde position: {reason}"
        );
    }

    #[test]
    fn multiline_input_points_at_the_offending_line() {
        let input =
            "{\n  \"version\": \"0.7.1\",\n  \"workUnits\": {\n    \"AUTH-001\": { \"id\": \"x\", status: \"done\" }\n  }\n}";
        let err = json_err(input);
        let reason = parse_json_reason(input, &err);
        assert!(reason.contains("4 |"), "missing numbered error line: {reason}");
        assert!(reason.contains("status:"), "missing offending content: {reason}");
        assert!(
            reason.contains("key must be a string at line 4 column"),
            "missing serde position: {reason}"
        );
    }

    #[test]
    fn never_fabricates_a_v8_unexpected_token_prefix() {
        let input = "{ bad";
        let err = json_err(input);
        let reason = parse_json_reason(input, &err);
        assert!(
            !reason.contains("Unexpected token in JSON"),
            "must not emit V8 wording: {reason}"
        );
    }
}
