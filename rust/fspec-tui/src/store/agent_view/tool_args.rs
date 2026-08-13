//! Port of `src/tui/utils/chunkProcessor.ts::extractToolArgsDisplay`.
//!
//! Feature: spec/features/tool-call-argument-header.feature
//!
//! Collapses a tool's raw JSON input into a one-line summary suitable
//! for the `"● {ToolName}({argsDisplay})"` header rendered by
//! `chunk_to_message`. Mirrors the TS three-branch algorithm:
//! Edit/Write family → `file_path` only; tools with a `command`/
//! `action_type` key → command first then remaining params; otherwise
//! all params as `{ key: value, ... }`. Every value is capped at 100
//! characters with a literal `...` suffix.

use serde_json::Value;
use unicode_width::UnicodeWidthStr;

/// Collapse the raw JSON `input_json` for a `ToolCallInfo` to a
/// human-readable one-line summary keyed by `tool_name`.
///
/// On JSON parse failure the original `input_json` is returned
/// verbatim — matches the TS fallback at
/// `src/tui/utils/chunkProcessor.ts::extractToolArgsDisplay`.
pub fn extract_tool_args_display(tool_name: &str, input_json: &str) -> String {
    let parsed: Value = match serde_json::from_str(input_json) {
        Ok(v) => v,
        Err(_) => return input_json.to_string(),
    };

    let obj = match parsed.as_object() {
        Some(o) => o,
        None => return input_json.to_string(),
    };

    // Branch 1 — Edit/Write family → file_path only (content shown as diff).
    let tool_lower = tool_name.to_lowercase();
    if matches!(
        tool_lower.as_str(),
        "edit" | "replace" | "write" | "write_file"
    ) {
        return match obj.get("file_path") {
            Some(Value::String(s)) => s.clone(),
            Some(other) if !other.is_null() => value_to_plain(other),
            _ => String::new(),
        };
    }

    // Branch 2 — has `command` (else `action_type`) key → command first,
    // then remaining params as `, { key: value, ... }`.
    let command_key = if obj.contains_key("command") {
        Some("command")
    } else if obj.contains_key("action_type") {
        Some("action_type")
    } else {
        None
    };

    if let Some(command_key) = command_key {
        let command = obj.get(command_key).map(value_to_plain).unwrap_or_default();
        let parts: Vec<String> = obj
            .iter()
            .filter(|(key, _)| key.as_str() != command_key)
            .map(|(key, value)| format_part(key, value))
            .collect();

        if parts.is_empty() {
            return command;
        }
        return format!("{command}, {{ {} }}", parts.join(", "));
    }

    // Branch 3 — default → all params as `{ key: value, ... }`.
    if obj.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = obj
        .iter()
        .map(|(key, value)| format_part(key, value))
        .collect();
    format!("{{ {} }}", parts.join(", "))
}

/// Render a JSON value as a bare string for the command position
/// (mirrors TS `String(value)`: strings verbatim, everything else
/// via compact JSON).
fn value_to_plain(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Format a single `key: value` part using the TS value formatter:
/// strings are single-quoted, `null` renders bare, everything else uses
/// compact JSON. Every value is capped at 100 characters with a literal
/// `...` suffix when longer (char-boundary-safe).
fn format_part(key: &str, value: &Value) -> String {
    match value {
        Value::String(s) => format!("{key}: '{}'", cap_value(s)),
        Value::Null => format!("{key}: null"),
        other => format!("{key}: {}", cap_value(&other.to_string())),
    }
}

/// Cap a display value at 100 characters, appending `...` when longer.
/// Uses `chars().take(100)` for char-boundary safety on multi-byte UTF-8.
fn cap_value(s: &str) -> String {
    if s.width() > 100 {
        let truncated: String = s.chars().take(100).collect();
        format!("{truncated}...")
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    // @step Given a tool call for "Edit" with input '{"file_path":"/a.rs","old_string":"x","new_string":"y"}'
    // @step When the tool-call argument header is extracted
    // @step Then the header args are "/a.rs"
    #[test]
    fn edit_family_shows_only_file_path() {
        let out = extract_tool_args_display(
            "Edit",
            r#"{"file_path":"/a.rs","old_string":"x","new_string":"y"}"#,
        );
        assert_eq!(out, "/a.rs");
    }

    // @step Given a tool call for "Write" with input '{"content":"..."}'
    // @step When the tool-call argument header is extracted
    // @step Then the header args are ""
    #[test]
    fn write_family_without_file_path_is_empty() {
        let out = extract_tool_args_display("Write", r#"{"content":"..."}"#);
        assert_eq!(out, "");
    }

    // @step Given a tool call for "Bash" with input '{"command":"ls -la","timeout":5000}'
    // @step When the tool-call argument header is extracted
    // @step Then the header args are "ls -la, { timeout: 5000 }"
    #[test]
    fn command_tool_shows_command_then_remaining_params() {
        let out = extract_tool_args_display("Bash", r#"{"command":"ls -la","timeout":5000}"#);
        assert_eq!(out, "ls -la, { timeout: 5000 }");
    }

    // @step Given a tool call for "WebSearch" with input '{"action_type":"search","query":"hi"}'
    // @step When the tool-call argument header is extracted
    // @step Then the header args are "search, { query: 'hi' }"
    #[test]
    fn action_type_tool_shows_action_then_remaining_params() {
        let out =
            extract_tool_args_display("WebSearch", r#"{"action_type":"search","query":"hi"}"#);
        assert_eq!(out, "search, { query: 'hi' }");
    }

    // @step Given a tool call for "Grep" with input '{"pattern":"foo","glob":"*.rs"}'
    // @step When the tool-call argument header is extracted
    // @step Then the header args are "{ pattern: 'foo', glob: '*.rs' }"
    #[test]
    fn no_command_tool_shows_all_params_as_object() {
        let out = extract_tool_args_display("Grep", r#"{"pattern":"foo","glob":"*.rs"}"#);
        assert_eq!(out, "{ pattern: 'foo', glob: '*.rs' }");
    }

    // @step Given a tool call for "Grep" with a single param whose string value is 120 characters long
    // @step When the tool-call argument header is extracted
    // @step Then the value is the first 100 characters followed by "..."
    #[test]
    fn long_value_is_capped_with_ellipsis() {
        let long = "a".repeat(120);
        let input = format!(r#"{{"pattern":"{long}"}}"#);
        let out = extract_tool_args_display("Grep", &input);
        let expected_value = format!("{}...", "a".repeat(100));
        assert_eq!(out, format!("{{ pattern: '{expected_value}' }}"));
    }

    // @step Given a tool call for "Bash" with input "not-json"
    // @step When the tool-call argument header is extracted
    // @step Then the header args are "not-json"
    #[test]
    fn invalid_json_returns_raw_input() {
        let out = extract_tool_args_display("Bash", "not-json");
        assert_eq!(out, "not-json");
    }

    #[test]
    fn unknown_tool_preserves_insertion_order() {
        // serde_json::Map (preserve_order) keeps insertion order; x first.
        let out = extract_tool_args_display("FooBar", r#"{"x":42,"y":"hi"}"#);
        assert_eq!(out, "{ x: 42, y: 'hi' }");
    }
}
