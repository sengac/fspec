//! Chrome Browser Management for Web Search
//!
//! Provides a wrapper around rust-headless-chrome for controlling Chrome/Chromium
//! via the DevTools Protocol. Supports three connection modes:
//! 1. Connect to existing Chrome via WebSocket URL
//! 2. Launch Chrome with a specific binary path
//! 3. Auto-detect installed Chrome/Chromium

use headless_chrome::{Browser, LaunchOptions, Tab};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during Chrome browser operations
#[derive(Debug, Error)]
pub enum ChromeError {
    #[error("Failed to launch Chrome: {0}")]
    LaunchError(String),

    #[error("Failed to connect to Chrome: {0}")]
    ConnectionError(String),

    #[error("Failed to create new tab: {0}")]
    TabError(String),

    #[error("Navigation failed: {0}")]
    NavigationError(String),

    #[error("JavaScript evaluation failed: {0}")]
    EvaluationError(String),

    #[error("Chrome not found at specified path: {0}")]
    ChromeNotFound(String),

    #[error("Timeout waiting for page load")]
    Timeout,
}

/// Configuration for Chrome browser connection
#[derive(Clone, Debug)]
pub struct ChromeConfig {
    /// WebSocket URL for existing Chrome instance (e.g., "ws://127.0.0.1:9222/...")
    /// Set via CODELET_CHROME_WS_URL environment variable
    pub ws_url: Option<String>,

    /// Path to Chrome/Chromium binary (auto-detect if None)
    /// Set via CODELET_CHROME_PATH environment variable
    pub chrome_path: Option<PathBuf>,

    /// Run in headless mode (default: true)
    /// Set via CODELET_CHROME_HEADLESS environment variable
    pub headless: bool,

    /// Page load timeout in seconds (default: 30)
    /// Set via CODELET_CHROME_TIMEOUT environment variable
    pub timeout: Duration,

    /// User agent string
    pub user_agent: Option<String>,
}

impl Default for ChromeConfig {
    fn default() -> Self {
        Self {
            ws_url: std::env::var("CODELET_CHROME_WS_URL").ok(),
            chrome_path: std::env::var("CODELET_CHROME_PATH").ok().map(PathBuf::from),
            headless: std::env::var("CODELET_CHROME_HEADLESS")
                .map(|v| v != "false")
                .unwrap_or(true),
            // Default timeout of 5 minutes (300 seconds) for idle browser
            // This prevents the browser from closing during long conversations
            timeout: Duration::from_secs(
                std::env::var("CODELET_CHROME_TIMEOUT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(300),
            ),
            user_agent: Some("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".into()),
        }
    }
}

/// Manages Chrome browser lifecycle
///
/// This struct wraps a headless_chrome::Browser and provides a simplified
/// interface for web scraping operations.
pub struct ChromeBrowser {
    browser: Arc<Browser>,
    config: ChromeConfig,
}

impl ChromeBrowser {
    /// Create a new ChromeBrowser instance
    ///
    /// Depending on the configuration, this will either:
    /// 1. Connect to an existing Chrome instance via WebSocket URL
    /// 2. Launch a new Chrome instance with the specified or auto-detected path
    pub fn new(config: ChromeConfig) -> Result<Self, ChromeError> {
        let browser = if let Some(ws_url) = &config.ws_url {
            // Connect to existing Chrome with remote debugging
            Browser::connect_with_timeout(ws_url.clone(), config.timeout)
                .map_err(|e| ChromeError::ConnectionError(e.to_string()))?
        } else {
            // Build launch options
            let launch_options = LaunchOptions {
                headless: config.headless,
                path: config.chrome_path.clone(),
                idle_browser_timeout: config.timeout,
                sandbox: true,
                ..Default::default()
            };

            // Launch new Chrome instance
            Browser::new(launch_options).map_err(|e| ChromeError::LaunchError(e.to_string()))?
        };

        Ok(Self {
            browser: Arc::new(browser),
            config,
        })
    }

    /// Get the browser configuration
    pub fn config(&self) -> &ChromeConfig {
        &self.config
    }

    /// Get a reference to the underlying browser
    pub fn browser(&self) -> &Browser {
        &self.browser
    }

    /// Create a new tab for navigation
    ///
    /// Each tab is independent and can be used for parallel page fetching.
    pub fn new_tab(&self) -> Result<Arc<Tab>, ChromeError> {
        let tab = self
            .browser
            .new_tab()
            .map_err(|e| ChromeError::TabError(e.to_string()))?;

        // Set user agent if configured
        if let Some(ua) = &self.config.user_agent {
            tab.set_user_agent(ua, None, None)
                .map_err(|e| ChromeError::TabError(format!("Failed to set user agent: {e}")))?;
        }

        Ok(tab)
    }

    /// Navigate to a URL and wait for the page to load
    pub fn navigate_and_wait(&self, tab: &Arc<Tab>, url: &str) -> Result<(), ChromeError> {
        // Navigate to the URL
        tab.navigate_to(url)
            .map_err(|e| ChromeError::NavigationError(e.to_string()))?;

        // Wait for navigation to complete
        tab.wait_until_navigated()
            .map_err(|e| ChromeError::NavigationError(e.to_string()))?;

        // Wait for body element to ensure basic page structure is loaded
        tab.wait_for_element("body")
            .map_err(|e| ChromeError::NavigationError(format!("Timeout waiting for body: {e}")))?;

        // Small delay to allow JavaScript to settle
        std::thread::sleep(Duration::from_millis(500));

        Ok(())
    }

    /// Evaluate JavaScript in the page context
    pub fn evaluate_js(&self, tab: &Arc<Tab>, script: &str) -> Result<Option<String>, ChromeError> {
        let result = tab
            .evaluate(script, false)
            .map_err(|e| ChromeError::EvaluationError(e.to_string()))?;

        // Extract string value from result
        Ok(result.value.and_then(|v| {
            if v.is_string() {
                v.as_str().map(String::from)
            } else {
                Some(v.to_string())
            }
        }))
    }

    /// Check if connected to an existing Chrome instance
    pub fn is_connected_to_existing(&self) -> bool {
        self.config.ws_url.is_some()
    }

    /// Get the Chrome path being used (if known)
    pub fn chrome_path(&self) -> Option<&PathBuf> {
        self.config.chrome_path.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_default_config() {
        // Clear environment variables for clean test
        std::env::remove_var("CODELET_CHROME_WS_URL");
        std::env::remove_var("CODELET_CHROME_PATH");
        std::env::remove_var("CODELET_CHROME_HEADLESS");
        std::env::remove_var("CODELET_CHROME_TIMEOUT");

        let config = ChromeConfig::default();

        assert!(config.ws_url.is_none());
        assert!(config.chrome_path.is_none());
        assert!(config.headless);
        assert_eq!(config.timeout, Duration::from_secs(300)); // 5 minute default
        assert!(config.user_agent.is_some());
    }

    #[test]
    #[serial]
    fn test_config_from_env() {
        std::env::set_var("CODELET_CHROME_WS_URL", "ws://localhost:9222");
        std::env::set_var("CODELET_CHROME_PATH", "/usr/bin/chromium");
        std::env::set_var("CODELET_CHROME_HEADLESS", "false");
        std::env::set_var("CODELET_CHROME_TIMEOUT", "60");

        let config = ChromeConfig::default();

        assert_eq!(config.ws_url, Some("ws://localhost:9222".to_string()));
        assert_eq!(config.chrome_path, Some(PathBuf::from("/usr/bin/chromium")));
        assert!(!config.headless);
        assert_eq!(config.timeout, Duration::from_secs(60));

        // Clean up
        std::env::remove_var("CODELET_CHROME_WS_URL");
        std::env::remove_var("CODELET_CHROME_PATH");
        std::env::remove_var("CODELET_CHROME_HEADLESS");
        std::env::remove_var("CODELET_CHROME_TIMEOUT");
    }
}
