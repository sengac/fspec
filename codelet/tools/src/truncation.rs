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
/// * `max_length` - Maximum length (defaults to MAX_LINE_LENGTH)
///
/// # Returns
/// Truncated line with "..." suffix if truncated
pub fn truncate_line(line: &str, max_length: usize) -> String {
    if line.len() <= max_length {
        line.to_string()
    } else {
        format!("{}...", &line[..max_length.saturating_sub(3)])
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

/// Truncate output with default limits
pub fn truncate_output_default(lines: &[String]) -> TruncateResult {
    truncate_output(lines, OutputLimits::MAX_OUTPUT_CHARS)
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
