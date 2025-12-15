// Bash syntax highlighting using tree-sitter-bash
// Used ONLY for command preview overlays (ApprovalRequest::Exec), NOT for markdown code blocks

use anyhow::Result;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

/// Highlight categories for bash - minimal styling following codex pattern
#[derive(Debug, Clone, Copy)]
enum BashHighlight {
    Comment,  // Dimmed
    Operator, // Dimmed
    String,   // Dimmed
    _Default, // No special styling
}

// Highlight index constants (mapped from tree-sitter-bash grammar)
const HIGHLIGHT_INDEX_COMMENT: usize = 0;
const HIGHLIGHT_INDEX_OPERATOR: usize = 1;
const HIGHLIGHT_INDEX_STRING: usize = 2;

// ANSI codes for bash highlighting
const ANSI_DIM: &str = "\x1b[2m";
const ANSI_DIM_RESET: &str = "\x1b[22m";

/// Highlight bash command with minimal styling (dims comments, operators, strings)
/// Returns ANSI-styled strings, one per line
pub fn highlight_bash_command(script: &str) -> Vec<String> {
    match highlight_bash_to_lines(script) {
        Ok(lines) => lines,
        Err(_) => {
            // Fallback: return script as-is if highlighting fails
            script.lines().map(std::string::ToString::to_string).collect()
        }
    }
}

fn highlight_bash_to_lines(script: &str) -> Result<Vec<String>> {
    let mut highlighter = Highlighter::new();

    // Tree-sitter-bash grammar and highlights
    let bash_language = tree_sitter_bash::LANGUAGE;
    let mut config = HighlightConfiguration::new(
        bash_language.into(),
        "bash",
        tree_sitter_bash::HIGHLIGHT_QUERY,
        "", // No injections
        "", // No locals
    )?;

    // Map highlight names to our categories
    let highlight_names = &[
        "comment", "operator", "string", "constant", "embedded", "function", "keyword", "number",
        "property",
    ];
    config.configure(highlight_names);

    let highlights = highlighter.highlight(&config, script.as_bytes(), None, |_| None)?;

    let mut result_lines = Vec::new();
    let mut current_line = String::new();
    let mut current_style: Option<BashHighlight> = None;

    for event in highlights {
        match event? {
            HighlightEvent::Source { start, end } => {
                let text = &script[start..end];

                // Apply ANSI styling based on current style
                let styled_text = match current_style {
                    Some(BashHighlight::Comment)
                    | Some(BashHighlight::Operator)
                    | Some(BashHighlight::String) => {
                        // Dim modifier
                        format!("{ANSI_DIM}{text}{ANSI_DIM_RESET}")
                    }
                    _ => text.to_string(), // Default styling
                };

                // Handle newlines
                for (i, line) in styled_text.split('\n').enumerate() {
                    if i > 0 {
                        result_lines.push(current_line.clone());
                        current_line.clear();
                    }
                    current_line.push_str(line);
                }
            }
            HighlightEvent::HighlightStart(highlight) => {
                current_style = match highlight.0 {
                    HIGHLIGHT_INDEX_COMMENT => Some(BashHighlight::Comment),
                    HIGHLIGHT_INDEX_OPERATOR => Some(BashHighlight::Operator),
                    HIGHLIGHT_INDEX_STRING => Some(BashHighlight::String),
                    _ => None, // Default for all others
                };
            }
            HighlightEvent::HighlightEnd => {
                current_style = None;
            }
        }
    }

    // Push final line
    if !current_line.is_empty() {
        result_lines.push(current_line);
    }

    Ok(result_lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_bash_basic() {
        let command = "echo hello";
        let lines = highlight_bash_command(command);
        assert!(!lines.is_empty());
        assert!(lines.join("").contains("echo"));
        assert!(lines.join("").contains("hello"));
    }

    #[test]
    fn test_highlight_bash_with_string() {
        let command = "echo \"hi\"";
        let lines = highlight_bash_command(command);
        let result = lines.join("");

        // String should be dimmed (\x1b[2m)
        assert!(result.contains("\x1b[2m"));
        // Should contain the string content
        assert!(result.contains("hi"));
    }

    #[test]
    fn test_highlight_bash_with_operators() {
        let command = "echo \"hi\" && bar | qux";
        let lines = highlight_bash_command(command);
        let result = lines.join("");

        // Operators and strings should be dimmed
        assert!(result.contains("\x1b[2m"));
        // Content should be present
        assert!(result.contains("echo"));
        assert!(result.contains("hi"));
        assert!(result.contains("&&"));
        assert!(result.contains("bar"));
        assert!(result.contains("|"));
        assert!(result.contains("qux"));
    }
}
