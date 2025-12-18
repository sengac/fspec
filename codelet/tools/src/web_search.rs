// Web Search Tool Implementation
// Implements web search capabilities with Search, OpenPage, and FindInPage actions
// Uses Chrome DevTools Protocol via rust-headless-chrome for full JavaScript support

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::{Arc, Mutex};

use crate::chrome_browser::{ChromeBrowser, ChromeConfig, ChromeError};
use crate::page_fetcher::PageFetcher;
use crate::search_engine::SearchEngine;
use crate::ToolError;
use codelet_common::web_search::WebSearchAction;

/// Global browser instance - lazily initialized on first use, can be recreated on error
static BROWSER: Mutex<Option<Arc<ChromeBrowser>>> = Mutex::new(None);

/// Create a new browser instance
fn create_browser() -> Result<Arc<ChromeBrowser>, ChromeError> {
    let config = ChromeConfig::default();
    let browser = ChromeBrowser::new(config)?;
    Ok(Arc::new(browser))
}

/// Get or initialize the global browser instance
/// If the browser connection has closed, creates a new one
fn get_browser() -> Result<Arc<ChromeBrowser>, ChromeError> {
    let mut guard = BROWSER
        .lock()
        .map_err(|e| ChromeError::LaunchError(format!("Failed to acquire browser lock: {e}")))?;

    // If we have a browser, return it
    if let Some(ref browser) = *guard {
        return Ok(Arc::clone(browser));
    }

    // Create a new browser
    let browser = create_browser()?;
    *guard = Some(Arc::clone(&browser));
    Ok(browser)
}

/// Clear the cached browser instance (called on connection errors)
fn clear_browser() {
    if let Ok(mut guard) = BROWSER.lock() {
        *guard = None;
    }
}

/// Execute a browser operation with automatic retry on connection failure
fn with_browser_retry<T, F>(operation: F) -> Result<T, ChromeError>
where
    F: Fn(Arc<ChromeBrowser>) -> Result<T, ChromeError>,
{
    // First attempt
    let browser = get_browser()?;
    match operation(Arc::clone(&browser)) {
        Ok(result) => Ok(result),
        Err(ChromeError::TabError(msg)) if msg.contains("connection is closed") => {
            // Browser connection died, clear cache and retry once
            clear_browser();
            let new_browser = get_browser()?;
            operation(new_browser)
        }
        Err(e) => Err(e),
    }
}

/// Web Search Tool
///
/// This tool implements web search and web content access capabilities
/// with three main actions: Search, OpenPage, and FindInPage.
///
/// Uses Chrome DevTools Protocol for full JavaScript rendering support,
/// enabling access to SPAs and dynamically-generated content.
#[derive(Clone, Debug)]
pub struct WebSearchTool;

/// Web search request arguments
#[derive(Debug, Deserialize, Serialize)]
pub struct WebSearchRequest {
    #[serde(deserialize_with = "deserialize_web_search_action")]
    pub action: WebSearchAction,
}

/// Custom deserializer that handles both JSON object and JSON string formats
///
/// This handles the case where Claude passes the action as a JSON string instead of an object
fn deserialize_web_search_action<'de, D>(deserializer: D) -> Result<WebSearchAction, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    use serde_json::Value;

    let value = Value::deserialize(deserializer)?;

    match value {
        // If it's already an object, deserialize directly
        Value::Object(_) => WebSearchAction::deserialize(value).map_err(D::Error::custom),
        // If it's a string, parse it as JSON first
        Value::String(s) => {
            let parsed: Value = serde_json::from_str(&s).map_err(D::Error::custom)?;
            WebSearchAction::deserialize(parsed).map_err(D::Error::custom)
        }
        _ => Err(D::Error::custom("Expected object or string for action")),
    }
}

/// Result from web search operations
#[derive(Debug, Serialize)]
pub struct WebSearchResult {
    pub success: bool,
    pub message: String,
    pub action: WebSearchAction,
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self
    }
}

impl WebSearchTool {
    /// Create a new WebSearchTool instance
    pub fn new() -> Self {
        Self
    }
}

impl Tool for WebSearchTool {
    const NAME: &'static str = "web_search";

    type Error = ToolError;
    type Args = WebSearchRequest;
    type Output = WebSearchResult;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        // This matches the schema structure that was working in the previous implementation
        ToolDefinition {
            name: "web_search".to_string(),
            description: "Perform web search, open web pages, or find content within pages using Chrome-based web scraping with full JavaScript support".to_string(),
            parameters: json!({
                "additionalProperties": false,
                "properties": {
                    "action": {
                        "oneOf": [
                            {
                                "additionalProperties": false,
                                "properties": {
                                    "query": {
                                        "description": "Search query",
                                        "type": "string"
                                    },
                                    "type": {
                                        "const": "search",
                                        "type": "string"
                                    }
                                },
                                "required": ["type"],
                                "type": "object"
                            },
                            {
                                "additionalProperties": false,
                                "properties": {
                                    "type": {
                                        "const": "open_page",
                                        "type": "string"
                                    },
                                    "url": {
                                        "description": "URL to open",
                                        "type": "string"
                                    }
                                },
                                "required": ["type"],
                                "type": "object"
                            },
                            {
                                "additionalProperties": false,
                                "properties": {
                                    "pattern": {
                                        "description": "Pattern to find",
                                        "type": "string"
                                    },
                                    "type": {
                                        "const": "find_in_page",
                                        "type": "string"
                                    },
                                    "url": {
                                        "description": "URL of page to search",
                                        "type": "string"
                                    }
                                },
                                "required": ["type"],
                                "type": "object"
                            }
                        ],
                        "type": "object"
                    }
                },
                "required": ["action"],
                "type": "object"
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Implement web search functionality using Chrome
        let (success, message) = match &args.action {
            WebSearchAction::Search { query } => {
                let query = query.as_deref().unwrap_or("");
                if query.is_empty() {
                    (false, "No search query provided".to_string())
                } else {
                    match perform_web_search(query) {
                        Ok(results) => (true, format!("Search results for '{query}':\n{results}")),
                        Err(e) => (false, format!("Search failed: {e}")),
                    }
                }
            }
            WebSearchAction::OpenPage { url } => {
                let url = url.as_deref().unwrap_or("");
                if url.is_empty() {
                    (false, "No URL provided".to_string())
                } else {
                    match fetch_page_content(url) {
                        Ok(content) => (true, format!("Page content from {url}:\n{content}")),
                        Err(e) => (false, format!("Failed to fetch page: {e}")),
                    }
                }
            }
            WebSearchAction::FindInPage { url, pattern } => {
                let url = url.as_deref().unwrap_or("");
                let pattern = pattern.as_deref().unwrap_or("");
                if url.is_empty() || pattern.is_empty() {
                    (false, "URL or pattern not provided".to_string())
                } else {
                    match find_pattern_in_page(url, pattern) {
                        Ok(found) => (
                            true,
                            format!("Pattern '{pattern}' search results in {url}:\n{found}"),
                        ),
                        Err(e) => (false, format!("Pattern search failed: {e}")),
                    }
                }
            }
            WebSearchAction::Other => (false, "Unknown web search action".to_string()),
        };

        Ok(WebSearchResult {
            success,
            message,
            action: args.action,
        })
    }
}

/// Perform a web search using DuckDuckGo via Chrome
fn perform_web_search(query: &str) -> Result<String, ChromeError> {
    let query = query.to_string();
    with_browser_retry(|browser| {
        let search_engine = SearchEngine::new(browser);
        let results = search_engine.search(&query)?;

        if results.is_empty() {
            return Ok("No search results found".to_string());
        }

        let mut output = Vec::new();
        for (i, result) in results.iter().enumerate() {
            output.push(format!(
                "{}. {}\n   URL: {}\n   {}",
                i + 1,
                result.title,
                result.url,
                result.snippet
            ));
        }

        Ok(output.join("\n\n"))
    })
}

/// Fetch content from a web page using Chrome
fn fetch_page_content(url: &str) -> Result<String, ChromeError> {
    let url = url.to_string();
    with_browser_retry(|browser| {
        let page_fetcher = PageFetcher::new(browser);
        let content = page_fetcher.fetch(&url)?;

        let mut output = Vec::new();

        if let Some(title) = &content.title {
            output.push(format!("# {title}"));
        }

        if let Some(desc) = &content.meta_description {
            output.push(format!("*{desc}*"));
        }

        if !content.main_content.is_empty() {
            output.push(String::new());
            output.push(content.main_content.clone());
        }

        if !content.links.is_empty() {
            output.push(String::new());
            output.push("## Links found:".to_string());
            for link in content.links.iter().take(10) {
                if !link.text.is_empty() {
                    output.push(format!("- [{}]({})", link.text, link.href));
                }
            }
        }

        Ok(output.join("\n"))
    })
}

/// Find a pattern in a web page using Chrome
fn find_pattern_in_page(url: &str, pattern: &str) -> Result<String, ChromeError> {
    let url = url.to_string();
    let pattern = pattern.to_string();
    with_browser_retry(|browser| {
        let page_fetcher = PageFetcher::new(browser);
        let matches = page_fetcher.find_in_page(&url, &pattern)?;

        if matches.is_empty() {
            return Ok(format!("Pattern '{pattern}' not found on page"));
        }

        let mut output = Vec::new();
        output.push(format!("Found {} matches:", matches.len()));

        for (i, context) in matches.iter().enumerate() {
            output.push(format!("{}. ...{}...", i + 1, context));
        }

        Ok(output.join("\n"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_web_search_action_from_object() {
        let json = r#"{"action": {"type": "search", "query": "test"}}"#;
        let request: WebSearchRequest = serde_json::from_str(json).expect("Should parse object");
        match request.action {
            WebSearchAction::Search { query } => {
                assert_eq!(query, Some("test".to_string()));
            }
            _ => panic!("Expected Search action"),
        }
    }

    #[test]
    fn test_deserialize_web_search_action_from_string() {
        let json = r#"{"action": "{\"type\": \"search\", \"query\": \"test\"}"}"#;
        let request: WebSearchRequest = serde_json::from_str(json).expect("Should parse string");
        match request.action {
            WebSearchAction::Search { query } => {
                assert_eq!(query, Some("test".to_string()));
            }
            _ => panic!("Expected Search action"),
        }
    }
}
