//! Output limits for tool execution
//!
//! Defines shared constants for output truncation across all tools.

/// Output limits for tool results
pub struct OutputLimits;

impl OutputLimits {
    /// Maximum total characters in tool output
    pub const MAX_OUTPUT_CHARS: usize = 30000;

    /// Maximum characters per line before truncation
    pub const MAX_LINE_LENGTH: usize = 2000;

    /// Maximum number of lines before truncation
    pub const MAX_LINES: usize = 2000;
}
