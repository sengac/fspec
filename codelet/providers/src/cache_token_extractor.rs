//! Cache Token Extractor for Anthropic SSE Responses
//!
//! This module extracts cache_read_input_tokens and cache_creation_input_tokens
//! from Anthropic's SSE message_start events, which are not exposed by rig's
//! streaming abstraction.

use serde::Deserialize;

/// Anthropic SSE MessageStart event structure
#[derive(Debug, Deserialize)]
pub struct MessageStartEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub message: Option<MessageStartPayload>,
}

/// Payload within a MessageStart event
#[derive(Debug, Deserialize)]
pub struct MessageStartPayload {
    pub usage: Option<AnthropicUsage>,
}

/// Anthropic usage structure with cache tokens
#[derive(Debug, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
}

/// Extracts cache tokens from SSE stream
///
/// This extractor processes SSE lines and captures cache token information
/// from message_start events.
#[derive(Debug, Default)]
pub struct CacheTokenExtractor {
    cache_read_tokens: Option<u64>,
    cache_creation_tokens: Option<u64>,
}

impl CacheTokenExtractor {
    /// Create a new cache token extractor
    pub fn new() -> Self {
        Self::default()
    }

    /// Process an SSE line and extract cache tokens if present
    ///
    /// SSE format: `data: {...json...}`
    pub fn process_sse_line(&mut self, line: &str) {
        // SSE data lines start with "data: "
        let json_str = match line.strip_prefix("data: ") {
            Some(s) => s.trim(),
            None => return,
        };

        // Skip empty or [DONE] markers
        if json_str.is_empty() || json_str == "[DONE]" {
            return;
        }

        // Try to parse as MessageStart event
        if let Ok(event) = serde_json::from_str::<MessageStartEvent>(json_str) {
            if event.event_type == "message_start" {
                if let Some(message) = event.message {
                    if let Some(usage) = message.usage {
                        self.cache_read_tokens = usage.cache_read_input_tokens;
                        self.cache_creation_tokens = usage.cache_creation_input_tokens;
                    }
                }
            }
        }
    }

    /// Get extracted cache read tokens
    pub fn cache_read_tokens(&self) -> Option<u64> {
        self.cache_read_tokens
    }

    /// Get extracted cache creation tokens
    pub fn cache_creation_tokens(&self) -> Option<u64> {
        self.cache_creation_tokens
    }

    /// Reset the extractor for a new stream
    pub fn reset(&mut self) {
        self.cache_read_tokens = None;
        self.cache_creation_tokens = None;
    }
}

/// Convenience function to extract cache tokens from a single SSE line
///
/// Returns (cache_read_input_tokens, cache_creation_input_tokens)
pub fn extract_cache_tokens_from_sse(sse_line: &str) -> (Option<u64>, Option<u64>) {
    let mut extractor = CacheTokenExtractor::new();
    extractor.process_sse_line(sse_line);
    (
        extractor.cache_read_tokens(),
        extractor.cache_creation_tokens(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_cache_read_tokens() {
        let sse_event = r#"data: {"type":"message_start","message":{"usage":{"input_tokens":10000,"cache_read_input_tokens":5000}}}"#;
        let (cache_read, _) = extract_cache_tokens_from_sse(sse_event);
        assert_eq!(cache_read, Some(5000));
    }

    #[test]
    fn test_extract_cache_creation_tokens() {
        let sse_event = r#"data: {"type":"message_start","message":{"usage":{"input_tokens":10000,"cache_creation_input_tokens":2000}}}"#;
        let (_, cache_creation) = extract_cache_tokens_from_sse(sse_event);
        assert_eq!(cache_creation, Some(2000));
    }

    #[test]
    fn test_non_message_start_ignored() {
        let sse_event =
            r#"data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"Hello"}}"#;
        let (cache_read, cache_creation) = extract_cache_tokens_from_sse(sse_event);
        assert!(cache_read.is_none());
        assert!(cache_creation.is_none());
    }

    #[test]
    fn test_missing_cache_tokens() {
        let sse_event =
            r#"data: {"type":"message_start","message":{"usage":{"input_tokens":10000}}}"#;
        let (cache_read, cache_creation) = extract_cache_tokens_from_sse(sse_event);
        assert!(cache_read.is_none());
        assert!(cache_creation.is_none());
    }
}
