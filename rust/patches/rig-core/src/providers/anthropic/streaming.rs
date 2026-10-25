use super::completion::{Content, Usage};
use crate::completion::GetTokenUsage;
use serde::{Deserialize, Serialize};

#[path = "streaming/events.rs"]
mod events;
#[path = "streaming/transport.rs"]
mod transport;
#[cfg(test)]
use events::handle_event;
#[cfg(test)]
#[path = "streaming/stop_tests.rs"]
mod stop_tests;
#[cfg(test)]
#[path = "streaming/thinking_tests.rs"]
mod thinking_tests;
#[cfg(test)]
#[path = "streaming/tool_tests.rs"]
mod tool_tests;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamingEvent {
    MessageStart {
        message: MessageStart,
    },
    ContentBlockStart {
        index: usize,
        content_block: Content,
    },
    ContentBlockDelta {
        index: usize,
        delta: ContentDelta,
    },
    ContentBlockStop {
        index: usize,
    },
    MessageDelta {
        delta: MessageDelta,
        usage: PartialUsage,
    },
    MessageStop,
    Ping,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct MessageStart {
    pub id: String,
    pub role: String,
    pub content: Vec<Content>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentDelta {
    TextDelta {
        text: String,
    },
    InputJsonDelta {
        partial_json: String,
    },
    ThinkingDelta {
        thinking: String,
    },
    SignatureDelta {
        signature: String,
    },
    /// Server-side compaction delta (Opus 4.6+)
    CompactionDelta {
        content: String,
    },
    /// Catch-all for unknown delta types
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct MessageDelta {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize, Default)]
pub struct PartialUsage {
    pub output_tokens: usize,
    #[serde(default)]
    pub input_tokens: Option<usize>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<usize>,
    #[serde(default)]
    pub cache_creation_input_tokens: Option<usize>,
}

impl GetTokenUsage for PartialUsage {
    fn token_usage(&self) -> Option<crate::completion::Usage> {
        let mut usage = crate::completion::Usage::new();
        usage.input_tokens = self.input_tokens? as u64;
        usage.output_tokens = self.output_tokens as u64;
        usage.total_tokens = usage
            .input_tokens
            .checked_add(usage.output_tokens)?
            .checked_add(self.cache_read_input_tokens.unwrap_or(0) as u64)?
            .checked_add(self.cache_creation_input_tokens.unwrap_or(0) as u64)?;
        usage.cache_read_input_tokens = self.cache_read_input_tokens.map(|tokens| tokens as u64);
        usage.cache_creation_input_tokens =
            self.cache_creation_input_tokens.map(|tokens| tokens as u64);
        Some(usage)
    }
}

#[derive(Default)]
struct ToolCallState {
    name: String,
    id: String,
    input_json: String,
}

#[derive(Default)]
struct ThinkingState {
    thinking: String,
    signature: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StreamingCompletionResponse {
    pub usage: PartialUsage,
    /// PROV-039: stop_reason from the MessageDelta event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
}

impl GetTokenUsage for StreamingCompletionResponse {
    fn token_usage(&self) -> Option<crate::completion::Usage> {
        self.usage.token_usage()
    }

    /// PROV-039: Return the captured stop_reason from the MessageDelta event
    fn stop_reason(&self) -> Option<&str> {
        self.stop_reason.as_deref()
    }
}
