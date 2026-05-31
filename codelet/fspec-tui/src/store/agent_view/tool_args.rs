//! Port of `src/tui/utils/toolFormatters.ts::extractToolArgsDisplay`.
//!
//! Feature: spec/features/agentview-chunkprocessor-parity.feature
//!
//! Collapses a tool's raw JSON input into a one-line summary suitable
//! for the `"● {ToolName}({argsDisplay})"` header rendered by
//! `chunk_to_message`. Per-tool dispatch matches the TS reference
//! so the Rust TUI displays the same first argument the Ink frontend
//! shows.

use serde_json::Value;

/// Collapse the raw JSON `input_json` for a `ToolCallInfo` to a
/// human-readable one-line summary keyed by `tool_name`.
///
/// On JSON parse failure the original `input_json` is returned
/// verbatim — matches the TS fallback at
/// `src/tui/utils/toolFormatters.ts::extractToolArgsDisplay`.
pub fn extract_tool_args_display(tool_name: &str, input_json: &str) -> String {
    let parsed: Value = match serde_json::from_str(input_json) {
        Ok(v) => v,
        Err(_) => return input_json.to_string(),
    };

    let obj = match parsed.as_object() {
        Some(o) => o,
        None => return input_json.to_string(),
    };

    // Tool-specific extraction matches the TS toolFormatters ladder.
    let key = match tool_name {
        "Bash" => "command",
        "Read" | "Write" | "Edit" | "MultiEdit" => "file_path",
        "Grep" | "Glob" => "pattern",
        "Fspec" => "command",
        "WebSearch" => "query",
        "WebFetch" => "url",
        "Task" => "description",
        // Default: first JSON value, rendered compactly.
        _ => {
            return obj
                .values()
                .next()
                .map(value_to_inline)
                .unwrap_or_else(|| input_json.to_string());
        }
    };

    obj.get(key)
        .map(value_to_inline)
        .unwrap_or_else(|| input_json.to_string())
}

/// Render a JSON value as a compact one-line string. Strings are
/// returned unquoted; everything else is serialised with `to_string`.
fn value_to_inline(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn bash_collapses_to_command_value() {
        let out = extract_tool_args_display("Bash", r#"{"command":"ls -la","timeout":5000}"#);
        assert_eq!(out, "ls -la");
    }

    #[test]
    fn fspec_collapses_to_command_value() {
        let out = extract_tool_args_display(
            "Fspec",
            r#"{"command":"show-work-unit","args":"{\"_\":[\"AUTH-001\"]}"}"#,
        );
        assert_eq!(out, "show-work-unit");
    }

    #[test]
    fn read_collapses_to_file_path() {
        let out = extract_tool_args_display("Read", r#"{"file_path":"/etc/hosts"}"#);
        assert_eq!(out, "/etc/hosts");
    }

    #[test]
    fn invalid_json_returns_raw_input() {
        let out = extract_tool_args_display("Bash", "not-json");
        assert_eq!(out, "not-json");
    }

    #[test]
    fn unknown_tool_returns_first_value() {
        let out = extract_tool_args_display("FooBar", r#"{"x":42,"y":"hi"}"#);
        // serde_json::Map preserves insertion order — the input has x
        // first, so the first value is 42 → "42".
        assert_eq!(out, "42");
    }
}
