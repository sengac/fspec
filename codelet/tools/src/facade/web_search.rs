//! Web search tool facades for different providers.

use super::traits::{InternalWebSearchParams, ToolDefinition, ToolFacade};
use crate::ToolError;
use serde_json::{json, Value};

/// Default headless mode for browser operations
const DEFAULT_HEADLESS: bool = true;

/// Extract the headless parameter from JSON input with a default of true
fn extract_headless(input: &Value) -> bool {
    input
        .get("headless")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(DEFAULT_HEADLESS)
}

/// Extract a string field from JSON input, returning empty string if missing
fn extract_string(input: &Value, field: &str) -> String {
    input
        .get(field)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Extract an optional string field from JSON input
fn extract_optional_string(input: &Value, field: &str) -> Option<String> {
    input
        .get(field)
        .and_then(|v| v.as_str())
        .map(std::string::ToString::to_string)
}

/// Extract a boolean field from JSON input with a default value
fn extract_bool(input: &Value, field: &str, default: bool) -> bool {
    input
        .get(field)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default)
}

/// Validate and extract a URL from JSON input
/// Returns an error if the URL is missing, doesn't start with http(s)://, or is malformed
fn extract_validated_url(input: &Value, tool_name: &'static str) -> Result<String, ToolError> {
    let url = input
        .get("url")
        .and_then(|u| u.as_str())
        .ok_or_else(|| ToolError::Validation {
            tool: tool_name,
            message: "Missing 'url' field".to_string(),
        })?;

    // Validate URL format (must start with http:// or https://)
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(ToolError::Validation {
            tool: tool_name,
            message: "URL must start with http:// or https://".to_string(),
        });
    }

    // Validate it's a well-formed URL
    url::Url::parse(url).map_err(|e| ToolError::Validation {
        tool: tool_name,
        message: format!("Invalid URL: {e}"),
    })?;

    Ok(url.to_string())
}

/// Claude-specific web search facade.
///
/// Claude supports complex schemas with `oneOf` and nested action objects,
/// so we can use the full-featured schema.
pub struct ClaudeWebSearchFacade;

impl ToolFacade for ClaudeWebSearchFacade {
    fn provider(&self) -> &'static str {
        "claude"
    }

    fn tool_name(&self) -> &'static str {
        "WebSearch"
    }

    fn definition(&self) -> ToolDefinition {
        // Use flat schema to avoid Claude serializing nested objects as strings
        ToolDefinition {
            name: "WebSearch".to_string(),
            description: "Perform web search, open web pages, find content within pages, or capture screenshots. Supports both headless (no UI, default) and non-headless (visible browser) modes for debugging. Use action_type to specify the operation.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action_type": {
                        "type": "string",
                        "enum": ["search", "open_page", "find_in_page", "capture_screenshot"],
                        "description": "The type of web action to perform"
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query (required for 'search' action)"
                    },
                    "url": {
                        "type": "string",
                        "description": "URL to open, search within, or capture (required for 'open_page', 'find_in_page', and 'capture_screenshot' actions)"
                    },
                    "pattern": {
                        "type": "string",
                        "description": "Pattern to find in page (required for 'find_in_page' action)"
                    },
                    "output_path": {
                        "type": "string",
                        "description": "File path to save screenshot. If not provided, saves to temp directory (optional for 'capture_screenshot' action)"
                    },
                    "full_page": {
                        "type": "boolean",
                        "description": "If true, captures entire scrollable page. If false (default), captures visible viewport only (optional for 'capture_screenshot' action)"
                    },
                    "headless": {
                        "type": "boolean",
                        "description": "If true (default), runs Chrome in headless mode with no visible UI. If false, shows the browser window for debugging or visual confirmation (optional for all actions except 'search')"
                    }
                },
                "required": ["action_type"]
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalWebSearchParams, ToolError> {
        // Handle flat schema with action_type at top level
        let action_type = input
            .get("action_type")
            .and_then(|t| t.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "web_search",
                message: "Missing 'action_type' field".to_string(),
            })?;

        match action_type {
            "search" => Ok(InternalWebSearchParams::Search {
                query: extract_string(&input, "query"),
            }),
            "open_page" => Ok(InternalWebSearchParams::OpenPage {
                url: extract_string(&input, "url"),
                headless: extract_headless(&input),
                pause: false,
            }),
            "find_in_page" => Ok(InternalWebSearchParams::FindInPage {
                url: extract_string(&input, "url"),
                pattern: extract_string(&input, "pattern"),
                headless: extract_headless(&input),
                pause: false,
            }),
            "capture_screenshot" => Ok(InternalWebSearchParams::CaptureScreenshot {
                url: extract_string(&input, "url"),
                output_path: extract_optional_string(&input, "output_path"),
                full_page: extract_bool(&input, "full_page", false),
                headless: extract_headless(&input),
                pause: false,
            }),
            _ => Err(ToolError::Validation {
                tool: "web_search",
                message: format!("Unknown action type: {action_type}"),
            }),
        }
    }
}

/// Gemini-specific web search facade.
///
/// Gemini prefers flat, simple schemas without `oneOf`.
/// This facade presents a simple `{query}` schema.
pub struct GeminiGoogleWebSearchFacade;

impl ToolFacade for GeminiGoogleWebSearchFacade {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn tool_name(&self) -> &'static str {
        "google_web_search"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "google_web_search".to_string(),
            description: "Search the web using Google Search".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalWebSearchParams, ToolError> {
        let query = input
            .get("query")
            .and_then(|q| q.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "google_web_search",
                message: "Missing 'query' field".to_string(),
            })?
            .to_string();

        Ok(InternalWebSearchParams::Search { query })
    }
}

/// Gemini-specific web fetch facade.
///
/// Uses a clean schema with separate `url` and `format` parameters
/// (following OpenCode's approach rather than Gemini CLI's prompt-embedded URLs).
pub struct GeminiWebFetchFacade;

impl ToolFacade for GeminiWebFetchFacade {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn tool_name(&self) -> &'static str {
        "web_fetch"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_fetch".to_string(),
            description: "Fetch content from a URL. Use this after google_web_search to retrieve page content from search results. Supports both headless (no UI, default) and non-headless (visible browser) modes for debugging.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to fetch content from (must start with http:// or https://)"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["text", "markdown", "html"],
                        "description": "The format to return the content in (default: markdown)"
                    },
                    "headless": {
                        "type": "boolean",
                        "description": "If true (default), runs Chrome in headless mode with no visible UI. If false, shows the browser window for debugging or visual confirmation"
                    }
                },
                "required": ["url"]
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalWebSearchParams, ToolError> {
        let url = extract_validated_url(&input, "web_fetch")?;

        Ok(InternalWebSearchParams::OpenPage {
            url,
            headless: extract_headless(&input),
            pause: false,
        })
    }
}

/// Gemini-specific web page screenshot capture facade.
///
/// Provides a dedicated screenshot tool following Gemini's preference
/// for separate, focused tools rather than action-based dispatching.
pub struct GeminiWebScreenshotFacade;

impl ToolFacade for GeminiWebScreenshotFacade {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn tool_name(&self) -> &'static str {
        "capture_screenshot"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "capture_screenshot".to_string(),
            description: "Capture a screenshot of a web page. Returns the file path to the saved PNG image. Use the Read tool to view the screenshot. Supports both headless (no UI, default) and non-headless (visible browser) modes for debugging.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL of the web page to capture (must start with http:// or https://)"
                    },
                    "output_path": {
                        "type": "string",
                        "description": "File path to save the screenshot. If not provided, saves to temp directory"
                    },
                    "full_page": {
                        "type": "boolean",
                        "description": "If true, captures the entire scrollable page. If false (default), captures only the visible viewport"
                    },
                    "headless": {
                        "type": "boolean",
                        "description": "If true (default), runs Chrome in headless mode with no visible UI. If false, shows the browser window for debugging or visual confirmation"
                    }
                },
                "required": ["url"]
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalWebSearchParams, ToolError> {
        let url = extract_validated_url(&input, "capture_screenshot")?;

        Ok(InternalWebSearchParams::CaptureScreenshot {
            url,
            output_path: extract_optional_string(&input, "output_path"),
            full_page: extract_bool(&input, "full_page", false),
            headless: extract_headless(&input),
            pause: false,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_facade_maps_search_action() {
        let facade = ClaudeWebSearchFacade;
        // Uses flat schema with action_type at top level
        let input = json!({
            "action_type": "search",
            "query": "rust async"
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalWebSearchParams::Search {
                query: "rust async".to_string()
            }
        );
    }

    #[test]
    fn test_claude_facade_maps_open_page_action() {
        let facade = ClaudeWebSearchFacade;
        // Uses flat schema with action_type at top level
        let input = json!({
            "action_type": "open_page",
            "url": "https://example.com"
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalWebSearchParams::OpenPage {
                url: "https://example.com".to_string(),
                headless: true, pause: false,
            }
        );
    }

    #[test]
    fn test_gemini_web_search_facade_maps_query() {
        let facade = GeminiGoogleWebSearchFacade;
        let input = json!({
            "query": "rust async"
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalWebSearchParams::Search {
                query: "rust async".to_string()
            }
        );
    }

    #[test]
    fn test_gemini_web_fetch_facade_maps_url() {
        let facade = GeminiWebFetchFacade;
        let input = json!({
            "url": "https://example.com"
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalWebSearchParams::OpenPage {
                url: "https://example.com".to_string(),
                headless: true, pause: false,
            }
        );
    }

    #[test]
    fn test_gemini_web_fetch_facade_with_format() {
        let facade = GeminiWebFetchFacade;
        let input = json!({
            "url": "https://example.com",
            "format": "markdown"
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalWebSearchParams::OpenPage {
                url: "https://example.com".to_string(),
                headless: true, pause: false,
            }
        );
    }

    #[test]
    fn test_gemini_web_fetch_facade_rejects_invalid_url() {
        let facade = GeminiWebFetchFacade;
        let input = json!({
            "url": "not-a-url"
        });

        let result = facade.map_params(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_gemini_web_fetch_facade_rejects_missing_protocol() {
        let facade = GeminiWebFetchFacade;
        let input = json!({
            "url": "example.com"
        });

        let result = facade.map_params(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_gemini_web_fetch_facade_with_headless_false() {
        let facade = GeminiWebFetchFacade;
        let input = json!({
            "url": "https://example.com",
            "headless": false
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalWebSearchParams::OpenPage {
                url: "https://example.com".to_string(),
                headless: false, pause: false,
            }
        );
    }

    #[test]
    fn test_claude_facade_maps_capture_screenshot_action() {
        let facade = ClaudeWebSearchFacade;
        let input = json!({
            "action_type": "capture_screenshot",
            "url": "https://example.com",
            "output_path": "/tmp/screenshot.png",
            "full_page": true
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalWebSearchParams::CaptureScreenshot {
                url: "https://example.com".to_string(),
                output_path: Some("/tmp/screenshot.png".to_string()),
                full_page: true,
                headless: true, pause: false,
            }
        );
    }

    #[test]
    fn test_claude_facade_maps_capture_screenshot_defaults() {
        let facade = ClaudeWebSearchFacade;
        let input = json!({
            "action_type": "capture_screenshot",
            "url": "https://example.com"
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalWebSearchParams::CaptureScreenshot {
                url: "https://example.com".to_string(),
                output_path: None,
                full_page: false,
                headless: true, pause: false,
            }
        );
    }

    #[test]
    fn test_claude_facade_maps_capture_screenshot_visible_mode() {
        let facade = ClaudeWebSearchFacade;
        let input = json!({
            "action_type": "capture_screenshot",
            "url": "https://example.com",
            "headless": false
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalWebSearchParams::CaptureScreenshot {
                url: "https://example.com".to_string(),
                output_path: None,
                full_page: false,
                headless: false, pause: false,
            }
        );
    }

    #[test]
    fn test_claude_facade_maps_find_in_page_with_headless() {
        let facade = ClaudeWebSearchFacade;
        let input = json!({
            "action_type": "find_in_page",
            "url": "https://example.com",
            "pattern": "search term",
            "headless": false
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalWebSearchParams::FindInPage {
                url: "https://example.com".to_string(),
                pattern: "search term".to_string(),
                headless: false, pause: false,
            }
        );
    }

    #[test]
    fn test_gemini_web_fetch_facade_has_url_and_format_schema() {
        let facade = GeminiWebFetchFacade;
        let def = facade.definition();

        // Should have url and format properties
        assert!(def.parameters["properties"]["url"].is_object());
        assert!(def.parameters["properties"]["format"].is_object());
        assert!(def.parameters["properties"]["format"]["enum"].is_array());

        // url should be required
        let required = def.parameters["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "url"));

        // Test non-headless mode
        let input_visible = json!({
            "url": "https://example.com",
            "headless": false
        });
        let result = facade.map_params(input_visible).unwrap();
        assert_eq!(
            result,
            InternalWebSearchParams::OpenPage {
                url: "https://example.com".to_string(),
                headless: false, pause: false,
            }
        );
    }

    #[test]
    fn test_gemini_facade_has_flat_schema() {
        let facade = GeminiGoogleWebSearchFacade;
        let def = facade.definition();

        // Should have flat query property, not nested action with oneOf
        assert!(def.parameters.get("properties").is_some());
        assert!(def.parameters["properties"].get("query").is_some());
        assert!(def.parameters.get("oneOf").is_none());
        assert!(def.parameters["properties"].get("action").is_none());
    }

    #[test]
    fn test_claude_facade_has_flat_schema_with_enum() {
        let facade = ClaudeWebSearchFacade;
        let def = facade.definition();

        // Should have flat schema with action_type enum (not nested oneOf)
        assert!(def.parameters["properties"]["action_type"].is_object());
        assert!(def.parameters["properties"]["action_type"]["enum"].is_array());

        let action_types = def.parameters["properties"]["action_type"]["enum"]
            .as_array()
            .unwrap();
        let types: Vec<&str> = action_types.iter().filter_map(|v| v.as_str()).collect();

        assert!(types.contains(&"search"));
        assert!(types.contains(&"open_page"));
        assert!(types.contains(&"find_in_page"));
        assert!(types.contains(&"capture_screenshot"));

        // Should have query, url, and pattern as top-level properties
        assert!(def.parameters["properties"]["query"].is_object());
        assert!(def.parameters["properties"]["url"].is_object());
        assert!(def.parameters["properties"]["pattern"].is_object());
        assert!(def.parameters["properties"]["output_path"].is_object());
        assert!(def.parameters["properties"]["full_page"].is_object());
        assert!(def.parameters["properties"]["headless"].is_object());

        // Verify headless description mentions default behavior
        let headless_desc = def.parameters["properties"]["headless"]["description"]
            .as_str()
            .unwrap_or("");
        assert!(headless_desc.contains("default"));

        // Test headless parameter mapping for open_page
        let input_non_headless = json!({
            "action_type": "open_page",
            "url": "https://example.com",
            "headless": false
        });
        let result = ClaudeWebSearchFacade.map_params(input_non_headless).unwrap();
        assert_eq!(
            result,
            InternalWebSearchParams::OpenPage {
                url: "https://example.com".to_string(),
                headless: false, pause: false,
            }
        );
    }

    #[test]
    fn test_gemini_web_screenshot_facade_maps_params() {
        let facade = GeminiWebScreenshotFacade;
        let input = json!({
            "url": "https://example.com",
            "output_path": "/tmp/screenshot.png",
            "full_page": true
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalWebSearchParams::CaptureScreenshot {
                url: "https://example.com".to_string(),
                output_path: Some("/tmp/screenshot.png".to_string()),
                full_page: true,
                headless: true, pause: false,
            }
        );
    }

    #[test]
    fn test_gemini_web_screenshot_facade_maps_minimal_params() {
        let facade = GeminiWebScreenshotFacade;
        let input = json!({
            "url": "https://example.com"
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalWebSearchParams::CaptureScreenshot {
                url: "https://example.com".to_string(),
                output_path: None,
                full_page: false,
                headless: true,
                pause: false,
            }
        );
    }

    #[test]
    fn test_gemini_web_screenshot_facade_rejects_invalid_url() {
        let facade = GeminiWebScreenshotFacade;
        let input = json!({
            "url": "not-a-url"
        });

        let result = facade.map_params(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_gemini_web_screenshot_facade_rejects_missing_protocol() {
        let facade = GeminiWebScreenshotFacade;
        let input = json!({
            "url": "example.com"
        });

        let result = facade.map_params(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_gemini_web_screenshot_facade_has_correct_schema() {
        let facade = GeminiWebScreenshotFacade;
        let def = facade.definition();

        assert_eq!(def.name, "capture_screenshot");
        assert!(def.description.contains("screenshot"));

        // Should have url, output_path, and full_page properties
        assert!(def.parameters["properties"]["url"].is_object());
        assert!(def.parameters["properties"]["output_path"].is_object());
        assert!(def.parameters["properties"]["full_page"].is_object());
        assert!(def.parameters["properties"]["headless"].is_object());

        // url should be required
        let required = def.parameters["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "url"));

        // Test non-headless mode
        let input_visible = json!({
            "url": "https://example.com",
            "headless": false
        });
        let result = facade.map_params(input_visible).unwrap();
        assert_eq!(
            result,
            InternalWebSearchParams::CaptureScreenshot {
                url: "https://example.com".to_string(),
                output_path: None,
                full_page: false,
                headless: false,
                pause: false,
            }
        );
    }
}
