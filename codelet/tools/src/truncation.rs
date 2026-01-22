//! Output truncation utilities
//!
//! Shared functions for truncating tool output to stay within limits.

use super::limits::OutputLimits;

/// Result of truncating output
#[derive(Debug)]
pub struct TruncateResult {
    /// The truncated output string
    pub output: String,
    /// Whether the output was truncated due to character limit
    pub char_truncated: bool,
    /// Number of items remaining after truncation
    pub remaining_count: usize,
    /// Number of items included in output
    pub included_count: usize,
}

/// Truncate output lines to stay within character limit
///
/// # Arguments
/// * `lines` - Vector of lines to truncate
/// * `max_chars` - Maximum characters allowed (defaults to MAX_OUTPUT_CHARS)
///
/// # Returns
/// TruncateResult with truncated output and metadata
pub fn truncate_output(lines: &[String], max_chars: usize) -> TruncateResult {
    let mut output = String::new();
    let mut included_count = 0;

    for line in lines {
        let line_with_newline = format!("{line}\n");
        if output.len() + line_with_newline.len() > max_chars {
            break;
        }
        output.push_str(&line_with_newline);
        included_count += 1;
    }

    let remaining_count = lines.len().saturating_sub(included_count);
    let char_truncated = remaining_count > 0;

    TruncateResult {
        output,
        char_truncated,
        remaining_count,
        included_count,
    }
}

/// Truncate a single line to max length with ellipsis
///
/// # Arguments
/// * `line` - The line to truncate
/// * `max_length` - Maximum length in bytes (defaults to MAX_LINE_LENGTH)
///
/// # Returns
/// Truncated line with "..." suffix if truncated
///
/// # Safety
/// Uses `str::get()` to handle UTF-8 boundaries safely. If the truncation
/// point falls within a multi-byte character, the function finds the nearest
/// valid UTF-8 boundary before that point.
pub fn truncate_line(line: &str, max_length: usize) -> String {
    if line.len() <= max_length {
        line.to_string()
    } else {
        // Calculate the target truncation point (leave room for "...")
        let target = max_length.saturating_sub(3);
        
        // Find a valid UTF-8 boundary at or before target
        // str::get() returns None if the range doesn't align with char boundaries
        // We iterate backwards from target to find the nearest valid boundary
        let truncate_at = (0..=target)
            .rev()
            .find(|&i| line.is_char_boundary(i))
            .unwrap_or(0);
        
        // Use get() for safe slicing (returns None if indices are invalid)
        match line.get(..truncate_at) {
            Some(truncated) => format!("{}...", truncated),
            None => "...".to_string(), // Fallback for edge cases
        }
    }
}

/// Format a truncation warning message
///
/// # Arguments
/// * `remaining_count` - Number of items not included
/// * `item_type` - Type of items (e.g., "lines", "files")
/// * `char_truncated` - Whether truncation was due to character limit
/// * `max_chars` - The character limit that was applied
///
/// # Returns
/// Formatted warning string
pub fn format_truncation_warning(
    remaining_count: usize,
    item_type: &str,
    char_truncated: bool,
    max_chars: usize,
) -> String {
    if char_truncated {
        format!(
            "... [{remaining_count} {item_type} truncated - output truncated at {max_chars} chars] ..."
        )
    } else {
        format!("... [{remaining_count} {item_type} truncated] ...")
    }
}

/// Truncate line with default limit
pub fn truncate_line_default(line: &str) -> String {
    truncate_line(line, OutputLimits::MAX_LINE_LENGTH)
}

/// Process output lines, replacing long lines with a placeholder
///
/// # Arguments
/// * `output` - The output string to process
///
/// # Returns
/// Vector of processed lines with long lines replaced
pub fn process_output_lines(output: &str) -> Vec<String> {
    output
        .lines()
        .map(|line| {
            if line.len() > OutputLimits::MAX_LINE_LENGTH {
                "[Omitted long line]".to_string()
            } else {
                line.to_string()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_line_short_line() {
        // Lines shorter than max_length should not be truncated
        let result = truncate_line("hello", 10);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_line_at_boundary() {
        // Lines exactly at max_length should not be truncated
        let result = truncate_line("hello", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_line_long_ascii() {
        // Long ASCII lines should be truncated with ellipsis
        let result = truncate_line("hello world", 8);
        // max_length=8, target=5, should truncate to "hello..."
        assert_eq!(result, "hello...");
    }

    #[test]
    fn test_truncate_line_utf8_boundary_safety() {
        // Test that truncation doesn't panic on multi-byte UTF-8 characters
        // "héllo" = h(1) + é(2) + l(1) + l(1) + o(1) = 6 bytes
        let result = truncate_line("héllo world", 8);
        // Should not panic and should produce valid UTF-8
        assert!(result.is_ascii() || result.chars().count() > 0);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_line_utf8_middle_of_char() {
        // Create a string where truncation would land in the middle of a character
        // "日本語" = 3 chars, 9 bytes (each char is 3 bytes)
        let japanese = "日本語test";
        let result = truncate_line(japanese, 7); // Would land in middle of 3rd kanji
        // Should not panic and should be valid UTF-8
        assert!(result.ends_with("...") || result.len() <= 7);
        // Verify it's valid UTF-8 by iterating chars
        let _ = result.chars().count();
    }

    #[test]
    fn test_truncate_line_emoji() {
        // Emojis can be 4 bytes each
        let emoji_line = "👋🌍🎉 hello";
        let result = truncate_line(emoji_line, 10);
        // Should not panic
        assert!(result.ends_with("..."));
        // Verify valid UTF-8
        let _ = result.chars().count();
    }

    #[test]
    fn test_truncate_line_very_small_max() {
        // Edge case: max_length smaller than "..."
        let result = truncate_line("hello world", 2);
        // Should produce "..." or empty depending on implementation
        // Key is it should not panic
        assert!(result.len() <= 5); // "..." is 3 chars, plus maybe up to 2
    }

    #[test]
    fn test_truncate_line_zero_max() {
        // Edge case: max_length of 0
        let result = truncate_line("hello world", 0);
        // Should not panic, produce "..." 
        assert_eq!(result, "...");
    }

    #[test]
    fn test_truncate_output_empty() {
        let result = truncate_output(&[], 1000);
        assert_eq!(result.output, "");
        assert_eq!(result.included_count, 0);
        assert_eq!(result.remaining_count, 0);
        assert!(!result.char_truncated);
    }

    #[test]
    fn test_truncate_output_within_limit() {
        let lines = vec!["line1".to_string(), "line2".to_string()];
        let result = truncate_output(&lines, 1000);
        assert_eq!(result.output, "line1\nline2\n");
        assert_eq!(result.included_count, 2);
        assert_eq!(result.remaining_count, 0);
        assert!(!result.char_truncated);
    }

    #[test]
    fn test_truncate_output_exceeds_limit() {
        let lines = vec![
            "line1".to_string(),
            "line2".to_string(),
            "line3".to_string(),
        ];
        // "line1\n" = 6 chars, "line2\n" = 6 chars
        // Set limit to 7 so only first line fits
        let result = truncate_output(&lines, 7);
        assert_eq!(result.output, "line1\n");
        assert_eq!(result.included_count, 1);
        assert_eq!(result.remaining_count, 2);
        assert!(result.char_truncated);
    }
}
