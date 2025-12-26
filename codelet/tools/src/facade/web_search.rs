//! Web search tool facades for different providers.

use super::traits::{InternalWebSearchParams, ToolDefinition, ToolFacade};
use crate::ToolError;
use serde_json::{json, Value};

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
        "web_search"
    }

    fn definition(&self) -> ToolDefinition {
        // Use flat schema to avoid Claude serializing nested objects as strings
        ToolDefinition {
            name: "web_search".to_string(),
            description: "Perform web search, open web pages, or find content within pages. Use action_type to specify the operation.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action_type": {
                        "type": "string",
                        "enum": ["search", "open_page", "find_in_page"],
                        "description": "The type of web action to perform"
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query (required for 'search' action)"
                    },
                    "url": {
                        "type": "string",
                        "description": "URL to open or search within (required for 'open_page' and 'find_in_page' actions)"
                    },
                    "pattern": {
                        "type": "string",
                        "description": "Pattern to find in page (required for 'find_in_page' action)"
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
            "search" => {
                let query = input
                    .get("query")
                    .and_then(|q| q.as_str())
                    .unwrap_or("")
                    .to_string();
                Ok(InternalWebSearchParams::Search { query })
            }
            "open_page" => {
                let url = input
                    .get("url")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();
                Ok(InternalWebSearchParams::OpenPage { url })
            }
            "find_in_page" => {
                let url = input
                    .get("url")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();
                let pattern = input
                    .get("pattern")
                    .and_then(|p| p.as_str())
                    .unwrap_or("")
                    .to_string();
                Ok(InternalWebSearchParams::FindInPage { url, pattern })
            }
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
/// Gemini CLI uses a separate `web_fetch` tool for fetching URL content.
/// The prompt parameter contains the URL and instructions.
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
            description: "Fetch and process content from URLs".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "prompt": {
                        "type": "string",
                        "description": "URL(s) and instructions for processing"
                    }
                },
                "required": ["prompt"]
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalWebSearchParams, ToolError> {
        let prompt = input
            .get("prompt")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "web_fetch",
                message: "Missing 'prompt' field".to_string(),
            })?;

        // Extract URL from prompt - look for http:// or https://
        let url = extract_url_from_prompt(prompt).ok_or_else(|| ToolError::Validation {
            tool: "web_fetch",
            message: "No valid URL found in prompt".to_string(),
        })?;

        Ok(InternalWebSearchParams::OpenPage { url })
    }
}

/// Extracts the first URL from a prompt string.
fn extract_url_from_prompt(prompt: &str) -> Option<String> {
    // Split on whitespace and find the first token that looks like a URL
    for token in prompt.split_whitespace() {
        if token.starts_with("http://") || token.starts_with("https://") {
            // Validate it's a proper URL
            if url::Url::parse(token).is_ok() {
                return Some(token.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
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
                url: "https://example.com".to_string()
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
    fn test_gemini_web_fetch_facade_extracts_url() {
        let facade = GeminiWebFetchFacade;
        let input = json!({
            "prompt": "https://example.com summarize this page"
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalWebSearchParams::OpenPage {
                url: "https://example.com".to_string()
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
        let types: Vec<&str> = action_types
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        assert!(types.contains(&"search"));
        assert!(types.contains(&"open_page"));
        assert!(types.contains(&"find_in_page"));

        // Should have query, url, and pattern as top-level properties
        assert!(def.parameters["properties"]["query"].is_object());
        assert!(def.parameters["properties"]["url"].is_object());
        assert!(def.parameters["properties"]["pattern"].is_object());
    }

    #[test]
    fn test_extract_url_from_prompt() {
        assert_eq!(
            extract_url_from_prompt("https://example.com summarize this"),
            Some("https://example.com".to_string())
        );

        assert_eq!(
            extract_url_from_prompt("Please fetch http://test.org and analyze"),
            Some("http://test.org".to_string())
        );

        assert_eq!(extract_url_from_prompt("no url here"), None);
    }
}
