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
    },
    FindInPage {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pattern: Option<String>,
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
