//! Shared LLM response parsing utilities.
//!
//! Extracts JSON from LLM responses that may be wrapped in markdown code blocks.
//! Used by `learnings_extraction.rs` for the Learnings extraction pipeline.

/// Extract JSON from an LLM response that may be wrapped in markdown code blocks.
///
/// Handles three formats:
/// 1. `\`\`\`json ... \`\`\`` blocks — extracts the inner JSON
/// 2. `\`\`\` ... \`\`\`` blocks — extracts inner content if it starts with `{`
/// 3. Plain text — returns as-is
pub fn extract_json_from_response(response: &str) -> &str {
    let trimmed = response.trim();

    // Try to find ```json ... ``` block
    if let Some(start) = trimmed.find("```json") {
        let json_start = start + 7; // skip "```json"
        if let Some(end) = trimmed[json_start..].find("```") {
            return trimmed[json_start..json_start + end].trim();
        }
    }

    // Try to find ``` ... ``` block (without language)
    if let Some(start) = trimmed.find("```") {
        let code_start = start + 3;
        if let Some(end) = trimmed[code_start..].find("```") {
            let inner = trimmed[code_start..code_start + end].trim();
            if inner.starts_with('{') {
                return inner;
            }
        }
    }

    // Return as-is if no code block found
    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_block() {
        let input = r#"```json
{"key": "value"}
```"#;
        assert_eq!(extract_json_from_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_extract_plain_json() {
        let input = r#"{"key": "value"}"#;
        assert_eq!(extract_json_from_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_extract_code_block_without_lang() {
        let input = r#"```
{"key": "value"}
```"#;
        assert_eq!(extract_json_from_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_extract_non_json_code_block() {
        let input = r#"```
not json content
```"#;
        // Should return the full trimmed input since inner doesn't start with '{'
        assert_eq!(extract_json_from_response(input), input.trim());
    }
}
