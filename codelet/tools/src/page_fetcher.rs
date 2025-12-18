//! Page Fetcher for Web Content Extraction
//!
//! Fetches web pages using Chrome and extracts main content using a
//! Readability-like algorithm implemented in JavaScript.

use crate::chrome_browser::{ChromeBrowser, ChromeError};
use headless_chrome::Tab;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// JavaScript code for extracting main content from a page
/// Uses Readability-like heuristics to identify and extract article content
const EXTRACT_CONTENT_JS: &str = r#"
(function() {
    // Remove unwanted elements
    const removeSelectors = [
        'script', 'style', 'noscript', 'iframe',
        'nav', 'header', 'footer', 'aside',
        '.sidebar', '.navigation', '.menu', '.nav',
        '.header', '.footer', '.ads', '.advertisement',
        '.social-share', '.comments', '.related-posts',
        '[role="navigation"]', '[role="banner"]', '[role="contentinfo"]',
        '.cookie-banner', '.popup', '.modal'
    ];

    // Clone document to avoid modifying original
    const doc = document.cloneNode(true);

    // Remove unwanted elements
    removeSelectors.forEach(function(sel) {
        try {
            doc.querySelectorAll(sel).forEach(function(el) { el.remove(); });
        } catch(e) {}
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
        '.article',
        '.story-body',
        '.article-body'
    ];

    var mainContent = null;
    for (var i = 0; i < contentSelectors.length; i++) {
        var sel = contentSelectors[i];
        try {
            var el = doc.querySelector(sel);
            if (el && el.innerText && el.innerText.trim().length > 200) {
                mainContent = el;
                break;
            }
        } catch(e) {}
    }

    // Fallback to body
    if (!mainContent) {
        mainContent = doc.body;
    }

    // Get text content with some structure preservation
    function extractText(element, depth) {
        if (depth > 10) return '';
        depth = depth || 0;

        var text = '';
        var children = element.childNodes;

        for (var i = 0; i < children.length; i++) {
            var node = children[i];

            if (node.nodeType === Node.TEXT_NODE) {
                var trimmed = node.textContent.trim();
                if (trimmed) text += trimmed + ' ';
            } else if (node.nodeType === Node.ELEMENT_NODE) {
                var tag = node.tagName.toLowerCase();

                // Skip hidden elements
                try {
                    var style = window.getComputedStyle(node);
                    if (style.display === 'none' || style.visibility === 'hidden') {
                        continue;
                    }
                } catch(e) {}

                // Add line breaks for block elements
                if (['p', 'div', 'br', 'li', 'tr'].indexOf(tag) >= 0) {
                    text += '\n';
                }

                // Add heading markers
                if (['h1', 'h2', 'h3', 'h4', 'h5', 'h6'].indexOf(tag) >= 0) {
                    text += '\n\n## ';
                }

                text += extractText(node, depth + 1);

                if (['p', 'div', 'li', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6'].indexOf(tag) >= 0) {
                    text += '\n';
                }
            }
        }
        return text;
    }

    var result = extractText(mainContent, 0);

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
"#;

/// JavaScript code for extracting page metadata
const EXTRACT_METADATA_JS: &str = r#"
(function() {
    var result = {
        title: document.title || null,
        metaDescription: null,
        ogTitle: null,
        ogDescription: null,
        author: null
    };

    // Meta description
    var metaDesc = document.querySelector('meta[name="description"]');
    if (metaDesc) result.metaDescription = metaDesc.getAttribute('content');

    // Open Graph
    var ogTitle = document.querySelector('meta[property="og:title"]');
    if (ogTitle) result.ogTitle = ogTitle.getAttribute('content');

    var ogDesc = document.querySelector('meta[property="og:description"]');
    if (ogDesc) result.ogDescription = ogDesc.getAttribute('content');

    // Author
    var author = document.querySelector('meta[name="author"]');
    if (author) result.author = author.getAttribute('content');

    return JSON.stringify(result);
})();
"#;

/// JavaScript code for extracting headings
const EXTRACT_HEADINGS_JS: &str = r#"
JSON.stringify(
    Array.from(document.querySelectorAll('h1,h2,h3,h4,h5,h6'))
        .slice(0, 20)
        .map(function(h) {
            return {
                level: parseInt(h.tagName.charAt(1)),
                text: h.innerText.trim().substring(0, 200)
            };
        })
);
"#;

/// JavaScript code for extracting links from main content
const EXTRACT_LINKS_JS: &str = r#"
JSON.stringify(
    Array.from(document.querySelectorAll('article a, main a, .content a, body a'))
        .slice(0, 50)
        .filter(function(a) {
            return a.href && !a.href.startsWith('javascript:');
        })
        .map(function(a) {
            return {
                text: a.innerText.trim().substring(0, 100),
                href: a.href
            };
        })
);
"#;

/// Extracted content from a web page
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageContent {
    /// The URL that was fetched
    pub url: String,

    /// Page title
    pub title: Option<String>,

    /// Main article/content text
    pub main_content: String,

    /// Meta description
    pub meta_description: Option<String>,

    /// Headings found in the page
    pub headings: Vec<Heading>,

    /// Links found in main content
    pub links: Vec<Link>,
}

/// A heading element from the page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heading {
    /// Heading level (1-6)
    pub level: u8,

    /// Heading text
    pub text: String,
}

/// A link element from the page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    /// Link text
    pub text: String,

    /// Link URL
    pub href: String,
}

/// Metadata extracted from a page
#[derive(Debug, Clone, Default, Deserialize)]
struct PageMetadata {
    title: Option<String>,
    #[serde(rename = "metaDescription")]
    meta_description: Option<String>,
    #[serde(rename = "ogTitle")]
    _og_title: Option<String>,
    #[serde(rename = "ogDescription")]
    _og_description: Option<String>,
    #[serde(rename = "author")]
    _author: Option<String>,
}

/// Fetches and extracts content from web pages using Chrome
pub struct PageFetcher {
    browser: Arc<ChromeBrowser>,
}

impl PageFetcher {
    /// Create a new PageFetcher with the given browser
    pub fn new(browser: Arc<ChromeBrowser>) -> Self {
        Self { browser }
    }

    /// Fetch a page and extract its main content
    pub fn fetch(&self, url: &str) -> Result<PageContent, ChromeError> {
        let tab = self.browser.new_tab()?;

        // Navigate and wait for page load
        self.browser.navigate_and_wait(&tab, url)?;

        // Extract content
        let content = self.extract_content(&tab, url)?;

        Ok(content)
    }

    /// Extract content from a loaded page
    fn extract_content(&self, tab: &Arc<Tab>, url: &str) -> Result<PageContent, ChromeError> {
        // Get main content
        let main_content = self
            .browser
            .evaluate_js(tab, EXTRACT_CONTENT_JS)?
            .unwrap_or_default();

        // Get metadata
        let metadata_json = self
            .browser
            .evaluate_js(tab, EXTRACT_METADATA_JS)?
            .unwrap_or_else(|| "{}".to_string());

        let metadata: PageMetadata = serde_json::from_str(&metadata_json).unwrap_or_default();

        // Get headings
        let headings_json = self
            .browser
            .evaluate_js(tab, EXTRACT_HEADINGS_JS)?
            .unwrap_or_else(|| "[]".to_string());

        let headings: Vec<Heading> = serde_json::from_str(&headings_json).unwrap_or_default();

        // Get links
        let links_json = self
            .browser
            .evaluate_js(tab, EXTRACT_LINKS_JS)?
            .unwrap_or_else(|| "[]".to_string());

        let links: Vec<Link> = serde_json::from_str(&links_json).unwrap_or_default();

        Ok(PageContent {
            url: url.to_string(),
            title: metadata.title,
            main_content,
            meta_description: metadata.meta_description,
            headings,
            links,
        })
    }

    /// Find a pattern in a page and return matches with context
    pub fn find_in_page(&self, url: &str, pattern: &str) -> Result<Vec<String>, ChromeError> {
        let tab = self.browser.new_tab()?;

        // Navigate and wait for page load
        self.browser.navigate_and_wait(&tab, url)?;

        // Search for pattern using JavaScript
        let escaped_pattern = pattern.replace('\\', "\\\\").replace('"', "\\\"");
        let find_js = format!(
            r#"
            (function() {{
                var pattern = "{escaped_pattern}".toLowerCase();
                var walker = document.createTreeWalker(
                    document.body,
                    NodeFilter.SHOW_TEXT,
                    null,
                    false
                );

                var matches = [];
                var node;
                while (node = walker.nextNode()) {{
                    var text = node.textContent;
                    var lower = text.toLowerCase();
                    var pos = 0;
                    while ((pos = lower.indexOf(pattern, pos)) !== -1) {{
                        var start = Math.max(0, pos - 50);
                        var end = Math.min(text.length, pos + pattern.length + 50);
                        var context = text.substring(start, end).trim();
                        if (context.length > 0) {{
                            matches.push(context);
                        }}
                        if (matches.length >= 10) break;
                        pos++;
                    }}
                    if (matches.length >= 10) break;
                }}
                return JSON.stringify(matches);
            }})();
            "#
        );

        let matches_json = self
            .browser
            .evaluate_js(&tab, &find_js)?
            .unwrap_or_else(|| "[]".to_string());

        let matches: Vec<String> = serde_json::from_str(&matches_json).unwrap_or_default();

        Ok(matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_content_default() {
        let content = PageContent::default();
        assert!(content.url.is_empty());
        assert!(content.title.is_none());
        assert!(content.main_content.is_empty());
        assert!(content.headings.is_empty());
        assert!(content.links.is_empty());
    }
}
