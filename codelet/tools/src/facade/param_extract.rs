//! Shared parameter extraction helpers for tool facades.
//!
//! These functions provide consistent extraction of required/optional fields
//! from JSON input across all provider-specific facades (Codex, Z.AI, etc.).
//!
//! Numeric and boolean extractors are lenient: they accept both native JSON types
//! AND string representations (e.g., `"10"` for integers, `"true"` for booleans)
//! to tolerate lower-powered LLMs that send incorrect types.

use crate::ToolError;
use serde_json::Value;

/// Coerce a `serde_json::Value` to `u64`, accepting both JSON numbers and numeric strings.
/// Returns `None` for null, missing, or non-parseable values.
pub fn value_as_u64_lenient(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.trim().parse::<u64>().ok(),
        _ => None,
    }
}

/// Coerce a `serde_json::Value` to `bool`, accepting JSON booleans, boolean strings, and numeric 0/1.
/// Returns `None` for null, missing, or non-parseable values.
pub fn value_as_bool_lenient(v: &Value) -> Option<bool> {
    match v {
        Value::Bool(b) => Some(*b),
        Value::String(s) => match s.trim().to_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        },
        Value::Number(n) => n.as_u64().map(|n| n != 0),
        _ => None,
    }
}

/// Extract a required non-empty string field from JSON input.
/// Returns an error if the field is missing, null, or empty.
pub fn extract_required_string(input: &Value, field: &str, tool: &'static str) -> Result<String, ToolError> {
    let value = input
        .get(field)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ToolError::Validation {
            tool,
            message: format!("Missing or empty required '{field}' field"),
        })?;
    Ok(value.to_string())
}

/// Extract an optional string field from JSON input.
/// Returns None if the field is missing, null, or empty.
pub fn extract_optional_string(input: &Value, field: &str) -> Option<String> {
    input
        .get(field)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
}

/// Extract an optional unsigned integer field from JSON input.
/// Accepts both native JSON numbers and numeric strings (e.g., `"10"` → `10`).
/// Returns None if the field is missing, null, or not parseable as a non-negative integer.
pub fn extract_optional_uint(input: &Value, field: &str) -> Option<usize> {
    input.get(field).and_then(|v| match v {
        Value::Number(n) => n.as_u64().map(|n| n as usize),
        Value::String(s) => s.trim().parse::<usize>().ok(),
        _ => None,
    })
}

/// Extract an optional boolean field from JSON input.
/// Accepts native JSON booleans, boolean strings (`"true"`, `"false"`, `"yes"`, `"no"`),
/// and numeric `0`/`1`.
/// Returns None if the field is missing, null, or not parseable as a boolean.
pub fn extract_optional_bool(input: &Value, field: &str) -> Option<bool> {
    input.get(field).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        Value::String(s) => match s.trim().to_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        },
        Value::Number(n) => n.as_u64().map(|n| n != 0),
        _ => None,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use serde_json::json;

    // =========================================================================
    // extract_required_string tests
    // =========================================================================

    #[test]
    fn test_extract_required_string_present() {
        let input = json!({"name": "hello"});
        let result = extract_required_string(&input, "name", "test_tool").unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_extract_required_string_missing() {
        let input = json!({});
        let result = extract_required_string(&input, "name", "test_tool");
        assert!(result.is_err());
        if let Err(ToolError::Validation { tool, message }) = result {
            assert_eq!(tool, "test_tool");
            assert!(message.contains("name"));
        }
    }

    #[test]
    fn test_extract_required_string_null() {
        let input = json!({"name": null});
        let result = extract_required_string(&input, "name", "test_tool");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_required_string_empty() {
        let input = json!({"name": ""});
        let result = extract_required_string(&input, "name", "test_tool");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_required_string_non_string() {
        let input = json!({"name": 42});
        let result = extract_required_string(&input, "name", "test_tool");
        assert!(result.is_err());
    }

    // =========================================================================
    // extract_optional_string tests
    // =========================================================================

    #[test]
    fn test_extract_optional_string_present() {
        let input = json!({"path": "/src"});
        assert_eq!(extract_optional_string(&input, "path"), Some("/src".to_string()));
    }

    #[test]
    fn test_extract_optional_string_missing() {
        let input = json!({});
        assert_eq!(extract_optional_string(&input, "path"), None);
    }

    #[test]
    fn test_extract_optional_string_null() {
        let input = json!({"path": null});
        assert_eq!(extract_optional_string(&input, "path"), None);
    }

    #[test]
    fn test_extract_optional_string_empty() {
        let input = json!({"path": ""});
        assert_eq!(extract_optional_string(&input, "path"), None);
    }

    // =========================================================================
    // extract_optional_uint tests
    // =========================================================================

    #[test]
    fn test_extract_optional_uint_present() {
        let input = json!({"limit": 10});
        assert_eq!(extract_optional_uint(&input, "limit"), Some(10));
    }

    #[test]
    fn test_extract_optional_uint_missing() {
        let input = json!({});
        assert_eq!(extract_optional_uint(&input, "limit"), None);
    }

    #[test]
    fn test_extract_optional_uint_null() {
        let input = json!({"limit": null});
        assert_eq!(extract_optional_uint(&input, "limit"), None);
    }

    #[test]
    fn test_extract_optional_uint_non_number() {
        let input = json!({"limit": "ten"});
        assert_eq!(extract_optional_uint(&input, "limit"), None);
    }

    #[test]
    fn test_extract_optional_uint_zero() {
        let input = json!({"limit": 0});
        assert_eq!(extract_optional_uint(&input, "limit"), Some(0));
    }

    #[test]
    fn test_extract_optional_uint_string_number() {
        let input = json!({"limit": "10"});
        assert_eq!(extract_optional_uint(&input, "limit"), Some(10));
    }

    #[test]
    fn test_extract_optional_uint_string_zero() {
        let input = json!({"limit": "0"});
        assert_eq!(extract_optional_uint(&input, "limit"), Some(0));
    }

    #[test]
    fn test_extract_optional_uint_string_with_whitespace() {
        let input = json!({"limit": " 42 "});
        assert_eq!(extract_optional_uint(&input, "limit"), Some(42));
    }

    // =========================================================================
    // extract_optional_bool tests
    // =========================================================================

    #[test]
    fn test_extract_optional_bool_true() {
        let input = json!({"flag": true});
        assert_eq!(extract_optional_bool(&input, "flag"), Some(true));
    }

    #[test]
    fn test_extract_optional_bool_false() {
        let input = json!({"flag": false});
        assert_eq!(extract_optional_bool(&input, "flag"), Some(false));
    }

    #[test]
    fn test_extract_optional_bool_missing() {
        let input = json!({});
        assert_eq!(extract_optional_bool(&input, "flag"), None);
    }

    #[test]
    fn test_extract_optional_bool_null() {
        let input = json!({"flag": null});
        assert_eq!(extract_optional_bool(&input, "flag"), None);
    }

    #[test]
    fn test_extract_optional_bool_string_true() {
        let input = json!({"flag": "true"});
        assert_eq!(extract_optional_bool(&input, "flag"), Some(true));
    }

    #[test]
    fn test_extract_optional_bool_string_false() {
        let input = json!({"flag": "false"});
        assert_eq!(extract_optional_bool(&input, "flag"), Some(false));
    }

    #[test]
    fn test_extract_optional_bool_string_yes() {
        let input = json!({"flag": "yes"});
        assert_eq!(extract_optional_bool(&input, "flag"), Some(true));
    }

    #[test]
    fn test_extract_optional_bool_string_no() {
        let input = json!({"flag": "no"});
        assert_eq!(extract_optional_bool(&input, "flag"), Some(false));
    }

    #[test]
    fn test_extract_optional_bool_number_1() {
        let input = json!({"flag": 1});
        assert_eq!(extract_optional_bool(&input, "flag"), Some(true));
    }

    #[test]
    fn test_extract_optional_bool_number_0() {
        let input = json!({"flag": 0});
        assert_eq!(extract_optional_bool(&input, "flag"), Some(false));
    }

    #[test]
    fn test_extract_optional_bool_invalid_string() {
        let input = json!({"flag": "maybe"});
        assert_eq!(extract_optional_bool(&input, "flag"), None);
    }
}
