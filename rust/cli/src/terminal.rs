//! Terminal display utilities
//!
//! Provides ANSI escape codes and terminal formatting utilities.
//! Single Responsibility: Handle terminal-specific display concerns.

/// ANSI escape codes for terminal text styling
///
/// These codes work in most Unix terminals and Windows Terminal.
/// Use `RESET` after applying any style to return to default.
pub mod style {
    /// Red foreground color
    pub const RED: &str = "\x1b[31m";
    /// Yellow foreground color
    pub const YELLOW: &str = "\x1b[33m";
    /// Green foreground color
    pub const GREEN: &str = "\x1b[32m";
    /// Cyan foreground color
    pub const CYAN: &str = "\x1b[36m";
    /// Bold text weight
    pub const BOLD: &str = "\x1b[1m";
    /// Dim/faint text
    pub const DIM: &str = "\x1b[2m";
    /// Reset all styles to default
    pub const RESET: &str = "\x1b[0m";
}

/// Format text with red color (for errors)
#[inline]
pub fn red(text: &str) -> String {
    format!("{}{}{}", style::RED, text, style::RESET)
}

/// Format text with bold red (for critical errors)
#[inline]
pub fn bold_red(text: &str) -> String {
    format!("{}{}{}{}", style::BOLD, style::RED, text, style::RESET)
}

/// Format text with yellow color (for warnings)
#[inline]
pub fn yellow(text: &str) -> String {
    format!("{}{}{}", style::YELLOW, text, style::RESET)
}

/// Format text with green color (for success)
#[inline]
pub fn green(text: &str) -> String {
    format!("{}{}{}", style::GREEN, text, style::RESET)
}

/// Format text with dim style (for secondary info)
#[inline]
pub fn dim(text: &str) -> String {
    format!("{}{}{}", style::DIM, text, style::RESET)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_red_wraps_text_with_codes() {
        let result = red("error");
        assert!(result.starts_with(style::RED));
        assert!(result.ends_with(style::RESET));
        assert!(result.contains("error"));
    }

    #[test]
    fn test_bold_red_includes_both_codes() {
        let result = bold_red("critical");
        assert!(result.contains(style::BOLD));
        assert!(result.contains(style::RED));
        assert!(result.contains("critical"));
        assert!(result.ends_with(style::RESET));
    }

    #[test]
    fn test_yellow_wraps_text() {
        let result = yellow("warning");
        assert!(result.contains(style::YELLOW));
        assert!(result.contains("warning"));
    }

    #[test]
    fn test_green_wraps_text() {
        let result = green("success");
        assert!(result.contains(style::GREEN));
        assert!(result.contains("success"));
    }

    #[test]
    fn test_dim_wraps_text() {
        let result = dim("secondary");
        assert!(result.contains(style::DIM));
        assert!(result.contains("secondary"));
    }
}
