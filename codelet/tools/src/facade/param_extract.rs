//! Shared parameter extraction helpers for tool facades.
//!
//! These functions provide consistent extraction of required/optional fields
//! from JSON input across all provider-specific facades (Codex, Z.AI, etc.).

use crate::ToolError;
use serde_json::Value;

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
/// Returns None if the field is missing, null, or not a valid unsigned integer.
pub fn extract_optional_uint(input: &Value, field: &str) -> Option<usize> {
    input.get(field).and_then(Value::as_u64).map(|n| n as usize)
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
}
