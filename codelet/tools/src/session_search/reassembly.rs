//! Streaming chunk reassembly — port of Python logic from session-search.sh
//!
//! Feature: spec/features/session-search.feature
//!
//! The persistence layer (session_manager.rs) stores assistant messages as
//! joined streaming chunks:
//!   - `[Thinking: <first 50 chars>...]` — thinking/reasoning fragments
//!   - `[Tool: <tool_name>]` — tool invocations
//!   - `[tool_result: ...]` — tool results (skipped in output)
//!   - Raw text fragments — response text split mid-word by SSE streaming
//!
//! This module reassembles those chunks into readable (type, content) sections.

/// A reassembled section of an assistant message
#[derive(Debug, Clone, PartialEq)]
pub enum Section {
    /// Reassembled thinking content
    Thinking(String),
    /// Tool invocation marker
    Tool(String),
    /// Reassembled response text
    Text(String),
}

/// Internal token types from line tokenization
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Thinking(String),
    Tool(String),
    ToolResult,
    Text(String),
}

/// Tokenize a single line, extracting `[Thinking: ...]`, `[Tool: ...]`,
/// `[tool_result: ...]`, and plain text tokens.
///
/// Handles both closed `[Thinking: text...]` and open `[Thinking: text`
/// forms (the persistence layer sometimes truncates without closing bracket).
fn tokenize_line(line: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut pos = 0;
    let bytes = line.as_bytes();

    while pos < bytes.len() {
        // Try [Thinking: ...] patterns at current position
        if let Some(rest) = line.get(pos..) {
            if rest.starts_with("[Thinking:") {
                let content_start = pos + "[Thinking:".len();
                // Skip optional space after colon
                let content_start = if line.get(content_start..content_start + 1) == Some(" ") {
                    content_start + 1
                } else {
                    content_start
                };

                // Look for closing bracket
                if let Some(close_pos) = line[content_start..].find(']') {
                    let end = content_start + close_pos;
                    let mut content = &line[content_start..end];
                    // Strip trailing dots
                    content = content.trim_end_matches('.');
                    tokens.push(Token::Thinking(content.to_string()));
                    pos = end + 1;
                    continue;
                } else {
                    // Open form — rest of line is thinking content
                    let content = &line[content_start..];
                    tokens.push(Token::Thinking(content.to_string()));
                    pos = bytes.len();
                    continue;
                }
            }

            // Try [Tool: ...]
            if rest.starts_with("[Tool:") {
                if let Some(close_pos) = rest.find(']') {
                    let inner = &rest["[Tool:".len()..close_pos].trim();
                    tokens.push(Token::Tool(inner.to_string()));
                    pos += close_pos + 1;
                    continue;
                }
            }

            // Try [tool_use: ...]
            if rest.starts_with("[tool_use:") {
                if let Some(close_pos) = rest.find(']') {
                    let inner = &rest["[tool_use:".len()..close_pos].trim();
                    tokens.push(Token::Tool(inner.to_string()));
                    pos += close_pos + 1;
                    continue;
                }
            }

            // Try [tool_result: ...] — skip these
            if rest.starts_with("[tool_result:") {
                if let Some(close_pos) = rest.find(']') {
                    tokens.push(Token::ToolResult);
                    pos += close_pos + 1;
                    continue;
                }
            }
        }

        // No special token — advance to next potential bracket or end
        // Advance past the current character safely (handles multi-byte UTF-8)
        let char_len = line[pos..].chars().next().map_or(1, char::len_utf8);
        let next_pos = pos + char_len;
        let next_bracket = line
            .get(next_pos..)
            .and_then(|s| s.find('['))
            .map(|p| p + next_pos);
        match next_bracket {
            Some(bp) => {
                let text = &line[pos..bp];
                if !text.trim().is_empty() {
                    tokens.push(Token::Text(text.to_string()));
                }
                pos = bp;
            }
            None => {
                let text = &line[pos..];
                if !text.trim().is_empty() {
                    tokens.push(Token::Text(text.to_string()));
                }
                break;
            }
        }
    }

    tokens
}

/// Reassemble stored streaming chunks into readable sections.
///
/// Takes the raw content string from a stored assistant message and
/// returns a list of `Section` values with merged adjacent same-type runs.
pub fn reassemble_content(raw: &str) -> Vec<Section> {
    let lines: Vec<&str> = raw.split('\n').collect();
    let mut sections: Vec<Section> = Vec::new();
    let mut buf_type: Option<&str> = None; // "thinking" or "text"
    let mut buf_parts: Vec<String> = Vec::new();

    let flush =
        |sections: &mut Vec<Section>, buf_type: &mut Option<&str>, buf_parts: &mut Vec<String>| {
            if let Some(bt) = buf_type.take() {
                let joined = buf_parts.join("").trim().to_string();
                if !joined.is_empty() {
                    match bt {
                        "thinking" => sections.push(Section::Thinking(joined)),
                        "text" => sections.push(Section::Text(joined)),
                        _ => {}
                    }
                }
            }
            buf_parts.clear();
        };

    for line in lines {
        let tokens = tokenize_line(line);
        if tokens.is_empty() {
            // Empty line — treat as text newline
            if buf_type == Some("text") {
                buf_parts.push("\n".to_string());
            }
            continue;
        }

        for token in tokens {
            match token {
                Token::Tool(name) => {
                    flush(&mut sections, &mut buf_type, &mut buf_parts);
                    sections.push(Section::Tool(name));
                }
                Token::ToolResult => {
                    // Skip tool results in output
                }
                Token::Thinking(content) => {
                    if buf_type != Some("thinking") {
                        flush(&mut sections, &mut buf_type, &mut buf_parts);
                        buf_type = Some("thinking");
                    }
                    buf_parts.push(content);
                }
                Token::Text(content) => {
                    if buf_type != Some("text") {
                        flush(&mut sections, &mut buf_type, &mut buf_parts);
                        buf_type = Some("text");
                    }
                    buf_parts.push(content);
                }
            }
        }
    }

    // Final flush
    flush(&mut sections, &mut buf_type, &mut buf_parts);

    sections
}

/// Format reassembled sections as plain text (no ANSI colors)
pub fn format_sections_plain(sections: &[Section]) -> String {
    let mut parts = Vec::new();
    for section in sections {
        match section {
            Section::Thinking(content) => {
                let display = if content.len() > 500 {
                    // Find a safe char boundary at or before byte 500
                    let mut truncate_at = 500;
                    while truncate_at > 0 && !content.is_char_boundary(truncate_at) {
                        truncate_at -= 1;
                    }
                    format!("{}...", &content[..truncate_at])
                } else {
                    content.clone()
                };
                parts.push(format!("[Thinking] {display}"));
            }
            Section::Tool(name) => {
                parts.push(format!("[Tool: {name}]"));
            }
            Section::Text(content) => {
                parts.push(content.clone());
            }
        }
    }
    parts.join("\n")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Scenario: Show session reassembles streaming chunks
    // @step Given a session contains an assistant message stored as raw streaming chunks
    // @step When the agent calls SessionSearch with action "show" for that session
    // @step Then thinking chunks are merged into coherent thinking sections
    // @step And tool invocations are preserved as structured markers
    // @step And text fragments are concatenated into readable prose
    #[test]
    fn test_reassemble_simple_text() {
        let raw = "Hello \nworld\n!";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 1);
        match &sections[0] {
            Section::Text(content) => {
                // Text fragments from separate lines are concatenated
                assert!(content.contains("Hello"));
                assert!(content.contains("world"));
                assert!(content.contains("!"));
            }
            _ => panic!("Expected Text section"),
        }
    }

    #[test]
    fn test_reassemble_thinking_closed() {
        let raw = "[Thinking: analyzing the code...]";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 1);
        match &sections[0] {
            Section::Thinking(content) => {
                assert_eq!(content, "analyzing the code");
            }
            _ => panic!("Expected Thinking section, got {sections:?}"),
        }
    }

    #[test]
    fn test_reassemble_thinking_open_form() {
        let raw = "[Thinking: analyzing the code without closing bracket";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 1);
        match &sections[0] {
            Section::Thinking(content) => {
                assert_eq!(content, "analyzing the code without closing bracket");
            }
            _ => panic!("Expected Thinking section"),
        }
    }

    #[test]
    fn test_reassemble_multiple_thinking_merge() {
        let raw = "[Thinking: part one...]\n[Thinking: part two...]";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 1);
        match &sections[0] {
            Section::Thinking(content) => {
                assert!(content.contains("part one"));
                assert!(content.contains("part two"));
            }
            _ => panic!("Expected merged Thinking section"),
        }
    }

    #[test]
    fn test_reassemble_tool_marker() {
        let raw = "[Tool: Read]";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 1);
        match &sections[0] {
            Section::Tool(name) => {
                assert_eq!(name, "Read");
            }
            _ => panic!("Expected Tool section"),
        }
    }

    #[test]
    fn test_reassemble_tool_use_marker() {
        let raw = "[tool_use: Edit]";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 1);
        match &sections[0] {
            Section::Tool(name) => {
                assert_eq!(name, "Edit");
            }
            _ => panic!("Expected Tool section"),
        }
    }

    #[test]
    fn test_reassemble_tool_result_skipped() {
        let raw = "[tool_result: success]\nSome text";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 1);
        match &sections[0] {
            Section::Text(content) => {
                assert_eq!(content, "Some text");
            }
            _ => panic!("Expected Text section only"),
        }
    }

    #[test]
    fn test_reassemble_mixed_content() {
        let raw = "[Thinking: planning...]\n[Tool: Read]\nHere is the file content\nMore text";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 3);
        assert!(matches!(&sections[0], Section::Thinking(_)));
        assert!(matches!(&sections[1], Section::Tool(_)));
        assert!(matches!(&sections[2], Section::Text(_)));
    }

    #[test]
    fn test_reassemble_text_fragments_concatenated() {
        // Simulates SSE streaming split mid-word
        let raw = "The quick \nbrown fox \njumps over";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 1);
        match &sections[0] {
            Section::Text(content) => {
                assert!(content.contains("quick"));
                assert!(content.contains("brown"));
                assert!(content.contains("jumps"));
            }
            _ => panic!("Expected Text section"),
        }
    }

    #[test]
    fn test_format_sections_plain() {
        let sections = vec![
            Section::Thinking("some thinking".to_string()),
            Section::Tool("Read".to_string()),
            Section::Text("response text".to_string()),
        ];
        let output = format_sections_plain(&sections);
        assert!(output.contains("[Thinking] some thinking"));
        assert!(output.contains("[Tool: Read]"));
        assert!(output.contains("response text"));
    }

    #[test]
    fn test_format_sections_truncates_long_thinking() {
        let long_thinking = "x".repeat(600);
        let sections = vec![Section::Thinking(long_thinking)];
        let output = format_sections_plain(&sections);
        assert!(output.contains("..."));
        // Should be truncated to ~500 chars + "..."
        assert!(output.len() < 600);
    }

    #[test]
    fn test_empty_input() {
        let sections = reassemble_content("");
        assert!(sections.is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        let sections = reassemble_content("   \n   \n   ");
        assert!(sections.is_empty());
    }

    #[test]
    fn test_multibyte_emoji_text() {
        // Regression: line starting with multi-byte char (✅ is 3 bytes)
        // previously panicked at line[pos + 1..] when pos=0
        let raw = "✅ What";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 1);
        match &sections[0] {
            Section::Text(content) => {
                assert!(content.contains('✅'));
                assert!(content.contains("What"));
            }
            _ => panic!("Expected Text section"),
        }
    }

    #[test]
    fn test_multibyte_emoji_before_bracket() {
        let raw = "✅ [Tool: Read]";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 2);
        assert!(matches!(&sections[0], Section::Text(_)));
        assert!(matches!(&sections[1], Section::Tool(_)));
    }

    #[test]
    fn test_multibyte_in_thinking_truncation() {
        // Build a string with multi-byte chars near the 500-byte boundary
        let prefix = "x".repeat(498);
        let content = format!("{prefix}✅✅ extra");
        let sections = vec![Section::Thinking(content)];
        let output = format_sections_plain(&sections);
        // Should not panic and should contain truncation marker
        assert!(output.contains("..."));
    }

    // ================================================================
    // Comprehensive UTF-8 safety tests
    // Covers: CJK (3-byte), 4-byte emoji, mixed widths, boundaries
    // ================================================================

    #[test]
    fn test_utf8_cjk_text_standalone() {
        // CJK characters are 3 bytes each (like ✅)
        let raw = "中文测试";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 1);
        match &sections[0] {
            Section::Text(content) => assert_eq!(content, "中文测试"),
            _ => panic!("Expected Text section"),
        }
    }

    #[test]
    fn test_utf8_cjk_before_bracket() {
        // CJK text then a bracket marker — tests the "advance past current char" path
        let raw = "中文 [Tool: Read]";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 2);
        match &sections[0] {
            Section::Text(t) => assert!(t.contains("中文")),
            _ => panic!("Expected Text first"),
        }
        match &sections[1] {
            Section::Tool(name) => assert_eq!(name, "Read"),
            _ => panic!("Expected Tool second"),
        }
    }

    #[test]
    fn test_utf8_4byte_emoji_standalone() {
        // 4-byte emoji (U+1F600 = 😀 is 4 bytes in UTF-8)
        let raw = "😀 Hello";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 1);
        match &sections[0] {
            Section::Text(content) => {
                assert!(content.contains('😀'));
                assert!(content.contains("Hello"));
            }
            _ => panic!("Expected Text section"),
        }
    }

    #[test]
    fn test_utf8_4byte_emoji_before_bracket() {
        // 4-byte emoji immediately before a bracket
        let raw = "🎉[Tool: Bash]";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 2);
        match &sections[0] {
            Section::Text(t) => assert!(t.contains('🎉')),
            _ => panic!("Expected Text first"),
        }
        assert!(matches!(&sections[1], Section::Tool(_)));
    }

    #[test]
    fn test_utf8_mixed_widths_in_line() {
        // ASCII (1-byte) + CJK (3-byte) + 4-byte emoji all in one line
        let raw = "Hello 世界 🌍 done";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 1);
        match &sections[0] {
            Section::Text(content) => {
                assert!(content.contains("Hello"));
                assert!(content.contains("世界"));
                assert!(content.contains('🌍'));
                assert!(content.contains("done"));
            }
            _ => panic!("Expected Text section"),
        }
    }

    #[test]
    fn test_utf8_all_multibyte_line() {
        // Entire line is multi-byte characters — no ASCII at all
        let raw = "中文日本語한국어";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 1);
        match &sections[0] {
            Section::Text(content) => assert_eq!(content, "中文日本語한국어"),
            _ => panic!("Expected Text section"),
        }
    }

    #[test]
    fn test_utf8_emoji_sequence_before_bracket() {
        // Multiple consecutive 4-byte emoji then a bracket
        let raw = "🔥🚀✨ [Thinking: analyzing...]";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 2);
        match &sections[0] {
            Section::Text(t) => {
                assert!(t.contains('🔥'));
                assert!(t.contains('🚀'));
                assert!(t.contains('✨'));
            }
            _ => panic!("Expected Text first"),
        }
        assert!(matches!(&sections[1], Section::Thinking(_)));
    }

    #[test]
    fn test_utf8_inside_thinking_brackets() {
        // Multi-byte characters inside [Thinking: ...] content
        let raw = "[Thinking: 分析代码中...]";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 1);
        match &sections[0] {
            Section::Thinking(content) => assert!(content.contains("分析代码中")),
            _ => panic!("Expected Thinking section"),
        }
    }

    #[test]
    fn test_utf8_cjk_in_tool_name() {
        // While unlikely in practice, ensure bracket parsing handles CJK
        let raw = "[Tool: テスト]";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 1);
        match &sections[0] {
            Section::Tool(name) => assert_eq!(name, "テスト"),
            _ => panic!("Expected Tool section"),
        }
    }

    #[test]
    fn test_utf8_truncation_at_exact_cjk_boundary() {
        // 500 bytes of 3-byte CJK = 166 chars + 2 bytes remainder
        // Byte 500 lands INSIDE a CJK char (which starts at byte 498)
        let cjk_str: String = std::iter::repeat_n('中', 200).collect(); // 600 bytes
        let sections = vec![Section::Thinking(cjk_str)];
        let output = format_sections_plain(&sections);
        assert!(output.contains("..."));
        // Verify the output is valid UTF-8 (would panic on format! if not)
        assert!(output.is_char_boundary(0));
    }

    #[test]
    fn test_utf8_truncation_at_exact_4byte_boundary() {
        // 4-byte emoji: byte 500 could land 1, 2, or 3 bytes into a char
        let emoji_str: String = std::iter::repeat_n('😀', 150).collect(); // 600 bytes
        let sections = vec![Section::Thinking(emoji_str)];
        let output = format_sections_plain(&sections);
        assert!(output.contains("..."));
        // The truncated content must be valid UTF-8
        let thinking_part = output.strip_prefix("[Thinking] ").unwrap();
        let without_dots = thinking_part.strip_suffix("...").unwrap();
        assert!(without_dots.is_char_boundary(without_dots.len()));
    }

    #[test]
    fn test_utf8_2byte_chars() {
        // 2-byte characters (Latin extended, e.g. é = 0xC3 0xA9)
        let raw = "café résumé naïve";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 1);
        match &sections[0] {
            Section::Text(content) => {
                assert!(content.contains("café"));
                assert!(content.contains("résumé"));
                assert!(content.contains("naïve"));
            }
            _ => panic!("Expected Text section"),
        }
    }

    #[test]
    fn test_utf8_2byte_before_bracket() {
        let raw = "café [Tool: Read]";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 2);
        match &sections[0] {
            Section::Text(t) => assert!(t.contains("café")),
            _ => panic!("Expected Text first"),
        }
        assert!(matches!(&sections[1], Section::Tool(_)));
    }

    #[test]
    fn test_utf8_multiline_mixed() {
        // Multi-byte on multiple lines with brackets interspersed
        let raw = "🔥 开始\n[Thinking: 分析...]\n[Tool: Read]\nrésultat final 🎉";
        let sections = reassemble_content(raw);
        assert_eq!(sections.len(), 4);
        assert!(matches!(&sections[0], Section::Text(_)));
        assert!(matches!(&sections[1], Section::Thinking(_)));
        assert!(matches!(&sections[2], Section::Tool(_)));
        assert!(matches!(&sections[3], Section::Text(_)));
        match &sections[3] {
            Section::Text(t) => {
                assert!(t.contains("résultat"));
                assert!(t.contains('🎉'));
            }
            _ => panic!("Expected Text last"),
        }
    }

    #[test]
    fn test_utf8_truncation_boundary_all_byte_widths() {
        // Test truncation safety for 1, 2, 3, and 4-byte chars near byte 500
        for (label, ch, byte_len) in [
            ("ascii", 'x', 1),
            ("2-byte", 'é', 2),
            ("3-byte", '中', 3),
            ("4-byte", '😀', 4),
        ] {
            // Fill up to just past 500 bytes
            let count = (502 / byte_len) + 1;
            let content: String = std::iter::repeat_n(ch, count).collect();
            assert!(
                content.len() > 500,
                "{label}: expected > 500 bytes, got {}",
                content.len()
            );

            let sections = vec![Section::Thinking(content)];
            let output = format_sections_plain(&sections);
            assert!(
                output.contains("..."),
                "{label}: expected truncation marker"
            );
            // Output must be valid UTF-8 (implicit — it's a String)
            // But also verify the sliced content doesn't end mid-char
            let thinking_part = output.strip_prefix("[Thinking] ").unwrap();
            let without_dots = thinking_part.strip_suffix("...").unwrap();
            assert!(
                without_dots.is_char_boundary(without_dots.len()),
                "{label}: truncation landed on invalid char boundary"
            );
        }
    }
}
