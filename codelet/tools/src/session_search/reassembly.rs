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
        let next_bracket = line[pos + 1..].find('[').map(|p| p + pos + 1);
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

    let flush = |sections: &mut Vec<Section>,
                 buf_type: &mut Option<&str>,
                 buf_parts: &mut Vec<String>| {
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
                    format!("{}...", &content[..500])
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
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
}
