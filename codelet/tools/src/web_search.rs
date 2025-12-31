// Web Search Tool Implementation
// Implements web search capabilities with Search, OpenPage, and FindInPage actions
// Uses Chrome DevTools Protocol via rust-headless-chrome for full JavaScript support

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::{Arc, Mutex};

use crate::chrome_browser::{ChromeBrowser, ChromeConfig, ChromeError};
use crate::limits::OutputLimits;
use crate::page_fetcher::PageFetcher;
use crate::search_engine::SearchEngine;
use crate::truncation::{format_truncation_warning, process_output_lines, truncate_output};
use crate::ToolError;
use codelet_common::web_search::WebSearchAction;

/// Convert ChromeError to ToolError for unified error handling at the API boundary
impl From<ChromeError> for ToolError {
    fn from(err: ChromeError) -> Self {
        match err {
            ChromeError::Timeout => ToolError::Timeout {
                tool: "web_search",
                seconds: 30, // Default Chrome timeout
            },
            ChromeError::LaunchError(msg) => ToolError::Execution {
                tool: "web_search",
                message: format!("Chrome launch failed: {msg}"),
            },
            ChromeError::ConnectionError(msg) => ToolError::Execution {
                tool: "web_search",
                message: format!("Chrome connection failed: {msg}"),
            },
            ChromeError::TabError(msg) => ToolError::Execution {
                tool: "web_search",
                message: format!("Chrome tab error: {msg}"),
            },
            ChromeError::NavigationError(msg) => ToolError::Execution {
                tool: "web_search",
                message: format!("Navigation failed: {msg}"),
            },
            ChromeError::EvaluationError(msg) => ToolError::Execution {
                tool: "web_search",
                message: format!("JavaScript evaluation failed: {msg}"),
            },
            ChromeError::ChromeNotFound(path) => ToolError::NotFound {
                tool: "web_search",
                message: format!("Chrome not found at: {path}"),
            },
            ChromeError::ScreenshotError(msg) => ToolError::Execution {
                tool: "web_search",
                message: format!("Screenshot capture failed: {msg}"),
            },
            ChromeError::IoError(msg) => ToolError::Execution {
                tool: "web_search",
                message: format!("File I/O error: {msg}"),
            },
        }
    }
}

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

/// Shutdown the browser and release all resources
///
/// This function should be called before process exit to ensure Chrome
/// is properly terminated. Rust does NOT drop static variables on exit,
/// so this must be called explicitly.
///
/// Safe to call multiple times or when no browser is running.
pub fn shutdown_browser() {
    if let Ok(mut guard) = BROWSER.lock() {
        if let Some(browser) = guard.take() {
            // Drop the Arc - if this is the last reference, Chrome will be killed
            // via Browser -> BrowserInner -> Process -> TemporaryProcess -> kill()
            drop(browser);
            tracing::info!("Browser shutdown complete");
        }
    }
}

/// Install a signal handler that shuts down the browser on SIGINT/SIGTERM
///
/// This ensures Chrome is properly terminated when:
/// - User presses Ctrl+C (SIGINT)
/// - Process receives SIGTERM
///
/// Call this once at program startup. The handler will call `shutdown_browser()`
/// and then exit the process.
///
/// # Example
/// ```ignore
/// codelet_tools::install_browser_cleanup_handler();
/// ```
pub fn install_browser_cleanup_handler() {
    if let Err(e) = ctrlc::set_handler(move || {
        tracing::info!("Received termination signal, shutting down browser...");
        shutdown_browser();
        // Exit after cleanup - use 130 for SIGINT (128 + 2)
        std::process::exit(130);
    }) {
        tracing::warn!("Failed to install browser cleanup handler: {e}");
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

/// Normalize action type strings across different LLM providers
///
/// Different providers may send action types in various formats:
/// - Claude: "search", "open_page", "find_in_page" (correct)
/// - Gemini: "web_search", "openPage", etc. (variations)
///
/// This function maps all known variations to the canonical snake_case format.
fn normalize_action_type(action_type: &str) -> &'static str {
    match action_type.to_lowercase().as_str() {
        // Search action variations
        // - "query" is what Gemini sends (uses the parameter name as the type)
        "search" | "web_search" | "websearch" | "query" => "search",
        // Open page variations
        "open_page" | "openpage" | "open" | "navigate" | "goto" | "url" => "open_page",
        // Find in page variations
        "find_in_page" | "findinpage" | "find" | "search_in_page" | "searchinpage" | "pattern" => {
            "find_in_page"
        }
        // Screenshot variations
        "capture_screenshot" | "capturescreenshot" | "screenshot" | "take_screenshot"
        | "takescreenshot" => "capture_screenshot",
        // Unknown - return as-is to let serde handle the error
        _ => "unknown",
    }
}

/// Custom deserializer that handles both JSON object and JSON string formats
/// and normalizes action types across different LLM providers
///
/// This handles:
/// 1. Action as JSON object vs JSON string (Claude sometimes stringifies)
/// 2. Different action type naming conventions across providers
fn deserialize_web_search_action<'de, D>(deserializer: D) -> Result<WebSearchAction, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    use serde_json::Value;

    let value = Value::deserialize(deserializer)?;

    // Parse the value into a mutable JSON object
    let mut obj = match value {
        Value::Object(obj) => obj,
        Value::String(s) => {
            let parsed: Value = serde_json::from_str(&s).map_err(D::Error::custom)?;
            match parsed {
                Value::Object(obj) => obj,
                _ => return Err(D::Error::custom("Expected object for action")),
            }
        }
        _ => return Err(D::Error::custom("Expected object or string for action")),
    };

    // Normalize the "type" field if present
    if let Some(Value::String(action_type)) = obj.get("type") {
        let normalized = normalize_action_type(action_type);
        obj.insert("type".to_string(), Value::String(normalized.to_string()));
    }

    // Deserialize the normalized object
    WebSearchAction::deserialize(Value::Object(obj)).map_err(D::Error::custom)
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
            description: "Perform web search, open web pages, find content within pages, or capture screenshots using Chrome-based web scraping with full JavaScript support".to_string(),
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
                            },
                            {
                                "additionalProperties": false,
                                "properties": {
                                    "type": {
                                        "const": "capture_screenshot",
                                        "type": "string"
                                    },
                                    "url": {
                                        "description": "URL of page to capture",
                                        "type": "string"
                                    },
                                    "output_path": {
                                        "description": "Optional file path for screenshot. If not provided, saves to temp directory",
                                        "type": "string"
                                    },
                                    "full_page": {
                                        "description": "If true, captures entire scrollable page. If false (default), captures visible viewport only",
                                        "type": "boolean"
                                    }
                                },
                                "required": ["type", "url"],
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
        // Return ToolError for validation failures, ChromeError converts via From impl
        let (success, message) = match &args.action {
            WebSearchAction::Search { query } => {
                let query = query.as_deref().unwrap_or("");
                if query.is_empty() {
                    return Err(ToolError::Validation {
                        tool: "web_search",
                        message: "Search query is required".to_string(),
                    });
                }
                match perform_web_search(query) {
                    Ok(results) => (true, format!("Search results for '{query}':\n{results}")),
                    Err(e) => return Err(e.into()),
                }
            }
            WebSearchAction::OpenPage { url } => {
                let url = url.as_deref().unwrap_or("");
                if url.is_empty() {
                    return Err(ToolError::Validation {
                        tool: "web_search",
                        message: "URL is required".to_string(),
                    });
                }
                match fetch_page_content(url) {
                    Ok(content) => (true, format!("Page content from {url}:\n{content}")),
                    Err(e) => return Err(e.into()),
                }
            }
            WebSearchAction::FindInPage { url, pattern } => {
                let url = url.as_deref().unwrap_or("");
                let pattern = pattern.as_deref().unwrap_or("");
                if url.is_empty() {
                    return Err(ToolError::Validation {
                        tool: "web_search",
                        message: "URL is required for find_in_page".to_string(),
                    });
                }
                if pattern.is_empty() {
                    return Err(ToolError::Validation {
                        tool: "web_search",
                        message: "Pattern is required for find_in_page".to_string(),
                    });
                }
                match find_pattern_in_page(url, pattern) {
                    Ok(found) => (
                        true,
                        format!("Pattern '{pattern}' search results in {url}:\n{found}"),
                    ),
                    Err(e) => return Err(e.into()),
                }
            }
            WebSearchAction::CaptureScreenshot {
                url,
                output_path,
                full_page,
            } => {
                let url = url.as_deref().unwrap_or("");
                if url.is_empty() {
                    return Err(ToolError::Validation {
                        tool: "web_search",
                        message: "URL is required for capture_screenshot".to_string(),
                    });
                }
                let full_page = full_page.unwrap_or(false);
                match capture_page_screenshot(url, output_path.clone(), full_page) {
                    Ok(file_path) => (true, format!("Screenshot saved to: {file_path}")),
                    Err(e) => return Err(e.into()),
                }
            }
            WebSearchAction::Other => {
                return Err(ToolError::Validation {
                    tool: "web_search",
                    message: "Unknown web search action type".to_string(),
                });
            }
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

        let raw_output = output.join("\n\n");

        // Apply truncation to prevent oversized output
        let lines = process_output_lines(&raw_output);
        let truncate_result = truncate_output(&lines, OutputLimits::MAX_OUTPUT_CHARS);
        let mut final_output = truncate_result.output;

        if truncate_result.char_truncated || truncate_result.remaining_count > 0 {
            let warning = format_truncation_warning(
                truncate_result.remaining_count,
                "lines",
                truncate_result.char_truncated,
                OutputLimits::MAX_OUTPUT_CHARS,
            );
            final_output.push_str(&warning);
        }

        Ok(final_output)
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

        let raw_output = output.join("\n");

        // Apply truncation to prevent oversized output (page content can be large)
        let lines = process_output_lines(&raw_output);
        let truncate_result = truncate_output(&lines, OutputLimits::MAX_OUTPUT_CHARS);
        let mut final_output = truncate_result.output;

        if truncate_result.char_truncated || truncate_result.remaining_count > 0 {
            let warning = format_truncation_warning(
                truncate_result.remaining_count,
                "lines",
                truncate_result.char_truncated,
                OutputLimits::MAX_OUTPUT_CHARS,
            );
            final_output.push_str(&warning);
        }

        Ok(final_output)
    })
}

/// Capture a screenshot of a web page using Chrome
fn capture_page_screenshot(
    url: &str,
    output_path: Option<String>,
    full_page: bool,
) -> Result<String, ChromeError> {
    let url = url.to_string();
    with_browser_retry(|browser| {
        let tab = browser.new_tab()?;
        browser.navigate_and_wait(&tab, &url)?;
        let file_path = browser.capture_screenshot(&tab, output_path.clone(), full_page)?;
        browser.cleanup_tab(&tab);
        Ok(file_path)
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

        let raw_output = output.join("\n");

        // Apply truncation to prevent oversized output
        let lines = process_output_lines(&raw_output);
        let truncate_result = truncate_output(&lines, OutputLimits::MAX_OUTPUT_CHARS);
        let mut final_output = truncate_result.output;

        if truncate_result.char_truncated || truncate_result.remaining_count > 0 {
            let warning = format_truncation_warning(
                truncate_result.remaining_count,
                "lines",
                truncate_result.char_truncated,
                OutputLimits::MAX_OUTPUT_CHARS,
            );
            final_output.push_str(&warning);
        }

        Ok(final_output)
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

    #[test]
    fn test_action_type_normalization_gemini_web_search() {
        // Gemini sends "web_search" instead of "search"
        let json = r#"{"action": {"type": "web_search", "query": "latest news"}}"#;
        let request: WebSearchRequest =
            serde_json::from_str(json).expect("Should parse Gemini format");
        match request.action {
            WebSearchAction::Search { query } => {
                assert_eq!(query, Some("latest news".to_string()));
            }
            _ => panic!("Expected Search action from Gemini's web_search type"),
        }
    }

    #[test]
    fn test_action_type_normalization_variations() {
        // Test various action type formats that different providers might send
        let test_cases = vec![
            // Search variations
            (r#"{"action": {"type": "search", "query": "q"}}"#, "search"),
            (
                r#"{"action": {"type": "web_search", "query": "q"}}"#,
                "search",
            ),
            (r#"{"action": {"type": "SEARCH", "query": "q"}}"#, "search"),
            (
                r#"{"action": {"type": "WebSearch", "query": "q"}}"#,
                "search",
            ),
            (r#"{"action": {"type": "query", "query": "q"}}"#, "search"), // Gemini uses parameter name as type
            // Open page variations
            (
                r#"{"action": {"type": "open_page", "url": "http://x"}}"#,
                "open_page",
            ),
            (
                r#"{"action": {"type": "openPage", "url": "http://x"}}"#,
                "open_page",
            ),
            (
                r#"{"action": {"type": "open", "url": "http://x"}}"#,
                "open_page",
            ),
            (
                r#"{"action": {"type": "navigate", "url": "http://x"}}"#,
                "open_page",
            ),
            // Find in page variations
            (
                r#"{"action": {"type": "find_in_page", "url": "http://x", "pattern": "p"}}"#,
                "find_in_page",
            ),
            (
                r#"{"action": {"type": "findInPage", "url": "http://x", "pattern": "p"}}"#,
                "find_in_page",
            ),
            (
                r#"{"action": {"type": "find", "url": "http://x", "pattern": "p"}}"#,
                "find_in_page",
            ),
        ];

        for (json, expected_type) in test_cases {
            let request: Result<WebSearchRequest, _> = serde_json::from_str(json);
            assert!(
                request.is_ok(),
                "Failed to parse: {} (expected {})",
                json,
                expected_type
            );
            let action = request.unwrap().action;
            match (expected_type, &action) {
                ("search", WebSearchAction::Search { .. }) => {}
                ("open_page", WebSearchAction::OpenPage { .. }) => {}
                ("find_in_page", WebSearchAction::FindInPage { .. }) => {}
                _ => panic!("Wrong action type for {}: got {:?}", json, action),
            }
        }
    }

    #[test]
    fn test_chrome_error_to_tool_error_conversion() {
        // Test Timeout conversion
        let chrome_err = ChromeError::Timeout;
        let tool_err: ToolError = chrome_err.into();
        assert!(matches!(
            tool_err,
            ToolError::Timeout {
                tool: "web_search",
                ..
            }
        ));

        // Test LaunchError conversion
        let chrome_err = ChromeError::LaunchError("test".to_string());
        let tool_err: ToolError = chrome_err.into();
        assert!(matches!(
            tool_err,
            ToolError::Execution {
                tool: "web_search",
                ..
            }
        ));
        assert!(tool_err.to_string().contains("Chrome launch failed"));

        // Test ConnectionError conversion
        let chrome_err = ChromeError::ConnectionError("conn failed".to_string());
        let tool_err: ToolError = chrome_err.into();
        assert!(matches!(
            tool_err,
            ToolError::Execution {
                tool: "web_search",
                ..
            }
        ));
        assert!(tool_err.to_string().contains("Chrome connection failed"));

        // Test ChromeNotFound conversion
        let chrome_err = ChromeError::ChromeNotFound("/usr/bin/chrome".to_string());
        let tool_err: ToolError = chrome_err.into();
        assert!(matches!(
            tool_err,
            ToolError::NotFound {
                tool: "web_search",
                ..
            }
        ));
        assert!(tool_err.to_string().contains("Chrome not found"));
    }
}
