// Copied from Codex /tmp/codex/codex-rs/protocol/src/models.rs line 258-282
// and protocol/src/protocol.rs lines 1102-1111
// Using astgrep research to copy exact implementation

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebSearchAction {
    Search {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        query: Option<String>,
    },
    OpenPage {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        #[serde(default = "default_headless", skip_serializing_if = "is_default_headless")]
        headless: bool,
        /// When true, pause after page load for user interaction before returning.
        /// Also implies headless: false (pausing a headless browser is pointless).
        #[serde(default, skip_serializing_if = "is_false")]
        pause: bool,
    },
    FindInPage {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pattern: Option<String>,
        #[serde(default = "default_headless", skip_serializing_if = "is_default_headless")]
        headless: bool,
        /// When true, pause after page load for user interaction before returning.
        /// Also implies headless: false (pausing a headless browser is pointless).
        #[serde(default, skip_serializing_if = "is_false")]
        pause: bool,
    },
    CaptureScreenshot {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        full_page: Option<bool>,
        #[serde(default = "default_headless", skip_serializing_if = "is_default_headless")]
        headless: bool,
        /// When true, pause after page load for user interaction before returning.
        /// Also implies headless: false (pausing a headless browser is pointless).
        #[serde(default, skip_serializing_if = "is_false")]
        pause: bool,
    },

    #[serde(other)]
    Other,
}

// Copied from Codex protocol events
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebSearchBeginEvent {
    pub call_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebSearchEndEvent {
    pub call_id: String,
    pub query: String,
}

/// Default value for headless parameter (true)
fn default_headless() -> bool {
    true
}

/// Check if headless is the default value (for serialization skipping)
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_default_headless(headless: &bool) -> bool {
    *headless
}

/// Check if bool is false (for serialization skipping)
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(b: &bool) -> bool {
    !*b
}
