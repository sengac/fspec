//! Thinking Level Detection (RPC-072 lift from
//! `codelet/napi/src/thinking_level_detection.rs`).
//!
//! Detects thinking/reasoning level from prompt keywords.
//! Single source of truth for thinking level detection - used by both
//! TUI and Bridge paths via [`agent_loop`].
//!
//! Feature: spec/features/thinking-config-facade-for-provider-specific-reasoning.feature
//!
//! Priority order:
//! 1. Disable keywords (quickly, briefly, etc.) → Off
//! 2. Conversational patterns (I think, what do you think) → Off
//! 3. High-level keywords (ultrathink, think harder) → High
//! 4. Medium-level keywords (megathink, think hard) → Medium
//! 5. Low-level keywords (think about, think through) → Low
//! 6. No match → Off
//!
//! This is a verbatim lift of the canonical NAPI implementation. The
//! NAPI-only `#[napi_derive::napi]` exports at the bottom of the
//! original file are dropped; the NAPI-side keeps its own copy of
//! those bindings while codelet-napi is alive.

use crate::thinking_config::JsThinkingLevel;
use once_cell::sync::Lazy;
use regex::Regex;

// ============================================================================
// Keyword and Pattern Definitions
// ============================================================================

/// Disable keywords have HIGHEST priority - always force Off
const DISABLE_KEYWORDS: &[&str] = &[
    "quickly",
    "brief",
    "briefly",
    "fast",
    "nothink",
    "no thinking",
    "don't think hard",
    "don't overthink",
];

/// High-level patterns (explicit thinking commands)
static HIGH_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)\bultrathink\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bthink\s+harder\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bthink\s+intensely\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bthink\s+very\s+hard\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bthink\s+super\s+hard\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bthink\s+really\s+hard\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bthink\s+longer\b").expect("Invalid regex"),
    ]
});

/// Medium-level patterns
static MEDIUM_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)\bmegathink\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bthink\s+hard\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bthink\s+deeply\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bthink\s+more\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bthink\s+a\s+lot\b").expect("Invalid regex"),
    ]
});

/// Low-level patterns (command-like phrases)
static LOW_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)\bthink\s+about\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bthink\s+through\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bthink\s+carefully\b").expect("Invalid regex"),
        Regex::new(r"(?i)^think\b").expect("Invalid regex"),
        Regex::new(r"(?i)[:.]\s*think\b").expect("Invalid regex"),
    ]
});

/// Conversational patterns - DO NOT match these as thinking commands
static CONVERSATIONAL_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)\bi\s+think\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bwhat\s+do\s+you\s+think\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bdon'?t\s+think\s+so\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bwas\s+thinking\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bthinking\s+about\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bi\s+was\s+thinking\b").expect("Invalid regex"),
        Regex::new(r"(?i)\bdo\s+you\s+think\b").expect("Invalid regex"),
    ]
});

// ============================================================================
// Public API
// ============================================================================

/// Detect thinking level from prompt keywords.
pub fn detect_thinking_level(prompt: &str) -> JsThinkingLevel {
    let lower = prompt.to_lowercase();

    // 1. DISABLE keywords have highest priority
    if DISABLE_KEYWORDS.iter().any(|kw| lower.contains(kw)) {
        return JsThinkingLevel::Off;
    }

    // 2. Skip if conversational usage
    if CONVERSATIONAL_PATTERNS.iter().any(|p| p.is_match(prompt)) {
        return JsThinkingLevel::Off;
    }

    // 3. Check HIGH level
    if HIGH_PATTERNS.iter().any(|p| p.is_match(prompt)) {
        return JsThinkingLevel::High;
    }

    // 4. Check MEDIUM level
    if MEDIUM_PATTERNS.iter().any(|p| p.is_match(prompt)) {
        return JsThinkingLevel::Medium;
    }

    // 5. Check LOW level
    if LOW_PATTERNS.iter().any(|p| p.is_match(prompt)) {
        return JsThinkingLevel::Low;
    }

    // 6. Default: Off
    JsThinkingLevel::Off
}

/// Check if disable keywords were detected in prompt.
pub fn has_disable_keywords(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    DISABLE_KEYWORDS.iter().any(|kw| lower.contains(kw))
}

/// Compute effective thinking level from base level and detected level.
pub fn compute_effective_thinking_level(
    base_level: JsThinkingLevel,
    detected_level: JsThinkingLevel,
    force_off: bool,
) -> JsThinkingLevel {
    if force_off {
        return JsThinkingLevel::Off;
    }

    let base_val = base_level as u8;
    let detected_val = detected_level as u8;

    if base_val >= detected_val {
        base_level
    } else {
        detected_level
    }
}

/// Convert u8 to JsThinkingLevel (for reading from session state)
pub fn thinking_level_from_u8(value: u8) -> JsThinkingLevel {
    match value {
        0 => JsThinkingLevel::Off,
        1 => JsThinkingLevel::Low,
        2 => JsThinkingLevel::Medium,
        3 => JsThinkingLevel::High,
        _ => JsThinkingLevel::Off,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disable_keyword_quickly() {
        assert_eq!(
            detect_thinking_level("do this quickly"),
            JsThinkingLevel::Off
        );
    }

    #[test]
    fn test_disable_keyword_briefly() {
        assert_eq!(
            detect_thinking_level("briefly explain"),
            JsThinkingLevel::Off
        );
    }

    #[test]
    fn test_disable_keyword_nothink() {
        assert_eq!(
            detect_thinking_level("nothink just do it"),
            JsThinkingLevel::Off
        );
    }

    #[test]
    fn test_disable_overrides_high() {
        assert_eq!(
            detect_thinking_level("ultrathink but do it quickly"),
            JsThinkingLevel::Off
        );
    }

    #[test]
    fn test_high_ultrathink() {
        assert_eq!(
            detect_thinking_level("ultrathink about this problem"),
            JsThinkingLevel::High
        );
    }

    #[test]
    fn test_high_think_harder() {
        assert_eq!(
            detect_thinking_level("think harder about the solution"),
            JsThinkingLevel::High
        );
    }

    #[test]
    fn test_high_think_really_hard() {
        assert_eq!(
            detect_thinking_level("think really hard about this"),
            JsThinkingLevel::High
        );
    }

    #[test]
    fn test_medium_megathink() {
        assert_eq!(
            detect_thinking_level("megathink this problem"),
            JsThinkingLevel::Medium
        );
    }

    #[test]
    fn test_medium_think_hard() {
        assert_eq!(
            detect_thinking_level("think hard about this"),
            JsThinkingLevel::Medium
        );
    }

    #[test]
    fn test_medium_think_deeply() {
        assert_eq!(
            detect_thinking_level("think deeply about the design"),
            JsThinkingLevel::Medium
        );
    }

    #[test]
    fn test_low_think_about() {
        assert_eq!(
            detect_thinking_level("think about the architecture"),
            JsThinkingLevel::Low
        );
    }

    #[test]
    fn test_low_think_through() {
        assert_eq!(
            detect_thinking_level("think through this problem"),
            JsThinkingLevel::Low
        );
    }

    #[test]
    fn test_low_think_carefully() {
        assert_eq!(
            detect_thinking_level("think carefully about edge cases"),
            JsThinkingLevel::Low
        );
    }

    #[test]
    fn test_low_starts_with_think() {
        assert_eq!(
            detect_thinking_level("think: what should we do?"),
            JsThinkingLevel::Low
        );
    }

    #[test]
    fn test_conversational_i_think() {
        assert_eq!(
            detect_thinking_level("I think we should use React"),
            JsThinkingLevel::Off
        );
    }

    #[test]
    fn test_conversational_what_do_you_think() {
        assert_eq!(
            detect_thinking_level("what do you think about this?"),
            JsThinkingLevel::Off
        );
    }

    #[test]
    fn test_conversational_dont_think_so() {
        assert_eq!(
            detect_thinking_level("I don't think so"),
            JsThinkingLevel::Off
        );
    }

    #[test]
    fn test_conversational_was_thinking() {
        assert_eq!(
            detect_thinking_level("I was thinking about lunch"),
            JsThinkingLevel::Off
        );
    }

    #[test]
    fn test_no_match_returns_off() {
        assert_eq!(
            detect_thinking_level("write a function to sort arrays"),
            JsThinkingLevel::Off
        );
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(detect_thinking_level(""), JsThinkingLevel::Off);
    }

    #[test]
    fn test_effective_force_off_overrides_all() {
        let result =
            compute_effective_thinking_level(JsThinkingLevel::High, JsThinkingLevel::High, true);
        assert_eq!(result, JsThinkingLevel::Off);
    }

    #[test]
    fn test_effective_max_of_base_and_detected() {
        let result =
            compute_effective_thinking_level(JsThinkingLevel::Medium, JsThinkingLevel::High, false);
        assert_eq!(result, JsThinkingLevel::High);
    }

    #[test]
    fn test_effective_base_wins_when_higher() {
        let result =
            compute_effective_thinking_level(JsThinkingLevel::High, JsThinkingLevel::Low, false);
        assert_eq!(result, JsThinkingLevel::High);
    }

    #[test]
    fn test_effective_both_off_returns_off() {
        let result =
            compute_effective_thinking_level(JsThinkingLevel::Off, JsThinkingLevel::Off, false);
        assert_eq!(result, JsThinkingLevel::Off);
    }

    #[test]
    fn test_has_disable_keywords_true() {
        assert!(has_disable_keywords("do it quickly"));
    }

    #[test]
    fn test_has_disable_keywords_false() {
        assert!(!has_disable_keywords("think about this"));
    }

    #[test]
    fn test_level_from_u8_valid() {
        assert_eq!(thinking_level_from_u8(0), JsThinkingLevel::Off);
        assert_eq!(thinking_level_from_u8(1), JsThinkingLevel::Low);
        assert_eq!(thinking_level_from_u8(2), JsThinkingLevel::Medium);
        assert_eq!(thinking_level_from_u8(3), JsThinkingLevel::High);
    }

    #[test]
    fn test_level_from_u8_invalid_clamps() {
        assert_eq!(thinking_level_from_u8(4), JsThinkingLevel::Off);
        assert_eq!(thinking_level_from_u8(255), JsThinkingLevel::Off);
    }
}
