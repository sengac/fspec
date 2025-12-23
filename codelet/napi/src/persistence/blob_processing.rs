//! Pure Rust blob processing functions for message envelopes.
//!
//! This module contains blob storage and rehydration logic that can be tested
//! without NAPI dependencies. The napi_bindings module wraps these functions
//! to expose them to JavaScript.

use super::{
    get_blob, should_use_blob_storage, store_blob, AssistantContent, DocumentSource, ImageSource,
    MessageEnvelope, MessagePayload, UserContent,
};

/// Blob reference prefix for content stored in blob storage
pub const BLOB_REF_PREFIX: &str = "blob:sha256:";

/// Check if a string is a blob reference
pub fn is_blob_reference(s: &str) -> bool {
    s.starts_with(BLOB_REF_PREFIX) && s.len() == BLOB_REF_PREFIX.len() + 64
}

/// Extract hash from a blob reference string
pub fn extract_blob_hash(s: &str) -> Option<&str> {
    if is_blob_reference(s) {
        Some(&s[BLOB_REF_PREFIX.len()..])
    } else {
        None
    }
}

/// Create a blob reference string from a hash
pub fn make_blob_reference(hash: &str) -> String {
    format!("{}{}", BLOB_REF_PREFIX, hash)
}

/// Check if content should be stored in blob storage and store it if needed.
/// Returns the blob hash if stored, None if content is small enough for inline storage.
fn maybe_store_blob(content: &str) -> Result<Option<String>, String> {
    let bytes = content.as_bytes();
    if should_use_blob_storage(bytes) {
        let hash = store_blob(bytes)?;
        Ok(Some(hash))
    } else {
        Ok(None)
    }
}

/// Process an envelope and extract large content to blob storage.
/// Large content is REPLACED with blob references (blob:sha256:<hash>).
///
/// Returns the processed envelope and a list of (key, hash) pairs for the blob references.
pub fn process_envelope_for_blob_storage(
    envelope: &MessageEnvelope,
) -> Result<(MessageEnvelope, Vec<(String, String)>), String> {
    let mut processed = envelope.clone();
    let mut blob_refs: Vec<(String, String)> = Vec::new();

    match &mut processed.message {
        MessagePayload::User(user_msg) => {
            for (idx, content) in user_msg.content.iter_mut().enumerate() {
                match content {
                    UserContent::ToolResult {
                        content: result_content,
                        tool_use_id,
                        ..
                    } => {
                        if let Some(hash) = maybe_store_blob(result_content)? {
                            blob_refs.push((
                                format!("tool_result:{}:{}", idx, tool_use_id),
                                hash.clone(),
                            ));
                            // REPLACE content with blob reference
                            *result_content = make_blob_reference(&hash);
                        }
                    }
                    UserContent::Image {
                        source: ImageSource::Base64 { data, .. },
                    } => {
                        if let Some(hash) = maybe_store_blob(data)? {
                            blob_refs.push((format!("image:{}", idx), hash.clone()));
                            // REPLACE data with blob reference
                            *data = make_blob_reference(&hash);
                        }
                    }
                    UserContent::Document {
                        source: DocumentSource::Base64 { data, .. },
                        ..
                    } => {
                        if let Some(hash) = maybe_store_blob(data)? {
                            blob_refs.push((format!("document:{}", idx), hash.clone()));
                            // REPLACE data with blob reference
                            *data = make_blob_reference(&hash);
                        }
                    }
                    _ => {}
                }
            }
        }
        MessagePayload::Assistant(assistant_msg) => {
            for (idx, content) in assistant_msg.content.iter_mut().enumerate() {
                match content {
                    AssistantContent::Thinking { thinking, .. } => {
                        if let Some(hash) = maybe_store_blob(thinking)? {
                            blob_refs.push((format!("thinking:{}", idx), hash.clone()));
                            // REPLACE thinking content with blob reference
                            *thinking = make_blob_reference(&hash);
                        }
                    }
                    AssistantContent::ToolUse { id, input, .. } => {
                        // ToolUse input can contain large content (e.g., file contents being written)
                        // Serialize the input and check if it should be blobified
                        let input_str = serde_json::to_string(input)
                            .map_err(|e| format!("Failed to serialize tool input: {}", e))?;
                        if let Some(hash) = maybe_store_blob(&input_str)? {
                            blob_refs.push((format!("tool_use:{}:{}", idx, id), hash.clone()));
                            // REPLACE input with a marker containing the blob reference
                            *input = serde_json::json!({
                                "_blob_ref": make_blob_reference(&hash)
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok((processed, blob_refs))
}

/// Rehydrate blob references in an envelope by fetching content from blob storage.
/// This restores the original content that was replaced with blob references.
pub fn rehydrate_envelope_blobs(envelope_json: &str) -> Result<String, String> {
    // Parse the envelope JSON
    let mut envelope: MessageEnvelope = serde_json::from_str(envelope_json)
        .map_err(|e| format!("Failed to parse envelope for rehydration: {}", e))?;

    match &mut envelope.message {
        MessagePayload::User(user_msg) => {
            for content in &mut user_msg.content {
                match content {
                    UserContent::ToolResult {
                        content: result_content,
                        ..
                    } => {
                        if let Some(hash) = extract_blob_hash(result_content) {
                            let blob_data = get_blob(hash)?;
                            *result_content = String::from_utf8_lossy(&blob_data).to_string();
                        }
                    }
                    UserContent::Image {
                        source: ImageSource::Base64 { data, .. },
                    } => {
                        if let Some(hash) = extract_blob_hash(data) {
                            let blob_data = get_blob(hash)?;
                            *data = String::from_utf8_lossy(&blob_data).to_string();
                        }
                    }
                    UserContent::Document {
                        source: DocumentSource::Base64 { data, .. },
                        ..
                    } => {
                        if let Some(hash) = extract_blob_hash(data) {
                            let blob_data = get_blob(hash)?;
                            *data = String::from_utf8_lossy(&blob_data).to_string();
                        }
                    }
                    _ => {}
                }
            }
        }
        MessagePayload::Assistant(assistant_msg) => {
            for content in &mut assistant_msg.content {
                match content {
                    AssistantContent::Thinking { thinking, .. } => {
                        if let Some(hash) = extract_blob_hash(thinking) {
                            let blob_data = get_blob(hash)?;
                            *thinking = String::from_utf8_lossy(&blob_data).to_string();
                        }
                    }
                    AssistantContent::ToolUse { input, .. } => {
                        // Check if input contains a blob reference marker
                        if let Some(blob_ref) = input.get("_blob_ref").and_then(|v| v.as_str()) {
                            if let Some(hash) = extract_blob_hash(blob_ref) {
                                let blob_data = get_blob(hash)?;
                                let input_str = String::from_utf8_lossy(&blob_data);
                                // Parse the original input JSON and restore it
                                *input = serde_json::from_str(&input_str).map_err(|e| {
                                    format!("Failed to parse rehydrated tool input: {}", e)
                                })?;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Serialize back to JSON
    serde_json::to_string(&envelope)
        .map_err(|e| format!("Failed to serialize rehydrated envelope: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_reference_format() {
        // Valid reference
        let valid_hash = "a".repeat(64);
        let valid_ref = make_blob_reference(&valid_hash);
        assert!(is_blob_reference(&valid_ref));
        assert_eq!(extract_blob_hash(&valid_ref), Some(valid_hash.as_str()));

        // Invalid: too short
        assert!(!is_blob_reference("blob:sha256:abc"));

        // Invalid: wrong prefix
        assert!(!is_blob_reference(&format!("blob:md5:{}", "a".repeat(64))));

        // Invalid: too long
        assert!(!is_blob_reference(&format!(
            "blob:sha256:{}",
            "a".repeat(65)
        )));
    }
}
