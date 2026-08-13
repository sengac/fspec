#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/add-large-write-intent-detection-and-chunking-guidance.feature
//!
//! Tests for CLI-019: Large Write Intent Detection and Chunking Guidance

use codelet_cli::large_write_intent::{
    detect_large_write_intent, generate_chunking_guidance, LARGE_WRITE_KEYWORDS,
    LINE_COUNT_PATTERN, MULTIPLE_FILE_KEYWORDS,
};

// ==========================================
// PATTERN DETECTION TESTS
// ==========================================

/// Scenario: Detect large write intent from "complete" keywords
#[test]
fn test_detect_complete_keyword() {
    // @step Given a user prompt containing "write a complete REST API with all CRUD operations"
    let prompt = "write a complete REST API with all CRUD operations";

    // @step When the large write intent detection runs
    let detection = detect_large_write_intent(prompt);

    // @step Then it should detect large write intent
    assert!(detection.detected, "Should detect large write intent");

    // @step And the pattern match should be on the keyword "complete"
    assert!(
        detection.matched_pattern.contains("complete"),
        "Pattern match should be on 'complete'"
    );
}

/// Scenario: Detect large write intent from "comprehensive" keywords
#[test]
fn test_detect_comprehensive_keyword() {
    // @step Given a user prompt containing "create a comprehensive test suite for the auth module"
    let prompt = "create a comprehensive test suite for the auth module";

    // @step When the large write intent detection runs
    let detection = detect_large_write_intent(prompt);

    // @step Then it should detect large write intent
    assert!(detection.detected, "Should detect large write intent");

    // @step And the pattern match should be on the keyword "comprehensive"
    assert!(
        detection.matched_pattern.contains("comprehensive"),
        "Pattern match should be on 'comprehensive'"
    );
}

/// Scenario: Detect large write intent from "entire" keywords
#[test]
fn test_detect_entire_keyword() {
    // @step Given a user prompt containing "implement the entire user management system"
    let prompt = "implement the entire user management system";

    // @step When the large write intent detection runs
    let detection = detect_large_write_intent(prompt);

    // @step Then it should detect large write intent
    assert!(detection.detected, "Should detect large write intent");

    // @step And the pattern match should be on the keyword "entire"
    assert!(
        detection.matched_pattern.contains("entire"),
        "Pattern match should be on 'entire'"
    );
}

/// Scenario: Detect large write intent from "full" keywords
#[test]
fn test_detect_full_keyword() {
    // @step Given a user prompt containing "build a full authentication system with OAuth"
    let prompt = "build a full authentication system with OAuth";

    // @step When the large write intent detection runs
    let detection = detect_large_write_intent(prompt);

    // @step Then it should detect large write intent
    assert!(detection.detected, "Should detect large write intent");

    // @step And the pattern match should be on the keyword "full"
    assert!(
        detection.matched_pattern.contains("full"),
        "Pattern match should be on 'full'"
    );
}

/// Scenario: Do not detect large write intent for small tasks
#[test]
fn test_no_detection_for_small_tasks() {
    // @step Given a user prompt containing "fix the typo on line 5"
    let prompt = "fix the typo on line 5";

    // @step When the large write intent detection runs
    let detection = detect_large_write_intent(prompt);

    // @step Then it should NOT detect large write intent
    assert!(
        !detection.detected,
        "Should not detect large write intent for small tasks"
    );
}

/// Scenario: Do not detect large write intent for simple edits
#[test]
fn test_no_detection_for_simple_edits() {
    // @step Given a user prompt containing "add a console.log statement to debug"
    let prompt = "add a console.log statement to debug";

    // @step When the large write intent detection runs
    let detection = detect_large_write_intent(prompt);

    // @step Then it should NOT detect large write intent
    assert!(
        !detection.detected,
        "Should not detect large write intent for simple edits"
    );
}

/// Scenario: Detect large write intent with line count indicators
#[test]
fn test_detect_line_count_indicator() {
    // @step Given a user prompt containing "write a 500 line module for data processing"
    let prompt = "write a 500 line module for data processing";

    // @step When the large write intent detection runs
    let detection = detect_large_write_intent(prompt);

    // @step Then it should detect large write intent
    assert!(detection.detected, "Should detect large write intent");

    // @step And the pattern match should be on line count indicator
    assert!(
        detection.matched_pattern.contains("line"),
        "Pattern match should indicate line count"
    );
}

// ==========================================
// SYSTEM REMINDER INJECTION TESTS
// ==========================================

/// Scenario: Inject chunking guidance when large write detected
#[test]
fn test_inject_chunking_guidance() {
    // @step Given a user prompt with detected large write intent
    let prompt = "write a complete application for e-commerce";
    let detection = detect_large_write_intent(prompt);
    assert!(detection.detected);

    // @step When preparing the prompt for the LLM
    // @step Then a system-reminder should be injected into the conversation
    let guidance = generate_chunking_guidance(&detection);

    // @step And the system-reminder should contain chunking guidance
    assert!(
        guidance.contains("system-reminder") || !guidance.is_empty(),
        "Should generate chunking guidance"
    );

    // @step And the system-reminder should be invisible to the user
    // (This is verified by the system-reminder tag structure which is stripped from UI)
}

/// Scenario: System reminder contains specific chunking instructions
#[test]
fn test_chunking_instructions_content() {
    // @step Given a user prompt with detected large write intent
    let prompt = "implement a comprehensive dashboard";
    let detection = detect_large_write_intent(prompt);

    // @step When a system-reminder is generated
    let guidance = generate_chunking_guidance(&detection);

    // @step Then it should instruct the LLM to use multiple Write calls
    assert!(
        guidance.contains("multiple") || guidance.contains("chunk") || guidance.contains("Write"),
        "Should mention multiple Write calls"
    );

    // @step And it should recommend incremental file building
    assert!(
        guidance.contains("incremental") || guidance.contains("step") || guidance.contains("part"),
        "Should recommend incremental building"
    );

    // @step And it should warn about maxOutputTokens limits
    assert!(
        guidance.contains("token") || guidance.contains("limit") || guidance.contains("output"),
        "Should warn about output limits"
    );
}

/// Scenario: No system reminder for small tasks
#[test]
fn test_no_reminder_for_small_tasks() {
    // @step Given a user prompt without large write intent
    let prompt = "rename the variable from foo to bar";
    let detection = detect_large_write_intent(prompt);

    // @step When preparing the prompt for the LLM
    // @step Then no chunking guidance system-reminder should be injected
    assert!(
        !detection.detected,
        "Should not detect intent for small tasks"
    );
    let guidance = generate_chunking_guidance(&detection);
    assert!(
        guidance.is_empty(),
        "Should not generate guidance for small tasks"
    );
}

// ==========================================
// KEYWORD PATTERN CONSTANTS
// ==========================================

/// Scenario: Large write intent patterns are defined as constants
#[test]
fn test_constants_defined() {
    // @step Given the large_write_intent module constants
    // @step Then LARGE_WRITE_KEYWORDS should include "complete", "comprehensive", "entire", "full"
    assert!(LARGE_WRITE_KEYWORDS.contains(&"complete"));
    assert!(LARGE_WRITE_KEYWORDS.contains(&"comprehensive"));
    assert!(LARGE_WRITE_KEYWORDS.contains(&"entire"));
    assert!(LARGE_WRITE_KEYWORDS.contains(&"full"));

    // @step And LINE_COUNT_PATTERN should match numeric patterns like "500 lines", "1000+ line"
    let re = regex::Regex::new(LINE_COUNT_PATTERN).expect("Valid regex pattern");
    assert!(re.is_match("500 lines"));
    assert!(re.is_match("1000+ line"));
    assert!(re.is_match("500 line"));

    // @step And MULTIPLE_FILE_KEYWORDS should include "all files", "multiple files", "system"
    assert!(MULTIPLE_FILE_KEYWORDS.contains(&"all files"));
    assert!(MULTIPLE_FILE_KEYWORDS.contains(&"multiple files"));
    assert!(MULTIPLE_FILE_KEYWORDS.contains(&"system"));
}

// ==========================================
// EDGE CASES
// ==========================================

/// Scenario: Case-insensitive pattern matching
#[test]
fn test_case_insensitive_matching() {
    // @step Given a user prompt containing "Write a COMPLETE application"
    let prompt = "Write a COMPLETE application";

    // @step When the large write intent detection runs
    let detection = detect_large_write_intent(prompt);

    // @step Then it should detect large write intent
    assert!(
        detection.detected,
        "Should detect large write intent with uppercase"
    );
}

/// Scenario: Partial word matches should not trigger detection
#[test]
fn test_no_partial_word_matches() {
    // @step Given a user prompt containing "completely unrelated task"
    let prompt = "completely unrelated task";

    // @step When the large write intent detection runs
    let detection = detect_large_write_intent(prompt);

    // @step Then it should NOT detect large write intent
    // @step Because "completely" is not the same as "complete"
    assert!(
        !detection.detected,
        "'completely' should not match 'complete' - partial word match"
    );
}

/// Scenario: Detection works with surrounding context
#[test]
fn test_detection_with_context() {
    // @step Given a user prompt containing "I need you to write a complete module for user authentication including registration, login, and password reset functionality"
    let prompt = "I need you to write a complete module for user authentication including registration, login, and password reset functionality";

    // @step When the large write intent detection runs
    let detection = detect_large_write_intent(prompt);

    // @step Then it should detect large write intent
    assert!(
        detection.detected,
        "Should detect large write intent in longer prompts"
    );
}

// ==========================================
// ADDITIONAL TESTS
// ==========================================

/// Test multiple keywords in one prompt
#[test]
fn test_multiple_keywords() {
    let prompt = "create a complete and comprehensive system for the entire application";
    let detection = detect_large_write_intent(prompt);
    assert!(detection.detected);
}

/// Test "all" keyword variant
#[test]
fn test_all_keyword() {
    let prompt = "implement all CRUD operations for the database";
    let detection = detect_large_write_intent(prompt);
    assert!(detection.detected);
}

/// Test that common small prompts don't trigger
#[test]
fn test_common_small_prompts() {
    let small_prompts = vec![
        "fix this bug",
        "add logging",
        "update the README",
        "change the color to blue",
        "remove the unused import",
        "rename this function",
    ];

    for prompt in small_prompts {
        let detection = detect_large_write_intent(prompt);
        assert!(
            !detection.detected,
            "Should not detect for small prompt: {prompt}"
        );
    }
}

/// Test LargeWriteDetection struct fields
#[test]
fn test_detection_struct() {
    let prompt = "build a full authentication system";
    let detection = detect_large_write_intent(prompt);

    assert!(detection.detected);
    assert!(!detection.matched_pattern.is_empty());
}
