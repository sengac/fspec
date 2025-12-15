//! Large Write Intent Detection (CLI-019)
//!
//! Detects when a user is likely requesting a large file write operation
//! (500+ lines, complete systems, comprehensive implementations) and
//! generates chunking guidance to help the LLM avoid maxOutputTokens limits.
//!
//! Reference: spec/features/add-large-write-intent-detection-and-chunking-guidance.feature

use regex::Regex;

/// Keywords that indicate a large write operation.
///
/// These words suggest the user wants a substantial, multi-part implementation
/// rather than a small edit or fix.
pub const LARGE_WRITE_KEYWORDS: &[&str] = &["complete", "comprehensive", "entire", "full", "all"];

/// Regex pattern for detecting line count indicators in prompts.
///
/// Matches patterns like "500 lines", "1000+ line", "500 line"
pub const LINE_COUNT_PATTERN: &str = r"\b\d{3,}\+?\s*lines?\b";

/// Keywords that indicate multiple files or a system-level implementation.
pub const MULTIPLE_FILE_KEYWORDS: &[&str] = &["all files", "multiple files", "system"];

/// Result of large write intent detection.
#[derive(Debug, Clone, Default)]
pub struct LargeWriteDetection {
    /// Whether large write intent was detected
    pub detected: bool,
    /// The pattern that matched (empty if not detected)
    pub matched_pattern: String,
}

/// Detect large write intent from a user prompt.
///
/// Uses keyword matching and regex patterns to identify prompts that
/// are likely to result in large file writes exceeding maxOutputTokens.
///
/// # Arguments
/// * `prompt` - The user's input prompt
///
/// # Returns
/// * `LargeWriteDetection` indicating whether intent was detected and what matched
///
/// # Examples
/// ```
/// use codelet_cli::large_write_intent::detect_large_write_intent;
///
/// let detection = detect_large_write_intent("write a complete REST API");
/// assert!(detection.detected);
///
/// let detection = detect_large_write_intent("fix the typo on line 5");
/// assert!(!detection.detected);
/// ```
pub fn detect_large_write_intent(prompt: &str) -> LargeWriteDetection {
    let prompt_lower = prompt.to_lowercase();

    // Check for large write keywords with word boundary matching
    for keyword in LARGE_WRITE_KEYWORDS {
        // Build a regex pattern that matches the keyword as a whole word
        let pattern = format!(r"\b{}\b", regex::escape(keyword));
        if let Ok(re) = Regex::new(&pattern) {
            if re.is_match(&prompt_lower) {
                return LargeWriteDetection {
                    detected: true,
                    matched_pattern: format!("keyword: {keyword}"),
                };
            }
        }
    }

    // Check for line count indicators
    if let Ok(re) = Regex::new(LINE_COUNT_PATTERN) {
        if re.is_match(&prompt_lower) {
            return LargeWriteDetection {
                detected: true,
                matched_pattern: "line count indicator".to_string(),
            };
        }
    }

    // Check for multiple file keywords
    for keyword in MULTIPLE_FILE_KEYWORDS {
        if prompt_lower.contains(keyword) {
            return LargeWriteDetection {
                detected: true,
                matched_pattern: format!("multiple files keyword: {keyword}"),
            };
        }
    }

    LargeWriteDetection::default()
}

/// Generate chunking guidance system-reminder for the LLM.
///
/// When large write intent is detected, this generates a system-reminder
/// that instructs the LLM to break the work into multiple Write calls
/// to avoid maxOutputTokens limits.
///
/// # Arguments
/// * `detection` - The result of large write intent detection
///
/// # Returns
/// * Empty string if no intent detected, otherwise chunking guidance
pub fn generate_chunking_guidance(detection: &LargeWriteDetection) -> String {
    if !detection.detected {
        return String::new();
    }

    format!(
        r#"<system-reminder>
LARGE WRITE INTENT DETECTED

The user is requesting a substantial implementation ({}).

To avoid hitting maxOutputTokens limits:
1. Break the work into multiple, incremental Write calls
2. Build the file step-by-step, adding sections one at a time
3. Use Edit tool to append content to existing files
4. Create separate files for distinct components
5. Implement core functionality first, then add features

For large files (500+ lines):
- Write the file in 2-4 parts
- Each Write/Edit call should produce 100-200 lines
- Verify each part compiles/works before continuing

This guidance is invisible to the user. Follow it to ensure successful file generation.
</system-reminder>"#,
        detection.matched_pattern
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_complete() {
        let detection = detect_large_write_intent("write a complete REST API");
        assert!(detection.detected);
        assert!(detection.matched_pattern.contains("complete"));
    }

    #[test]
    fn test_detect_comprehensive() {
        let detection = detect_large_write_intent("create a comprehensive test suite");
        assert!(detection.detected);
        assert!(detection.matched_pattern.contains("comprehensive"));
    }

    #[test]
    fn test_no_detect_small_task() {
        let detection = detect_large_write_intent("fix the typo");
        assert!(!detection.detected);
    }

    #[test]
    fn test_case_insensitive() {
        let detection = detect_large_write_intent("Write a COMPLETE application");
        assert!(detection.detected);
    }

    #[test]
    fn test_no_partial_match() {
        let detection = detect_large_write_intent("completely unrelated task");
        assert!(!detection.detected);
    }

    #[test]
    fn test_line_count_detection() {
        let detection = detect_large_write_intent("write a 500 line module");
        assert!(detection.detected);
        assert!(detection.matched_pattern.contains("line"));
    }

    #[test]
    fn test_generate_guidance() {
        let detection = LargeWriteDetection {
            detected: true,
            matched_pattern: "keyword: complete".to_string(),
        };
        let guidance = generate_chunking_guidance(&detection);
        assert!(guidance.contains("system-reminder"));
        assert!(guidance.contains("multiple"));
        assert!(guidance.contains("incremental"));
    }

    #[test]
    fn test_no_guidance_when_not_detected() {
        let detection = LargeWriteDetection::default();
        let guidance = generate_chunking_guidance(&detection);
        assert!(guidance.is_empty());
    }
}
