// Feature: spec/features/structurally-lossless-trimmer.feature
//
// This test file validates the acceptance criteria for the Layer 0
// Structurally Lossless Trimmer Module. Tests map 1:1 to Gherkin scenarios.

use super::trimmer::Trimmer;
use serde_json::{json, Value};
use std::collections::HashMap;

// ============================================================================
// Test helpers — construct metadata in the real MessageEnvelope format
// ============================================================================

/// Build metadata for an assistant message containing a ToolUse block.
fn make_assistant_tool_use_metadata(
    tool_name: &str,
    tool_id: &str,
    input: Value,
) -> HashMap<String, Value> {
    let mut meta = HashMap::new();
    meta.insert("type".to_string(), json!("assistant"));
    meta.insert(
        "message".to_string(),
        json!({
            "role": "assistant",
            "content": [
                {
                    "type": "tool_use",
                    "id": tool_id,
                    "name": tool_name,
                    "input": input
                }
            ]
        }),
    );
    meta
}

/// Build metadata for a user message containing a ToolResult block.
fn make_user_tool_result_metadata(
    tool_use_id: &str,
    is_error: bool,
) -> HashMap<String, Value> {
    let mut meta = HashMap::new();
    meta.insert("type".to_string(), json!("user"));
    meta.insert(
        "message".to_string(),
        json!({
            "role": "user",
            "content": [
                {
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": "[content in main field]",
                    "is_error": is_error
                }
            ]
        }),
    );
    meta
}

/// Build metadata for a user message containing a ToolResult with extended metadata.
fn make_user_tool_result_metadata_with_exit_code(
    tool_use_id: &str,
    is_error: bool,
    is_image: bool,
) -> HashMap<String, Value> {
    let mut meta = HashMap::new();
    meta.insert("type".to_string(), json!("user"));
    meta.insert(
        "message".to_string(),
        json!({
            "role": "user",
            "content": [
                {
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": "[content in main field]",
                    "is_error": is_error,
                    "tool_use_result": {
                        "stdout": "",
                        "stderr": "",
                        "interrupted": false,
                        "isImage": is_image
                    }
                }
            ]
        }),
    );
    meta
}

/// Build metadata for a plain user text message (no tool result).
fn make_plain_user_metadata() -> HashMap<String, Value> {
    let mut meta = HashMap::new();
    meta.insert("type".to_string(), json!("user"));
    meta.insert(
        "message".to_string(),
        json!({
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": "plain user message"
                }
            ]
        }),
    );
    meta
}

/// Build metadata for a plain assistant text message (no tool use).
fn make_plain_assistant_metadata() -> HashMap<String, Value> {
    let mut meta = HashMap::new();
    meta.insert("type".to_string(), json!("assistant"));
    meta.insert(
        "message".to_string(),
        json!({
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "assistant reasoning"
                }
            ]
        }),
    );
    meta
}

/// Generate N lines of content for testing.
fn generate_lines(n: usize) -> String {
    (1..=n)
        .map(|i| format!("     {i}: line {i} content here with some text"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Generate N lines of grep-style output.
fn generate_grep_matches(n: usize) -> String {
    (1..=n)
        .map(|i| format!("/src/file{}.rs:{}:matching line content {}", i % 5, i * 10, i))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Minimal base64 encoder for test helpers (no external crate needed).
fn simple_base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

// ============================================================================
// Scenario: Trim Read tool result to compact file reference
// ============================================================================

#[test]
fn test_trim_read_tool_result_to_compact_file_reference() {
    // @step Given a StoredMessage with role "user" containing 500 lines of file content
    let content = generate_lines(500);
    let tool_id = "toolu_read_001";

    // @step And the message metadata indicates a Read tool result for path "src/main.rs"
    // First register the Read tool use in an assistant message
    let assistant_meta = make_assistant_tool_use_metadata(
        "Read",
        tool_id,
        json!({"file_path": "src/main.rs"}),
    );

    let user_meta = make_user_tool_result_metadata(tool_id, false);

    let mut trimmer = Trimmer::new();

    // Process assistant message first to register tool_use_id -> tool_name
    let _ = trimmer.trim_message("assistant", "[tool_use: Read]", &assistant_meta);

    // @step When the Trimmer processes the message
    let trimmed = trimmer.trim_message("user", &content, &user_meta);

    // @step Then the content should be replaced with a compact reference "[file: src/main.rs, 500 lines, {tokens} tok — use Read to retrieve]"
    assert!(
        trimmed.starts_with("[file: src/main.rs, 500 lines,"),
        "Expected compact file reference, got: {}",
        &trimmed[..trimmed.len().min(120)]
    );
    assert!(
        trimmed.contains("tok — use Read to retrieve]"),
        "Expected 'tok — use Read to retrieve]' in: {trimmed}"
    );

    // @step And the trimmed output should be significantly smaller than the original
    assert!(
        trimmed.len() < content.len() / 5,
        "Trimmed ({}) should be much smaller than original ({})",
        trimmed.len(),
        content.len()
    );
}

// ============================================================================
// Scenario: Trim Write tool parameters to persistence reference
// ============================================================================

#[test]
fn test_trim_write_tool_parameters_to_persistence_reference() {
    // @step Given a StoredMessage with role "assistant" containing a Write tool use
    let file_content = generate_lines(200);
    let tool_id = "toolu_write_001";

    // @step And the Write tool input includes full file content of 200 lines for path "src/auth.rs"
    let meta = make_assistant_tool_use_metadata(
        "Write",
        tool_id,
        json!({
            "file_path": "src/auth.rs",
            "content": file_content
        }),
    );

    let mut trimmer = Trimmer::new();

    // @step When the Trimmer processes the message
    // In production, assistant tool_use content is just the summary "[tool_use: Write]"
    let trimmed = trimmer.trim_message("assistant", "[tool_use: Write]", &meta);

    // @step Then the file content should be replaced with "[Write: src/auth.rs, 200 lines — file persisted to disk]"
    assert_eq!(
        trimmed,
        "[Write: src/auth.rs, 200 lines — file persisted to disk]",
        "Expected Write compact reference, got: {trimmed}"
    );
}

// ============================================================================
// Scenario: Condense Edit tool parameters to change summary
// ============================================================================

#[test]
fn test_condense_edit_tool_parameters_to_change_summary() {
    // @step Given a StoredMessage with role "assistant" containing an Edit tool use
    let tool_id = "toolu_edit_001";
    let old_string = "a".repeat(50);
    let new_string = "b".repeat(80);

    // @step And the Edit tool input has a 50-character old_string and 80-character new_string for path "src/auth.rs"
    let meta = make_assistant_tool_use_metadata(
        "Edit",
        tool_id,
        json!({
            "file_path": "src/auth.rs",
            "old_string": old_string,
            "new_string": new_string
        }),
    );

    let mut trimmer = Trimmer::new();

    // @step When the Trimmer processes the message
    // In production, assistant tool_use content is just the summary "[tool_use: Edit]"
    let trimmed = trimmer.trim_message("assistant", "[tool_use: Edit]", &meta);

    // @step Then the edit parameters should be condensed to "[Edit: src/auth.rs — replaced 50 chars with 80 chars]"
    assert_eq!(
        trimmed,
        "[Edit: src/auth.rs — replaced 50 chars with 80 chars]",
        "Expected Edit condensed reference, got: {trimmed}"
    );
}

// ============================================================================
// Scenario: Truncate long Bash output with head and tail
// ============================================================================

#[test]
fn test_truncate_long_bash_output_with_head_and_tail() {
    // @step Given a StoredMessage with role "user" containing Bash tool output of 200 lines
    let lines: Vec<String> = (1..=200)
        .map(|i| format!("output line {i}"))
        .collect();
    let content = lines.join("\n");
    let tool_id = "toolu_bash_001";

    // @step And the message metadata indicates a Bash tool result with exit code 0
    let assistant_meta = make_assistant_tool_use_metadata(
        "Bash",
        tool_id,
        json!({"command": "npm test"}),
    );

    let user_meta = make_user_tool_result_metadata(tool_id, false);

    let mut trimmer = Trimmer::new();
    let _ = trimmer.trim_message("assistant", "[tool_use: Bash]", &assistant_meta);

    // @step When the Trimmer processes the message
    let trimmed = trimmer.trim_message("user", &content, &user_meta);

    // @step Then the output should contain the first 10 lines of the original output
    for i in 1..=10 {
        assert!(
            trimmed.contains(&format!("output line {i}")),
            "Expected first 10 lines, missing line {i}"
        );
    }

    // @step And the output should contain "... (185 lines omitted)"
    assert!(
        trimmed.contains("... (185 lines omitted)"),
        "Expected '... (185 lines omitted)' in: {trimmed}"
    );

    // @step And the output should contain the last 5 lines of the original output
    for i in 196..=200 {
        assert!(
            trimmed.contains(&format!("output line {i}")),
            "Expected last 5 lines, missing line {i}"
        );
    }

    // @step And the output should include the exit code
    assert!(
        trimmed.contains("[exit code: 0]"),
        "Expected exit code in output, got: {trimmed}"
    );
}

// ============================================================================
// Scenario: Short Bash output passes through unchanged
// ============================================================================

#[test]
fn test_short_bash_output_passes_through_unchanged() {
    // @step Given a StoredMessage with role "user" containing Bash tool output of 14 lines
    let lines: Vec<String> = (1..=14)
        .map(|i| format!("short output line {i}"))
        .collect();
    let content = lines.join("\n");
    let tool_id = "toolu_bash_002";

    // @step And the message metadata indicates a Bash tool result
    let assistant_meta = make_assistant_tool_use_metadata(
        "Bash",
        tool_id,
        json!({"command": "echo hello"}),
    );
    let user_meta = make_user_tool_result_metadata(tool_id, false);

    let mut trimmer = Trimmer::new();
    let _ = trimmer.trim_message("assistant", "[tool_use: Bash]", &assistant_meta);

    // @step When the Trimmer processes the message
    let trimmed = trimmer.trim_message("user", &content, &user_meta);

    // @step Then the content should pass through completely unchanged
    assert_eq!(
        trimmed, content,
        "Short Bash output should pass through unchanged"
    );
}

// ============================================================================
// Scenario: Strip base64 image data to metadata placeholder
// ============================================================================

#[test]
fn test_strip_base64_image_data_to_metadata_placeholder() {
    // @step Given a StoredMessage containing base64-encoded image data
    // Construct a minimal PNG header with 800x600 dimensions:
    // PNG signature (8 bytes) + IHDR chunk: length(4) + "IHDR"(4) + width(4) + height(4) = 24 bytes
    let png_header: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR length
        0x49, 0x48, 0x44, 0x52, // "IHDR"
        0x00, 0x00, 0x03, 0x20, // width = 800
        0x00, 0x00, 0x02, 0x58, // height = 600
    ];
    let b64_header = simple_base64_encode(&png_header);
    let base64_data = format!("{}{}",  b64_header, "A".repeat(60000));
    let content = format!("data:image/png;base64,{base64_data}");
    let tool_id = "toolu_screenshot_001";

    // @step And the image metadata indicates dimensions 800x600 and source path "screenshot.png"
    let assistant_meta = make_assistant_tool_use_metadata(
        "Read",
        tool_id,
        json!({"file_path": "screenshot.png"}),
    );
    let user_meta = make_user_tool_result_metadata_with_exit_code(tool_id, false, true);

    let mut trimmer = Trimmer::new();
    let _ = trimmer.trim_message("assistant", "[tool_use: Read]", &assistant_meta);

    // @step When the Trimmer processes the message
    let trimmed = trimmer.trim_message("user", &content, &user_meta);

    // @step Then the base64 data should be replaced with "[image: 800x600, {bytes} bytes, from screenshot.png]"
    assert!(
        trimmed.starts_with("[image: 800x600,"),
        "Expected image placeholder with 800x600 dimensions, got: {}",
        &trimmed[..trimmed.len().min(100)]
    );
    assert!(
        trimmed.contains("from screenshot.png]"),
        "Expected source path in placeholder, got: {trimmed}"
    );
    // Base64 data should be completely stripped
    assert!(
        !trimmed.contains("iVBOR"),
        "Base64 data should be stripped"
    );
    assert!(
        trimmed.len() < 200,
        "Trimmed image should be a short placeholder, got {} chars",
        trimmed.len()
    );
}

// ============================================================================
// Scenario: Truncate Search/Grep output to first matches
// ============================================================================

#[test]
fn test_truncate_search_grep_output_to_first_matches() {
    // @step Given a StoredMessage with role "user" containing Grep tool output with 50 matching lines
    let content = generate_grep_matches(50);
    let tool_id = "toolu_grep_001";

    // @step And the message metadata indicates a Grep tool result
    let assistant_meta = make_assistant_tool_use_metadata(
        "Grep",
        tool_id,
        json!({"pattern": "matching", "path": "src/"}),
    );
    let user_meta = make_user_tool_result_metadata(tool_id, false);

    let mut trimmer = Trimmer::new();
    let _ = trimmer.trim_message("assistant", "[tool_use: Grep]", &assistant_meta);

    // @step When the Trimmer processes the message
    let trimmed = trimmer.trim_message("user", &content, &user_meta);

    // @step Then the output should contain the first 10 matches
    let trimmed_lines: Vec<&str> = trimmed.lines().collect();
    // Should have 10 match lines + the "more matches" line
    assert!(
        trimmed_lines.len() <= 12,
        "Expected ~11 lines (10 matches + summary), got {}",
        trimmed_lines.len()
    );

    // @step And the output should end with "... (40 more matches)"
    assert!(
        trimmed.contains("... (40 more matches)"),
        "Expected '... (40 more matches)' in: {trimmed}"
    );
}

// ============================================================================
// Scenario: User messages pass through unchanged
// ============================================================================

#[test]
fn test_user_messages_pass_through_unchanged() {
    // @step Given a StoredMessage with role "user" and content "please fix the login bug"
    let content = "please fix the login bug";

    // @step And the message has no tool metadata
    let meta = make_plain_user_metadata();

    let mut trimmer = Trimmer::new();

    // @step When the Trimmer processes the message
    let trimmed = trimmer.trim_message("user", content, &meta);

    // @step Then the content should be exactly "please fix the login bug"
    assert_eq!(trimmed, "please fix the login bug");

    // @step And zero transformation should have been applied
    assert_eq!(trimmed.len(), content.len());
}

// ============================================================================
// Scenario: Assistant reasoning text passes through unchanged
// ============================================================================

#[test]
fn test_assistant_reasoning_text_passes_through_unchanged() {
    // @step Given a StoredMessage with role "assistant" containing reasoning text
    // @step And the content is "I need to check the error handling in the auth module"
    let content = "I need to check the error handling in the auth module";

    // @step And the message has no tool use metadata
    let meta = make_plain_assistant_metadata();

    let mut trimmer = Trimmer::new();

    // @step When the Trimmer processes the message
    let trimmed = trimmer.trim_message("assistant", content, &meta);

    // @step Then the content should be exactly "I need to check the error handling in the auth module"
    assert_eq!(
        trimmed,
        "I need to check the error handling in the auth module"
    );
}

// ============================================================================
// Scenario: Messages without tool metadata pass through unchanged
// ============================================================================

#[test]
fn test_messages_without_tool_metadata_pass_through_unchanged() {
    // @step Given a StoredMessage with role "user" and arbitrary content
    let content = "This is just a regular user message with some text in it.";

    // @step And the message metadata HashMap is empty
    let meta: HashMap<String, Value> = HashMap::new();

    let mut trimmer = Trimmer::new();

    // @step When the Trimmer processes the message
    let trimmed = trimmer.trim_message("user", content, &meta);

    // @step Then the content should be returned unchanged
    assert_eq!(trimmed, content);
}

// ============================================================================
// Scenario: Trimming is deterministic
// ============================================================================

#[test]
fn test_trimming_is_deterministic() {
    // @step Given a StoredMessage with role "user" containing Bash tool output of 200 lines
    let lines: Vec<String> = (1..=200)
        .map(|i| format!("deterministic line {i}"))
        .collect();
    let content = lines.join("\n");
    let tool_id = "toolu_bash_det";

    let assistant_meta = make_assistant_tool_use_metadata(
        "Bash",
        tool_id,
        json!({"command": "test"}),
    );
    let user_meta = make_user_tool_result_metadata(tool_id, false);

    // @step When the Trimmer processes the same message twice
    let mut trimmer1 = Trimmer::new();
    let _ = trimmer1.trim_message("assistant", "[tool_use: Bash]", &assistant_meta);
    let result1 = trimmer1.trim_message("user", &content, &user_meta);

    let mut trimmer2 = Trimmer::new();
    let _ = trimmer2.trim_message("assistant", "[tool_use: Bash]", &assistant_meta);
    let result2 = trimmer2.trim_message("user", &content, &user_meta);

    // @step Then both outputs should be identical byte-for-byte
    assert_eq!(
        result1, result2,
        "Trimming must be deterministic — same input, same output"
    );
}

// ============================================================================
// Scenario: Trimmed output is measurably smaller than input
// ============================================================================

#[test]
fn test_trimmed_output_is_measurably_smaller_than_input() {
    // @step Given a StoredMessage with role "user" containing Read tool output of 1000 lines
    let content = generate_lines(1000);
    let tool_id = "toolu_read_big";

    let assistant_meta = make_assistant_tool_use_metadata(
        "Read",
        tool_id,
        json!({"file_path": "src/large_file.rs"}),
    );
    let user_meta = make_user_tool_result_metadata(tool_id, false);

    let mut trimmer = Trimmer::new();
    let _ = trimmer.trim_message("assistant", "[tool_use: Read]", &assistant_meta);

    // @step When the Trimmer processes the message
    let trimmed = trimmer.trim_message("user", &content, &user_meta);

    // @step Then the character count of the trimmed output should be less than 10% of the original
    let ratio = (trimmed.len() as f64) / (content.len() as f64);
    assert!(
        ratio < 0.10,
        "Trimmed output ({} chars) should be <10% of original ({} chars), ratio: {:.4}",
        trimmed.len(),
        content.len(),
        ratio
    );
}
