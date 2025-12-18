// Web Search Tool Implementation
// Implements web search capabilities with Search, OpenPage, and FindInPage actions
// Uses HTTP requests to perform actual web searches

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use reqwest;

use crate::ToolError;
use codelet_common::web_search::WebSearchAction;

/// Web Search Tool
/// 
/// This tool implements web search and web content access capabilities
/// with three main actions: Search, OpenPage, and FindInPage.
/// 
/// Unlike the original stub implementation, this actually performs HTTP requests
/// to search the web and fetch page content.
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
        Value::Object(_) => {
            WebSearchAction::deserialize(value).map_err(D::Error::custom)
        }
        // If it's a string, parse it as JSON first
        Value::String(s) => {
            let parsed: Value = serde_json::from_str(&s).map_err(D::Error::custom)?;
            WebSearchAction::deserialize(parsed).map_err(D::Error::custom)
        }
        _ => Err(D::Error::custom("Expected object or string for action"))
    }
}

/// Result from web search operations
#[derive(Debug, Serialize)]
pub struct WebSearchResult {
    pub success: bool,
    pub message: String,
    pub action: WebSearchAction,
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
            description: "Perform web search, open web pages, or find content within pages using web scraping capabilities".to_string(),
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
        // Implement actual web search functionality using HTTP requests
        let (success, message) = match &args.action {
            WebSearchAction::Search { query } => {
                let query = query.as_deref().unwrap_or("");
                if query.is_empty() {
                    (false, "No search query provided".to_string())
                } else {
                    match perform_web_search(query).await {
                        Ok(results) => (true, format!("Search results for '{}': {}", query, results)),
                        Err(e) => (false, format!("Search failed: {}", e)),
                    }
                }
            }
            WebSearchAction::OpenPage { url } => {
                let url = url.as_deref().unwrap_or("");
                if url.is_empty() {
                    (false, "No URL provided".to_string())
                } else {
                    match fetch_page_content(url).await {
                        Ok(content) => (true, format!("Page content from {}: {}", url, content)),
                        Err(e) => (false, format!("Failed to fetch page: {}", e)),
                    }
                }
            }
            WebSearchAction::FindInPage { url, pattern } => {
                let url = url.as_deref().unwrap_or("");
                let pattern = pattern.as_deref().unwrap_or("");
                if url.is_empty() || pattern.is_empty() {
                    (false, "URL or pattern not provided".to_string())
                } else {
                    match find_pattern_in_page(url, pattern).await {
                        Ok(found) => (true, format!("Pattern '{}' found in {}: {}", pattern, url, found)),
                        Err(e) => (false, format!("Pattern search failed: {}", e)),
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

/// Perform a web search using DuckDuckGo Instant Answer API
async fn perform_web_search(query: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    
    // Use DuckDuckGo Instant Answer API as a simple search option
    let url = format!("https://api.duckduckgo.com/?q={}&format=json&no_redirect=1&no_html=1&skip_disambig=1", 
                     urlencoding::encode(query));
    
    let response = client.get(&url).send().await?;
    let text = response.text().await?;
    
    // Parse the JSON response
    let json: serde_json::Value = serde_json::from_str(&text)?;
    
    let mut results = Vec::new();
    
    // Extract instant answer if available
    if let Some(answer) = json.get("Answer").and_then(|v| v.as_str()) {
        if !answer.is_empty() {
            results.push(format!("Answer: {}", answer));
        }
    }
    
    // Extract abstract if available
    if let Some(abstract_text) = json.get("Abstract").and_then(|v| v.as_str()) {
        if !abstract_text.is_empty() {
            results.push(format!("Summary: {}", abstract_text));
        }
    }
    
    // Extract definition if available
    if let Some(definition) = json.get("Definition").and_then(|v| v.as_str()) {
        if !definition.is_empty() {
            results.push(format!("Definition: {}", definition));
        }
    }
    
    // Extract related topics
    if let Some(related) = json.get("RelatedTopics").and_then(|v| v.as_array()) {
        for (i, topic) in related.iter().take(3).enumerate() {
            if let Some(text) = topic.get("Text").and_then(|v| v.as_str()) {
                results.push(format!("Result {}: {}", i + 1, text));
            }
        }
    }
    
    if results.is_empty() {
        Ok("No search results found".to_string())
    } else {
        Ok(results.join("\n"))
    }
}

/// Fetch content from a web page
async fn fetch_page_content(url: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    
    let response = client.get(url).send().await?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()).into());
    }
    
    let text = response.text().await?;
    
    // Basic HTML stripping - extract text content
    let stripped = strip_html_tags(&text);
    
    // Limit content size to avoid overwhelming responses
    let truncated = if stripped.len() > 2000 {
        format!("{}... [truncated]", &stripped[..2000])
    } else {
        stripped
    };
    
    Ok(truncated)
}

/// Find a pattern in a web page
async fn find_pattern_in_page(url: &str, pattern: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let content = fetch_page_content(url).await?;
    
    // Simple case-insensitive search
    let lower_content = content.to_lowercase();
    let lower_pattern = pattern.to_lowercase();
    
    if lower_content.contains(&lower_pattern) {
        // Find context around the match
        if let Some(start) = lower_content.find(&lower_pattern) {
            let context_start = if start >= 100 { start - 100 } else { 0 };
            let context_end = std::cmp::min(start + pattern.len() + 100, content.len());
            
            let context = &content[context_start..context_end];
            Ok(format!("Found match: ...{}...", context))
        } else {
            Ok("Pattern found in page".to_string())
        }
    } else {
        Ok("Pattern not found".to_string())
    }
}

/// Basic HTML tag stripping
fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut inside_tag = false;
    
    for ch in html.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => result.push(ch),
            _ => {}
        }
    }
    
    // Clean up whitespace
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}