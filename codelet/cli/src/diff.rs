// Diff rendering using diffy crate
// Renders file changes with green additions and red deletions

use anyhow::Result;

// ANSI color codes for diff rendering
const ANSI_GREEN: &str = "\x1b[32m";
const ANSI_RED: &str = "\x1b[31m";
const ANSI_RESET: &str = "\x1b[39m";

/// Render diff summary showing addition and deletion counts
/// Format: "+5 -2" with green for additions, red for deletions
pub fn render_diff_summary(additions: usize, deletions: usize) -> String {
    let green_plus = format!("{ANSI_GREEN}+{additions}{ANSI_RESET}");
    let red_minus = format!("{ANSI_RED}-{deletions}{ANSI_RESET}");
    format!("{green_plus} {red_minus}")
}

/// Render a single diff line with color-coded prefix
/// prefix: '+' for addition, '-' for deletion, ' ' for context
pub fn render_diff_line(line_number: usize, prefix: char, content: &str) -> String {
    match prefix {
        '+' => {
            // Addition: green color
            format!(
                "{line_number} {ANSI_GREEN}{prefix} {content}{ANSI_RESET}"
            )
        }
        '-' => {
            // Deletion: red color
            format!(
                "{line_number} {ANSI_RED}{prefix} {content}{ANSI_RESET}"
            )
        }
        _ => {
            // Context: default color
            format!("{line_number} {prefix} {content}")
        }
    }
}

/// Render a complete file diff with file path, line numbers, and colored changes
pub fn render_file_diff(file_path: &str, changes: &[(usize, &str, char)]) -> Result<String> {
    let mut output = Vec::new();

    // File path header
    output.push(format!("File: {file_path}"));
    output.push(String::new()); // Blank line

    // Render each change
    for (line_num, content, prefix) in changes {
        let line = render_diff_line(*line_num, *prefix, content);
        output.push(line);
    }

    Ok(output.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_diff_summary() {
        let summary = render_diff_summary(5, 2);

        // Should contain ANSI green code for additions
        assert!(summary.contains("\x1b[32m"));
        // Should contain ANSI red code for deletions
        assert!(summary.contains("\x1b[31m"));
        // Should contain counts
        assert!(summary.contains("5"));
        assert!(summary.contains("2"));
        // Should contain +/- symbols
        assert!(summary.contains("+"));
        assert!(summary.contains("-"));
    }

    #[test]
    fn test_render_diff_line_addition() {
        let line = render_diff_line(1, '+', "new line");

        // Should contain line number
        assert!(line.contains("1"));
        // Should contain + prefix
        assert!(line.contains("+"));
        // Should contain content
        assert!(line.contains("new line"));
        // Should have green color code
        assert!(line.contains("\x1b[32m"));
    }

    #[test]
    fn test_render_diff_line_deletion() {
        let line = render_diff_line(2, '-', "old line");

        // Should contain line number
        assert!(line.contains("2"));
        // Should contain - prefix
        assert!(line.contains("-"));
        // Should contain content
        assert!(line.contains("old line"));
        // Should have red color code
        assert!(line.contains("\x1b[31m"));
    }

    #[test]
    fn test_render_diff_line_context() {
        let line = render_diff_line(3, ' ', "unchanged line");

        // Should contain line number
        assert!(line.contains("3"));
        // Should contain content
        assert!(line.contains("unchanged line"));
        // Should NOT have color codes (context is default)
        assert!(!line.contains("\x1b[32m"));
        assert!(!line.contains("\x1b[31m"));
    }

    #[test]
    fn test_render_file_diff() {
        let changes = vec![(10, "let x = 3;", '-'), (10, "let x = 5;", '+')];

        let diff = render_file_diff("src/main.rs", &changes).unwrap();

        // Should contain file path
        assert!(diff.contains("src/main.rs"));
        // Should contain line numbers
        assert!(diff.contains("10"));
        // Should contain both changes
        assert!(diff.contains("let x = 3;"));
        assert!(diff.contains("let x = 5;"));
        // Should have color codes
        assert!(diff.contains("\x1b[32m")); // Green for addition
        assert!(diff.contains("\x1b[31m")); // Red for deletion
    }
}
