//! Search Engine for Web Search Results
//!
//! Performs web searches using DuckDuckGo's HTML interface via Chrome.
//! Extracts search results including titles, URLs, and snippets.

use crate::chrome_browser::{ChromeBrowser, ChromeError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// JavaScript code for extracting DuckDuckGo search results
/// Handles multiple possible HTML structures including legacy and current DDG layouts
const EXTRACT_DDG_RESULTS_JS: &str = r#"
(function() {
    var results = [];

    // Check for CAPTCHA page
    if (document.body.innerText.includes('bots use DuckDuckGo') ||
        document.body.innerText.includes('CAPTCHA') ||
        document.querySelector('input[name="captcha"]')) {
        return JSON.stringify({error: 'captcha', results: []});
    }

    // Helper function to extract real URL from DDG tracking wrapper
    // DDG wraps URLs like: https://duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com&rut=...
    function extractRealUrl(href) {
        if (!href) return null;

        // Check if it's a DDG tracking URL
        if (href.includes('duckduckgo.com/l/?uddg=')) {
            try {
                var url = new URL(href);
                var uddg = url.searchParams.get('uddg');
                if (uddg) {
                    return decodeURIComponent(uddg);
                }
            } catch(e) {}
        }

        // If not a tracking URL and not a DDG internal link, return as-is
        if (!href.includes('duckduckgo.com') && href.startsWith('http')) {
            return href;
        }

        return null;
    }

    // DuckDuckGo HTML results - try multiple selectors for different page versions
    var resultElements = document.querySelectorAll(
        '.result, .web-result, .results_links, .result--web, [data-result]'
    );

    // Fallback: look for any links that look like search results
    if (resultElements.length === 0) {
        var allLinks = document.querySelectorAll('a[href]');
        for (var i = 0; i < allLinks.length && results.length < 10; i++) {
            var link = allLinks[i];
            var text = link.innerText.trim();
            var realUrl = extractRealUrl(link.href);

            // Skip empty links and javascript links
            if (!realUrl || !text || text.length < 5 ||
                link.href.startsWith('javascript:') ||
                link.href.startsWith('#') ||
                link.href.includes('/feedback') ||
                link.href.includes('/privacy')) {
                continue;
            }

            results.push({
                title: text.substring(0, 200),
                url: realUrl,
                snippet: ''
            });
        }
        return JSON.stringify({error: null, results: results});
    }

    for (var i = 0; i < resultElements.length && results.length < 10; i++) {
        var el = resultElements[i];

        // Skip ad results
        if (el.classList.contains('result--ad')) {
            continue;
        }

        // Find the title link - try multiple selectors
        var titleEl = el.querySelector('.result__a, .result__title a, a.result__a, a[data-testid="result-title-a"], h2 a, a');
        if (!titleEl) continue;

        var title = titleEl.innerText.trim();
        var realUrl = extractRealUrl(titleEl.href);

        // Skip if no valid URL found
        if (!realUrl || titleEl.href.startsWith('javascript:')) {
            continue;
        }

        // Find the snippet - try multiple selectors
        var snippetEl = el.querySelector('.result__snippet, .result__body, .result__description, [data-result="snippet"]');
        var snippet = snippetEl ? snippetEl.innerText.trim() : '';

        if (title && realUrl) {
            results.push({
                title: title.substring(0, 200),
                url: realUrl,
                snippet: snippet.substring(0, 500)
            });
        }
    }

    return JSON.stringify({error: null, results: results});
})();
"#;

/// A single search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The title of the search result
    pub title: String,

    /// The URL of the result
    pub url: String,

    /// A snippet/description of the result
    pub snippet: String,
}

/// Response from DDG extraction script
#[derive(Debug, Deserialize)]
struct DdgResponse {
    error: Option<String>,
    results: Vec<SearchResult>,
}

/// Search engine that uses Chrome to scrape DuckDuckGo
pub struct SearchEngine {
    browser: Arc<ChromeBrowser>,
}

impl SearchEngine {
    /// Create a new SearchEngine with the given browser
    pub fn new(browser: Arc<ChromeBrowser>) -> Self {
        Self { browser }
    }

    /// Perform a web search using DuckDuckGo's HTML interface
    ///
    /// Uses the lite/HTML version of DuckDuckGo which doesn't require JavaScript
    /// but we use Chrome anyway to handle any edge cases and cookies.
    ///
    /// Note: DuckDuckGo may show CAPTCHA for headless browsers. If this happens,
    /// the function returns an error indicating CAPTCHA detection.
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, ChromeError> {
        let tab = self.browser.new_tab()?;

        // Use DuckDuckGo's HTML-only version for more reliable scraping
        let encoded_query = urlencoding::encode(query);
        let search_url = format!("https://html.duckduckgo.com/html/?q={encoded_query}");

        // Navigate and wait for results
        if let Err(e) = self.browser.navigate_and_wait(&tab, &search_url) {
            self.browser.cleanup_tab(&tab);
            return Err(e);
        }

        // Extract results using JavaScript
        let results_json = match self.browser.evaluate_js(&tab, EXTRACT_DDG_RESULTS_JS) {
            Ok(json) => json.unwrap_or_else(|| r#"{"error":null,"results":[]}"#.to_string()),
            Err(e) => {
                self.browser.cleanup_tab(&tab);
                return Err(e);
            }
        };

        // Clean up tab before processing results
        self.browser.cleanup_tab(&tab);

        // Try to parse as new format with error detection
        if let Ok(response) = serde_json::from_str::<DdgResponse>(&results_json) {
            if let Some(error) = response.error {
                if error == "captcha" {
                    return Err(ChromeError::EvaluationError(
                        "DuckDuckGo CAPTCHA detected - search blocked. Try using a different search method or wait and retry.".to_string()
                    ));
                }
                return Err(ChromeError::EvaluationError(format!(
                    "Search error: {error}"
                )));
            }
            return Ok(response.results);
        }

        // Fallback: try to parse as old format (just array of results)
        let results: Vec<SearchResult> = serde_json::from_str(&results_json).unwrap_or_default();

        Ok(results)
    }

    /// Search with a minimum number of results requirement
    ///
    /// Returns an error if fewer than `min_results` are found
    pub fn search_with_minimum(
        &self,
        query: &str,
        min_results: usize,
    ) -> Result<Vec<SearchResult>, ChromeError> {
        let results = self.search(query)?;

        if results.len() < min_results {
            return Err(ChromeError::EvaluationError(format!(
                "Expected at least {} results, got {}",
                min_results,
                results.len()
            )));
        }

        Ok(results)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            title: "Test Title".to_string(),
            url: "https://example.com".to_string(),
            snippet: "Test snippet".to_string(),
        };

        let json = serde_json::to_string(&result).expect("Serialization should work");
        assert!(json.contains("Test Title"));
        assert!(json.contains("https://example.com"));

        let deserialized: SearchResult =
            serde_json::from_str(&json).expect("Deserialization should work");
        assert_eq!(deserialized.title, "Test Title");
        assert_eq!(deserialized.url, "https://example.com");
    }
}
