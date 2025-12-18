# WEB-002: Chrome DevTools Web Search Implementation Plan

## Executive Summary

Replace the current naive web search implementation with `rust-headless-chrome` to provide full browser-based web search, page fetching, and content extraction with JavaScript support. This enables codelet to access modern web content including SPAs, JavaScript-rendered pages, and dynamic content - replicating the capabilities that Codex achieves via OpenAI's server-side web search tool.

---

## Current State Analysis

### What We Have (WEB-001)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    CURRENT IMPLEMENTATION                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  WebSearchAction::Search                                                │
│  └─ DuckDuckGo Instant Answer API                                      │
│     • Only returns "instant answers" (Wikipedia summaries)             │
│     • No actual search result URLs                                     │
│     • Many queries return empty results                                │
│                                                                         │
│  WebSearchAction::OpenPage                                              │
│  └─ reqwest HTTP GET + strip_html_tags()                               │
│     • Naive character-by-character tag stripping                       │
│     • Includes script/style/nav/footer content                         │
│     • No JavaScript execution                                          │
│     • Truncates to 2000 chars                                          │
│                                                                         │
│  WebSearchAction::FindInPage                                            │
│  └─ Case-insensitive substring search                                  │
│     • No semantic understanding                                        │
│     • No DOM awareness                                                  │
│     • Returns 100 char context window                                  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### Critical Flaws

1. **No JavaScript Support**: SPAs, React/Vue/Angular sites won't render
2. **Naive HTML Stripping**: Includes navigation, ads, scripts in output
3. **Limited Search**: DuckDuckGo Instant Answers aren't real search results
4. **No Content Intelligence**: Can't identify main article content

---

## Proposed Architecture

### rust-headless-chrome Integration

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    NEW ARCHITECTURE                                     │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                     ChromeWebSearchTool                          │   │
│  │                                                                  │   │
│  │  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐       │   │
│  │  │ BrowserPool  │    │ SearchEngine │    │ ContentExtract│       │   │
│  │  │              │    │              │    │              │       │   │
│  │  │ • Lazy init  │    │ • DuckDuckGo │    │ • Readability│       │   │
│  │  │ • Connection │    │   HTML scrape│    │   algorithm  │       │   │
│  │  │   pooling    │    │ • Or SearXNG │    │ • DOM queries│       │   │
│  │  │ • Timeout    │    │ • Brave API  │    │ • Structured │       │   │
│  │  │   handling   │    │   (optional) │    │   output     │       │   │
│  │  └──────────────┘    └──────────────┘    └──────────────┘       │   │
│  │         │                   │                   │                │   │
│  │         ▼                   ▼                   ▼                │   │
│  │  ┌──────────────────────────────────────────────────────────┐   │   │
│  │  │              Chrome DevTools Protocol                     │   │   │
│  │  │  • Page.navigate()                                       │   │   │
│  │  │  • Runtime.evaluate() - JavaScript execution             │   │   │
│  │  │  • DOM.getDocument() - DOM traversal                     │   │   │
│  │  │  • Network.* - Request interception                      │   │   │
│  │  └──────────────────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  Connection Options:                                                    │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ Option A: Connect to existing Chrome                            │   │
│  │   Browser::connect("ws://127.0.0.1:9222/devtools/browser/...")  │   │
│  │                                                                  │   │
│  │ Option B: Launch local Chrome with custom path                  │   │
│  │   LaunchOptions { path: Some("/path/to/chrome"), ... }          │   │
│  │                                                                  │   │
│  │ Option C: Auto-detect installed Chrome                          │   │
│  │   LaunchOptions { path: None, ... } // Disable "fetch" feature  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Details

### 1. Dependencies

Add to `Cargo.toml` (workspace):

```toml
[workspace.dependencies]
# Chrome DevTools Protocol
headless_chrome = "1.0"  # WITHOUT "fetch" feature = no auto-download
```

Add to `tools/Cargo.toml`:

```toml
[dependencies]
headless_chrome.workspace = true
```

### 2. Browser Management

```rust
// tools/src/chrome_browser.rs

use headless_chrome::{Browser, LaunchOptions, Tab};
use std::sync::Arc;
use std::path::PathBuf;
use std::time::Duration;

/// Configuration for Chrome browser connection
#[derive(Clone, Debug)]
pub struct ChromeConfig {
    /// WebSocket URL for existing Chrome instance (e.g., "ws://127.0.0.1:9222/...")
    pub ws_url: Option<String>,

    /// Path to Chrome/Chromium binary (auto-detect if None)
    pub chrome_path: Option<PathBuf>,

    /// Run in headless mode
    pub headless: bool,

    /// Page load timeout
    pub timeout: Duration,

    /// User agent string
    pub user_agent: Option<String>,
}

impl Default for ChromeConfig {
    fn default() -> Self {
        Self {
            ws_url: None,
            chrome_path: None,
            headless: true,
            timeout: Duration::from_secs(30),
            user_agent: Some("Mozilla/5.0 (compatible; Codelet/1.0)".into()),
        }
    }
}

/// Manages Chrome browser lifecycle
pub struct ChromeBrowser {
    browser: Arc<Browser>,
    config: ChromeConfig,
}

impl ChromeBrowser {
    /// Connect to existing Chrome or launch new instance
    pub fn new(config: ChromeConfig) -> Result<Self, ChromeError> {
        let browser = if let Some(ws_url) = &config.ws_url {
            // Connect to existing Chrome with remote debugging
            Browser::connect_with_timeout(
                ws_url.clone(),
                config.timeout,
            )?
        } else {
            // Launch new Chrome instance
            Browser::new(LaunchOptions {
                headless: config.headless,
                path: config.chrome_path.clone(),
                idle_browser_timeout: config.timeout,
                ..Default::default()
            })?
        };

        Ok(Self {
            browser: Arc::new(browser),
            config,
        })
    }

    /// Get a new tab for navigation
    pub fn new_tab(&self) -> Result<Arc<Tab>, ChromeError> {
        let tab = self.browser.new_tab()?;

        // Set user agent if configured
        if let Some(ua) = &self.config.user_agent {
            tab.set_user_agent(ua, None, None)?;
        }

        Ok(tab)
    }
}
```

### 3. Page Fetching with Full Rendering

```rust
// tools/src/page_fetcher.rs

use headless_chrome::Tab;
use std::sync::Arc;
use std::time::Duration;

/// Result of fetching and extracting page content
#[derive(Debug, Clone)]
pub struct PageContent {
    pub url: String,
    pub title: Option<String>,
    pub main_content: String,
    pub meta_description: Option<String>,
    pub headings: Vec<Heading>,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone)]
pub struct Heading {
    pub level: u8,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Link {
    pub text: String,
    pub href: String,
}

/// Fetches and extracts content from web pages using Chrome
pub struct PageFetcher {
    browser: Arc<ChromeBrowser>,
}

impl PageFetcher {
    pub fn new(browser: Arc<ChromeBrowser>) -> Self {
        Self { browser }
    }

    /// Navigate to URL and extract main content
    pub async fn fetch(&self, url: &str) -> Result<PageContent, FetchError> {
        let tab = self.browser.new_tab()?;

        // Navigate and wait for page load
        tab.navigate_to(url)?;
        tab.wait_until_navigated()?;

        // Wait for JavaScript rendering (important for SPAs)
        // Wait for network idle or timeout
        tab.wait_for_element("body")?;
        std::thread::sleep(Duration::from_millis(500)); // Allow JS to settle

        // Extract content using JavaScript
        let content = self.extract_content(&tab)?;

        // Close tab to free resources
        drop(tab);

        Ok(content)
    }

    /// Extract main content using Readability-like algorithm via JS
    fn extract_content(&self, tab: &Tab) -> Result<PageContent, FetchError> {
        // Get page title
        let title: Option<String> = tab.evaluate(
            "document.title || null",
            false,
        )?.value.and_then(|v| v.as_str().map(String::from));

        // Get meta description
        let meta_description: Option<String> = tab.evaluate(
            r#"
            (function() {
                const meta = document.querySelector('meta[name="description"]');
                return meta ? meta.getAttribute('content') : null;
            })()
            "#,
            false,
        )?.value.and_then(|v| v.as_str().map(String::from));

        // Extract main content using Readability-like heuristics
        let main_content: String = tab.evaluate(
            include_str!("js/extract_content.js"),
            false,
        )?.value
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();

        // Extract headings
        let headings_json: String = tab.evaluate(
            r#"
            JSON.stringify(
                Array.from(document.querySelectorAll('h1,h2,h3,h4,h5,h6'))
                    .slice(0, 20)
                    .map(h => ({
                        level: parseInt(h.tagName.charAt(1)),
                        text: h.innerText.trim().substring(0, 200)
                    }))
            )
            "#,
            false,
        )?.value
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "[]".into());

        let headings: Vec<Heading> = serde_json::from_str(&headings_json)
            .unwrap_or_default();

        // Extract links (limited to main content area)
        let links_json: String = tab.evaluate(
            r#"
            JSON.stringify(
                Array.from(document.querySelectorAll('article a, main a, .content a'))
                    .slice(0, 50)
                    .filter(a => a.href && !a.href.startsWith('javascript:'))
                    .map(a => ({
                        text: a.innerText.trim().substring(0, 100),
                        href: a.href
                    }))
            )
            "#,
            false,
        )?.value
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "[]".into());

        let links: Vec<Link> = serde_json::from_str(&links_json)
            .unwrap_or_default();

        Ok(PageContent {
            url: tab.get_url(),
            title,
            main_content,
            meta_description,
            headings,
            links,
        })
    }
}
```

### 4. Content Extraction JavaScript

```javascript
// tools/src/js/extract_content.js
// Embedded in Rust binary via include_str!()

(function() {
    // Remove unwanted elements
    const removeSelectors = [
        'script', 'style', 'noscript', 'iframe',
        'nav', 'header', 'footer', 'aside',
        '.sidebar', '.navigation', '.menu', '.nav',
        '.header', '.footer', '.ads', '.advertisement',
        '.social-share', '.comments', '.related-posts',
        '[role="navigation"]', '[role="banner"]', '[role="contentinfo"]'
    ];

    // Clone document to avoid modifying original
    const doc = document.cloneNode(true);

    // Remove unwanted elements
    removeSelectors.forEach(sel => {
        doc.querySelectorAll(sel).forEach(el => el.remove());
    });

    // Try to find main content container
    const contentSelectors = [
        'article',
        'main',
        '[role="main"]',
        '.post-content',
        '.article-content',
        '.entry-content',
        '.content',
        '#content',
        '.post',
        '.article'
    ];

    let mainContent = null;
    for (const sel of contentSelectors) {
        const el = doc.querySelector(sel);
        if (el && el.innerText.trim().length > 200) {
            mainContent = el;
            break;
        }
    }

    // Fallback to body
    if (!mainContent) {
        mainContent = doc.body;
    }

    // Get text content with some structure preservation
    function extractText(element, depth = 0) {
        if (depth > 10) return '';

        let text = '';
        for (const node of element.childNodes) {
            if (node.nodeType === Node.TEXT_NODE) {
                const trimmed = node.textContent.trim();
                if (trimmed) text += trimmed + ' ';
            } else if (node.nodeType === Node.ELEMENT_NODE) {
                const tag = node.tagName.toLowerCase();

                // Skip hidden elements
                const style = window.getComputedStyle(node);
                if (style.display === 'none' || style.visibility === 'hidden') {
                    continue;
                }

                // Add line breaks for block elements
                if (['p', 'div', 'br', 'li', 'tr'].includes(tag)) {
                    text += '\n';
                }

                // Add heading markers
                if (['h1', 'h2', 'h3', 'h4', 'h5', 'h6'].includes(tag)) {
                    text += '\n\n## ';
                }

                text += extractText(node, depth + 1);

                if (['p', 'div', 'li', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6'].includes(tag)) {
                    text += '\n';
                }
            }
        }
        return text;
    }

    let result = extractText(mainContent);

    // Clean up whitespace
    result = result
        .replace(/[ \t]+/g, ' ')
        .replace(/\n\s*\n\s*\n/g, '\n\n')
        .trim();

    // Truncate if too long (keep first 8000 chars for LLM context)
    if (result.length > 8000) {
        result = result.substring(0, 8000) + '\n\n[Content truncated...]';
    }

    return result;
})();
```

### 5. Search Implementation

```rust
// tools/src/search_engine.rs

use headless_chrome::Tab;

/// Search result from web search
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Performs web search using browser
pub struct SearchEngine {
    browser: Arc<ChromeBrowser>,
}

impl SearchEngine {
    /// Search using DuckDuckGo HTML (no API needed)
    pub async fn search_duckduckgo(&self, query: &str) -> Result<Vec<SearchResult>, SearchError> {
        let tab = self.browser.new_tab()?;

        // Use DuckDuckGo HTML version (works without JavaScript)
        let search_url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );

        tab.navigate_to(&search_url)?;
        tab.wait_until_navigated()?;

        // Extract search results
        let results_json: String = tab.evaluate(
            r#"
            JSON.stringify(
                Array.from(document.querySelectorAll('.result'))
                    .slice(0, 10)
                    .map(r => ({
                        title: r.querySelector('.result__title')?.innerText?.trim() || '',
                        url: r.querySelector('.result__url')?.href ||
                             r.querySelector('.result__a')?.href || '',
                        snippet: r.querySelector('.result__snippet')?.innerText?.trim() || ''
                    }))
                    .filter(r => r.title && r.url)
            )
            "#,
            false,
        )?.value
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "[]".into());

        let results: Vec<SearchResult> = serde_json::from_str(&results_json)?;

        drop(tab);
        Ok(results)
    }

    /// Alternative: Search using Google (with cookie consent handling)
    pub async fn search_google(&self, query: &str) -> Result<Vec<SearchResult>, SearchError> {
        let tab = self.browser.new_tab()?;

        let search_url = format!(
            "https://www.google.com/search?q={}",
            urlencoding::encode(query)
        );

        tab.navigate_to(&search_url)?;
        tab.wait_until_navigated()?;

        // Handle cookie consent if present
        if let Ok(consent_button) = tab.wait_for_element_with_custom_timeout(
            "button[id*='accept'], button[id*='agree']",
            Duration::from_secs(2),
        ) {
            let _ = consent_button.click();
            std::thread::sleep(Duration::from_millis(500));
        }

        // Extract search results
        let results_json: String = tab.evaluate(
            r#"
            JSON.stringify(
                Array.from(document.querySelectorAll('div.g'))
                    .slice(0, 10)
                    .map(r => ({
                        title: r.querySelector('h3')?.innerText?.trim() || '',
                        url: r.querySelector('a')?.href || '',
                        snippet: r.querySelector('.VwiC3b')?.innerText?.trim() || ''
                    }))
                    .filter(r => r.title && r.url && !r.url.includes('google.com'))
            )
            "#,
            false,
        )?.value
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "[]".into());

        let results: Vec<SearchResult> = serde_json::from_str(&results_json)?;

        drop(tab);
        Ok(results)
    }
}
```

### 6. Updated WebSearchTool

```rust
// tools/src/web_search.rs (updated)

use crate::chrome_browser::{ChromeBrowser, ChromeConfig};
use crate::page_fetcher::PageFetcher;
use crate::search_engine::SearchEngine;
use codelet_common::web_search::WebSearchAction;
use std::sync::Arc;
use tokio::sync::OnceCell;

/// Lazily initialized browser instance
static BROWSER: OnceCell<Arc<ChromeBrowser>> = OnceCell::const_new();

/// Get or initialize the browser
async fn get_browser() -> Result<Arc<ChromeBrowser>, ToolError> {
    BROWSER.get_or_try_init(|| async {
        let config = ChromeConfig::default();
        ChromeBrowser::new(config)
            .map(Arc::new)
            .map_err(|e| ToolError::BrowserInit(e.to_string()))
    }).await.cloned()
}

#[derive(Clone, Debug)]
pub struct WebSearchTool;

impl Tool for WebSearchTool {
    const NAME: &'static str = "web_search";
    type Error = ToolError;
    type Args = WebSearchRequest;
    type Output = WebSearchResult;

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let browser = get_browser().await?;

        let (success, message) = match &args.action {
            WebSearchAction::Search { query } => {
                let query = query.as_deref().unwrap_or("");
                if query.is_empty() {
                    (false, "No search query provided".to_string())
                } else {
                    let engine = SearchEngine::new(browser);
                    match engine.search_duckduckgo(query).await {
                        Ok(results) if results.is_empty() => {
                            (false, "No search results found".to_string())
                        }
                        Ok(results) => {
                            let formatted = results.iter()
                                .enumerate()
                                .map(|(i, r)| format!(
                                    "{}. {}\n   URL: {}\n   {}\n",
                                    i + 1, r.title, r.url, r.snippet
                                ))
                                .collect::<Vec<_>>()
                                .join("\n");
                            (true, format!("Search results for '{}':\n\n{}", query, formatted))
                        }
                        Err(e) => (false, format!("Search failed: {}", e)),
                    }
                }
            }

            WebSearchAction::OpenPage { url } => {
                let url = url.as_deref().unwrap_or("");
                if url.is_empty() {
                    (false, "No URL provided".to_string())
                } else {
                    let fetcher = PageFetcher::new(browser);
                    match fetcher.fetch(url).await {
                        Ok(content) => {
                            let mut output = String::new();

                            if let Some(title) = &content.title {
                                output.push_str(&format!("# {}\n\n", title));
                            }

                            if let Some(desc) = &content.meta_description {
                                output.push_str(&format!("*{}*\n\n", desc));
                            }

                            output.push_str(&content.main_content);

                            if !content.headings.is_empty() {
                                output.push_str("\n\n## Page Structure:\n");
                                for h in &content.headings {
                                    output.push_str(&format!(
                                        "{} {}\n",
                                        "#".repeat(h.level as usize),
                                        h.text
                                    ));
                                }
                            }

                            (true, output)
                        }
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
                    let browser = get_browser().await?;
                    let tab = browser.new_tab()?;

                    tab.navigate_to(url)?;
                    tab.wait_until_navigated()?;

                    // Use browser's find function
                    let matches_json: String = tab.evaluate(
                        &format!(r#"
                        (function() {{
                            const pattern = "{}".toLowerCase();
                            const walker = document.createTreeWalker(
                                document.body,
                                NodeFilter.SHOW_TEXT,
                                null,
                                false
                            );

                            const matches = [];
                            let node;
                            while (node = walker.nextNode()) {{
                                const text = node.textContent;
                                const lower = text.toLowerCase();
                                let pos = 0;
                                while ((pos = lower.indexOf(pattern, pos)) !== -1) {{
                                    const start = Math.max(0, pos - 50);
                                    const end = Math.min(text.length, pos + pattern.length + 50);
                                    const context = text.substring(start, end);
                                    matches.push(context);
                                    if (matches.length >= 10) break;
                                    pos++;
                                }}
                                if (matches.length >= 10) break;
                            }}
                            return JSON.stringify(matches);
                        }})()
                        "#, pattern.replace('"', r#"\""#)),
                        false,
                    )?.value
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_else(|| "[]".into());

                    let matches: Vec<String> = serde_json::from_str(&matches_json)
                        .unwrap_or_default();

                    drop(tab);

                    if matches.is_empty() {
                        (true, format!("Pattern '{}' not found in page", pattern))
                    } else {
                        let formatted = matches.iter()
                            .enumerate()
                            .map(|(i, m)| format!("{}. ...{}...", i + 1, m.trim()))
                            .collect::<Vec<_>>()
                            .join("\n");
                        (true, format!(
                            "Found {} match(es) for '{}' in {}:\n\n{}",
                            matches.len(), pattern, url, formatted
                        ))
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
```

---

## Configuration

### Environment Variables

```bash
# Optional: Connect to existing Chrome instance
CODELET_CHROME_WS_URL=ws://127.0.0.1:9222/devtools/browser/...

# Optional: Custom Chrome path
CODELET_CHROME_PATH=/Applications/Google Chrome.app/Contents/MacOS/Google Chrome

# Optional: Run with visible browser (for debugging)
CODELET_CHROME_HEADLESS=false

# Optional: Page load timeout (seconds)
CODELET_CHROME_TIMEOUT=30
```

### Starting Chrome with Remote Debugging

For users who want to connect to an existing Chrome:

```bash
# macOS
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome \
  --remote-debugging-port=9222 \
  --headless=new

# Get WebSocket URL
curl http://127.0.0.1:9222/json/version | jq -r '.webSocketDebuggerUrl'
```

---

## Comparison: Before vs After

| Feature | Before (WEB-001) | After (WEB-002) |
|---------|------------------|-----------------|
| **JavaScript Support** | None | Full V8 engine |
| **SPA/React Sites** | Broken | Works |
| **Content Extraction** | Naive tag stripping | Readability-like algorithm |
| **Search Results** | DuckDuckGo Instant Answers only | Full search results with URLs |
| **Content Structure** | Lost | Preserved (headings, links) |
| **Hidden Elements** | Included | Filtered by CSS visibility |
| **Output Quality** | Poor (includes nav/ads) | Clean main content |
| **Performance** | Fast | Slower (browser overhead) |
| **Dependencies** | reqwest only | Chrome required |

---

## Acceptance Criteria

1. **Search Action**
   - Returns actual search results with titles, URLs, and snippets
   - Supports at least 10 results per query
   - Works with complex queries including special characters

2. **OpenPage Action**
   - Renders JavaScript before extracting content
   - Extracts main article content (not nav/footer/ads)
   - Preserves heading structure
   - Includes page title and meta description
   - Handles SPAs and dynamic content

3. **FindInPage Action**
   - Searches rendered DOM (after JavaScript)
   - Returns context around matches
   - Case-insensitive matching
   - Limits results to prevent overwhelming output

4. **Browser Management**
   - Lazy initialization (don't start browser until needed)
   - Proper cleanup on shutdown
   - Timeout handling
   - Error recovery

5. **Configuration**
   - Support connecting to existing Chrome
   - Support custom Chrome path
   - Configurable timeouts
   - Headless/headed mode toggle

---

## File Changes Summary

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` | Modify | Add `headless_chrome` to workspace deps |
| `tools/Cargo.toml` | Modify | Add `headless_chrome` dependency |
| `tools/src/chrome_browser.rs` | Create | Browser management |
| `tools/src/page_fetcher.rs` | Create | Page fetching with content extraction |
| `tools/src/search_engine.rs` | Create | Search implementation |
| `tools/src/js/extract_content.js` | Create | Content extraction JavaScript |
| `tools/src/web_search.rs` | Modify | Update to use Chrome |
| `tools/src/lib.rs` | Modify | Export new modules |

---

## Testing Strategy

1. **Unit Tests**
   - Mock Chrome responses for deterministic testing
   - Test content extraction on sample HTML

2. **Integration Tests**
   - Test against real websites (example.com, wikipedia.org)
   - Test JavaScript rendering (simple SPA test page)
   - Test search result parsing

3. **Manual Testing**
   - Test with various real-world sites
   - Test error handling (timeouts, network errors)
   - Test memory usage over many requests

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Chrome not installed | Tool fails | Clear error message with install instructions |
| Browser crash | Memory leak | Proper cleanup, process monitoring |
| Slow page loads | Timeout | Configurable timeout, abort slow pages |
| Anti-bot protection | Blocked | User-agent spoofing, rate limiting |
| Memory usage | High | Tab cleanup, browser restart after N requests |

---

## Future Enhancements (Out of Scope)

- Screenshot capture
- PDF rendering
- Cookie/session persistence
- Proxy support
- Browser extension loading
- Multiple browser support (Firefox via WebDriver)
