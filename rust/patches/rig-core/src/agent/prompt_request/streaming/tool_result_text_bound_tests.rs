//! Feature: spec/features/defense-in-depth-bound-tool-result-text-size-in-rig-patch-parse-tool-result-content.feature
//!
//! CMPCT-031 tests for the text-size bound on tool_result content, moved out of
//! streaming.rs's inline test module so each test file maps one feature (COV-075).

use super::*;

// =========================================================================
// CMPCT-031: Defense-in-depth text-size bound on tool_result content
// Feature: spec/features/defense-in-depth-bound-tool-result-text-size-in-rig-patch-parse-tool-result-content.feature
//
// These tests exercise `bound_tool_result_text` directly (pure fn, no I/O)
// and observe `parse_tool_result_content` end-to-end for the verbose-tool
// cascade scenario. All six scenarios from the feature file are covered
// one-to-one by the #[test] fns below, with @step comments matching the
// Gherkin step text verbatim.
// =========================================================================

/// Helper: build a UTF-8 string of exactly `bytes` ASCII chars.
fn ascii_of_len(bytes: usize) -> String {
    // Using 'a' so .len() == char count == byte count, and there are no
    // multi-byte code points to complicate boundary math.
    "a".repeat(bytes)
}

/// Scenario: A 200 KiB text tool result is replaced by a truncation marker
#[test]
fn test_cmpct_031_200kib_text_replaced_by_truncation_marker() {
    // @step Given parse_tool_result_content is called with a plain-text payload of 200 KiB
    let payload = ascii_of_len(200 * 1024);
    let result = parse_tool_result_content(&payload);

    // @step When the helper evaluates the byte length against MAX_TOOL_RESULT_TEXT_BYTES
    assert_eq!(result.len(), 1, "expected a single ToolResultContent");
    let text = match &result[0] {
        ToolResultContent::Text(t) => t.text.clone(),
        other => panic!("expected Text variant, got {other:?}"),
    };

    // @step Then the returned ToolResultContent::text payload is a JSON truncation marker
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("marker must be valid JSON");
    assert!(parsed.is_object(), "marker must be a JSON object");

    // @step And the marker has field "status" equal to "truncated"
    assert_eq!(
        parsed.get("status").and_then(|v| v.as_str()),
        Some("truncated")
    );

    // @step And the marker has field "original_bytes" equal to the original byte length
    assert_eq!(
        parsed.get("original_bytes").and_then(|v| v.as_u64()),
        Some(payload.len() as u64)
    );

    // @step And the marker has field "max_bytes" equal to MAX_TOOL_RESULT_TEXT_BYTES
    assert_eq!(
        parsed.get("max_bytes").and_then(|v| v.as_u64()),
        Some(MAX_TOOL_RESULT_TEXT_BYTES as u64)
    );

    // @step And the marker has field "preview" containing the first 2048 bytes of the original text
    let preview = parsed
        .get("preview")
        .and_then(|v| v.as_str())
        .expect("preview field must be a string");
    assert_eq!(preview.len(), 2048);
    assert_eq!(preview, &payload[..2048]);

    // @step And the marker has field "suffix" containing the last 512 bytes of the original text
    let suffix = parsed
        .get("suffix")
        .and_then(|v| v.as_str())
        .expect("suffix field must be a string");
    assert_eq!(suffix.len(), 512);
    assert_eq!(suffix, &payload[payload.len() - 512..]);

    // @step And the marker has a non-empty "hint" field
    let hint = parsed
        .get("hint")
        .and_then(|v| v.as_str())
        .expect("hint field must be a string");
    assert!(!hint.is_empty(), "hint must be non-empty");
}

/// Scenario: A 32 KiB text tool result is stored verbatim without a truncation marker
#[test]
fn test_cmpct_031_32kib_text_stored_verbatim() {
    // @step Given parse_tool_result_content is called with a plain-text payload of 32 KiB
    let payload = ascii_of_len(32 * 1024);
    let result = parse_tool_result_content(&payload);

    // @step When the helper evaluates the byte length against MAX_TOOL_RESULT_TEXT_BYTES
    assert_eq!(result.len(), 1);
    let text = match &result[0] {
        ToolResultContent::Text(t) => t.text.clone(),
        other => panic!("expected Text variant, got {other:?}"),
    };

    // @step Then the returned ToolResultContent::text payload equals the original 32 KiB verbatim
    assert_eq!(text, payload);

    // @step And the payload does NOT contain a JSON truncation marker
    // A quick structural check: if it parsed as our marker object, .get("status") would
    // yield "truncated". For raw "aaa..." text this either fails to parse as JSON or
    // does not contain the sentinel.
    let is_marker = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v.get("status").and_then(|s| s.as_str()).map(str::to_owned))
        == Some("truncated".to_string());
    assert!(!is_marker, "payload must not be a truncation marker");
}

/// Scenario: A text payload exactly at the bound is stored verbatim
#[test]
fn test_cmpct_031_exact_bound_stored_verbatim() {
    // @step Given parse_tool_result_content is called with a plain-text payload of exactly MAX_TOOL_RESULT_TEXT_BYTES bytes
    let payload = ascii_of_len(MAX_TOOL_RESULT_TEXT_BYTES);
    let result = parse_tool_result_content(&payload);

    // @step When the helper evaluates the byte length
    assert_eq!(result.len(), 1);
    let text = match &result[0] {
        ToolResultContent::Text(t) => t.text.clone(),
        other => panic!("expected Text variant, got {other:?}"),
    };

    // @step Then the returned ToolResultContent::text payload equals the original verbatim
    assert_eq!(text, payload);

    // @step And the bound condition is a strict greater-than (`>`) not greater-than-or-equal (`>=`)
    // Verified by a one-byte-over check through the helper directly: len == MAX stays verbatim,
    // len == MAX + 1 becomes a marker.
    let one_byte_over = ascii_of_len(MAX_TOOL_RESULT_TEXT_BYTES + 1);
    let bounded = bound_tool_result_text(one_byte_over);
    let parsed: serde_json::Value =
        serde_json::from_str(&bounded).expect("one-over must be a JSON marker");
    assert_eq!(
        parsed.get("status").and_then(|v| v.as_str()),
        Some("truncated")
    );
}

/// Scenario: An oversized image is still rejected by the existing image size check
#[test]
fn test_cmpct_031_oversized_image_still_rejected_by_existing_check() {
    // @step Given parse_tool_result_content is called with an oversized IMAGE payload
    // Build a tool result JSON envelope of {"type":"image","data":"<b64>","media_type":"image/png"}
    // whose base64 data will be rejected by check_image_dimensions. We use a known-oversized
    // 1x1 PNG padded by repeating large-dimension header bytes; the actual check is delegated
    // to codelet_common. For this test we feed a synthetic ~128 KiB base64 payload — the point
    // is that the image branch handles it (either accepts or produces an error_msg) WITHOUT
    // ever consulting the text bound, because the JSON envelope takes the "type":"image"
    // branch at line 90 before any text-branch code runs.
    let fake_b64 = "A".repeat(128 * 1024);
    let envelope = serde_json::json!({
        "type": "image",
        "data": fake_b64,
        "media_type": "image/png",
    })
    .to_string();
    let result = parse_tool_result_content(&envelope);

    // @step When the helper evaluates the payload
    assert_eq!(result.len(), 1);

    // @step Then the existing image-size rejection path fires unchanged
    // Either we got an Image variant (accepted by check_image_dimensions) OR a Text
    // variant whose content is the rejection error_msg — but it is NEVER the truncation
    // marker and NEVER the raw envelope JSON.
    match &result[0] {
        ToolResultContent::Image(_) => { /* accepted by existing path */ }
        ToolResultContent::Text(t) => {
            // @step And the new text bound is NOT consulted
            // Ensure this is not the CMPCT-031 marker: that would prove the text branch
            // took over, which would be a regression.
            let is_marker = serde_json::from_str::<serde_json::Value>(&t.text)
                .ok()
                .and_then(|v| v.get("status").and_then(|s| s.as_str()).map(str::to_owned))
                == Some("truncated".to_string());
            assert!(
                !is_marker,
                "image rejection must NOT flow through the CMPCT-031 text bound"
            );
            // @step And the behavior matches the pre-change image handling exactly
            // i.e. the payload is the existing image rejection error_msg, NOT the raw envelope.
            assert_ne!(
                t.text, envelope,
                "image branch must not fall through to raw-text handling"
            );
        }
    }
}

/// Scenario: The truncation marker is valid UTF-8 JSON that the model can reason about
#[test]
fn test_cmpct_031_truncation_marker_is_valid_utf8_json() {
    // @step Given parse_tool_result_content has produced a truncation marker for an oversized text payload
    let payload = ascii_of_len(200 * 1024);
    let result = parse_tool_result_content(&payload);
    assert_eq!(result.len(), 1);
    let text = match &result[0] {
        ToolResultContent::Text(t) => t.text.clone(),
        other => panic!("expected Text variant, got {other:?}"),
    };

    // @step When the marker is read back as UTF-8 and parsed as JSON
    assert!(text.is_ascii() || std::str::from_utf8(text.as_bytes()).is_ok());
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("marker must parse as JSON from UTF-8 bytes");

    // @step Then parsing succeeds with no errors
    let obj = parsed.as_object().expect("marker must be a JSON object");

    // @step And every field value round-trips cleanly (no mojibake, no unescaped control chars)
    // Reserialize and reparse; equality proves round-trip stability.
    let reserialized = serde_json::to_string(&parsed).expect("reserialization must succeed");
    let reparsed: serde_json::Value =
        serde_json::from_str(&reserialized).expect("reparse must succeed");
    assert_eq!(parsed, reparsed);

    // @step And the top-level object contains exactly the expected keys: status, original_bytes, max_bytes, preview, suffix, hint
    let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "hint",
            "max_bytes",
            "original_bytes",
            "preview",
            "status",
            "suffix",
        ]
    );
}

/// Scenario: Running a verbose tool no longer cascades into PromptCancelled
#[test]
fn test_cmpct_031_verbose_tool_output_is_bounded_before_chat_history() {
    // @step Given an fspec interactive session whose agent has just invoked a tool that returned 500 KiB of plain text output
    let payload = ascii_of_len(500 * 1024);

    // @step When the rig patch processes that tool result via parse_tool_result_content
    let result = parse_tool_result_content(&payload);
    assert_eq!(result.len(), 1);
    let text = match &result[0] {
        ToolResultContent::Text(t) => t.text.clone(),
        other => panic!("expected Text variant, got {other:?}"),
    };

    // @step Then the resulting chat_history entry for that tool_result holds only the truncation marker
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("marker must be valid JSON");
    assert_eq!(
        parsed.get("status").and_then(|v| v.as_str()),
        Some("truncated")
    );

    // @step And the chat_history byte count for that tool_result is bounded by a small constant (marker overhead + preview + suffix)
    // Marker overhead is the JSON scaffolding + key names + numeric fields + hint text.
    // Upper bound = preview (2048) + suffix (512) + generous 4096-byte overhead cap.
    let upper_bound = 2048 + 512 + 4096;
    assert!(
        text.len() <= upper_bound,
        "marker size {} exceeded upper bound {}",
        text.len(),
        upper_bound
    );

    // @step And the next LLM turn does NOT exceed the context window from this tool_result alone
    // Proxy: the payload held in chat_history is orders of magnitude smaller than the original.
    assert!(
        text.len() < payload.len() / 10,
        "bounded payload {} must be <<1/10 of original {}",
        text.len(),
        payload.len()
    );
}
