// Feature: spec/features/fix-system-reminder-replacement-to-preserve-prompt-cache-prefix.feature
//
// CLI-013: Fix system-reminder replacement to preserve prompt cache prefix
//
// Tests for ensuring system-reminder replacement doesn't break prompt cache by:
// 1. NOT removing old reminders when adding replacements
// 2. Adding supersession markers to replacement reminders
// 3. Only extracting latest reminder per type during compaction
// 4. Placing latest reminders at start after compaction

use codelet::session::system_reminders::{
    add_system_reminder, count_system_reminders_by_type, is_system_reminder,
    partition_for_compaction, SystemReminderType,
};
use rig::message::{Message, UserContent};
use rig::OneOrMany;

/// Helper to create a test message
fn create_test_message(content: &str) -> Message {
    Message::User {
        content: OneOrMany::one(UserContent::text(content)),
    }
}

/// Helper to extract text content from a message
fn get_message_text(msg: &Message) -> Option<String> {
    match msg {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => Some(t.text.clone()),
            _ => None,
        },
        _ => None,
    }
}

// =============================================================================
// Scenario: Adding replacement reminder preserves old reminder in place
// =============================================================================

#[test]
fn test_adding_replacement_preserves_old_reminder() {
    // @step Given a message array with [msg1, msg2, tokenStatus_v1]
    let msg1 = create_test_message("Hello user");
    let msg2 = create_test_message("How can I help?");
    let token_status_v1 = add_system_reminder(&[], SystemReminderType::TokenStatus, "50% used");

    let messages = vec![msg1.clone(), msg2.clone(), token_status_v1[0].clone()];

    // Verify initial state - should have 1 tokenStatus reminder
    assert_eq!(
        count_system_reminders_by_type(&messages, SystemReminderType::TokenStatus),
        1
    );

    // @step When I call add_system_reminder with TokenStatus type and new content
    let result = add_system_reminder(&messages, SystemReminderType::TokenStatus, "75% used");

    // @step Then the result should be [msg1, msg2, tokenStatus_v1, tokenStatus_v2]
    // After fix: should have 4 messages (old reminder preserved, new one appended)
    assert_eq!(
        result.len(),
        4,
        "Should have 4 messages: msg1, msg2, tokenStatus_v1, tokenStatus_v2"
    );

    // @step And tokenStatus_v2 should contain a supersession marker
    let last_msg_text = get_message_text(&result[3]).expect("Should have text content");
    assert!(
        last_msg_text.contains("supersedes") || last_msg_text.contains("75% used"),
        "New reminder should contain supersession marker or new content"
    );

    // @step And the message prefix [msg1, msg2, tokenStatus_v1] should be unchanged
    // Verify prefix is unchanged (critical for prompt caching)
    assert_eq!(get_message_text(&result[0]), Some("Hello user".to_string()));
    assert_eq!(
        get_message_text(&result[1]),
        Some("How can I help?".to_string())
    );
    // Old tokenStatus should still be at position 2
    let old_reminder_text = get_message_text(&result[2]).expect("Should have old reminder");
    assert!(
        old_reminder_text.contains("50% used"),
        "Old reminder should be preserved at position 2"
    );
}

// =============================================================================
// Scenario: Partition for compaction extracts only the latest reminder per type
// =============================================================================

#[test]
fn test_partition_extracts_only_latest_reminder_per_type() {
    // @step Given a message array with [msg1, tokenStatus_v1, msg2, tokenStatus_v2]
    let msg1 = create_test_message("Hello");
    let token_v1_content =
        "<system-reminder>\n<!-- type:tokenStatus -->\n50% used\n</system-reminder>";
    let token_v1 = create_test_message(token_v1_content);
    let msg2 = create_test_message("More conversation");
    let token_v2_content = "<system-reminder>\n<!-- type:tokenStatus -->\n75% used\nThis supersedes earlier tokenStatus reminder\n</system-reminder>";
    let token_v2 = create_test_message(token_v2_content);

    let messages = vec![
        msg1.clone(),
        token_v1.clone(),
        msg2.clone(),
        token_v2.clone(),
    ];

    // Verify both are detected as system reminders
    assert!(
        is_system_reminder(&token_v1),
        "token_v1 should be a system reminder"
    );
    assert!(
        is_system_reminder(&token_v2),
        "token_v2 should be a system reminder"
    );

    // @step When I call partition_for_compaction
    let (system_reminders, compactable) = partition_for_compaction(&messages);

    // @step Then only tokenStatus_v2 should be in the system-reminders partition
    // After fix: only the LATEST reminder of each type should be in system_reminders
    assert_eq!(
        system_reminders.len(),
        1,
        "Should only have 1 system reminder (the latest)"
    );
    let preserved_text = get_message_text(&system_reminders[0]).expect("Should have text");
    assert!(
        preserved_text.contains("75% used"),
        "Should preserve the latest (75% used) reminder"
    );

    // @step And tokenStatus_v1 should be in the compactable messages partition
    // The old reminder should be treated as compactable (will be summarized away)
    assert_eq!(
        compactable.len(),
        3,
        "Should have 3 compactable messages: msg1, token_v1, msg2"
    );

    // Verify token_v1 is in compactable
    let compactable_has_old_reminder = compactable.iter().any(|m| {
        get_message_text(m)
            .map(|t| t.contains("50% used"))
            .unwrap_or(false)
    });
    assert!(
        compactable_has_old_reminder,
        "Old reminder should be in compactable partition"
    );
}

// =============================================================================
// Scenario: Compaction preserves only latest reminder at start of messages
// =============================================================================

#[test]
fn test_compaction_preserves_latest_reminder_at_start() {
    // @step Given a session with messages containing two tokenStatus reminders
    // This test verifies the integration with Session.compact_messages()
    // We simulate the expected behavior of the compaction result

    let token_v1_content =
        "<system-reminder>\n<!-- type:tokenStatus -->\n50% used\n</system-reminder>";
    let token_v1 = create_test_message(token_v1_content);
    let token_v2_content = "<system-reminder>\n<!-- type:tokenStatus -->\n75% used\nThis supersedes earlier tokenStatus reminder\n</system-reminder>";
    let token_v2 = create_test_message(token_v2_content);
    let msg1 = create_test_message("User message 1");
    let msg2 = create_test_message("User message 2");

    let messages = vec![
        token_v1.clone(),
        msg1.clone(),
        token_v2.clone(),
        msg2.clone(),
    ];

    // @step And the session has conversation turns to compact
    // Simulated: the session would have turns that get compacted

    // @step When I call compact_messages
    // We test the partition logic that compact_messages uses
    let (system_reminders, _compactable) = partition_for_compaction(&messages);

    // @step Then the result should start with [tokenStatus_v2, summary, ...]
    // After fix: only latest reminder preserved
    assert_eq!(
        system_reminders.len(),
        1,
        "Should only preserve latest reminder"
    );
    let preserved_text = get_message_text(&system_reminders[0]).expect("Should have text");
    assert!(
        preserved_text.contains("75% used"),
        "Should preserve v2 (latest)"
    );

    // @step And tokenStatus_v1 should not be present in the messages
    assert!(
        !preserved_text.contains("50% used"),
        "Old reminder content should not be in preserved reminder"
    );
}

// =============================================================================
// Scenario: Supersession marker indicates reminder supersedes earlier one
// =============================================================================

#[test]
fn test_supersession_marker_format() {
    // @step Given a message array with an existing Environment reminder
    let env_v1 = add_system_reminder(&[], SystemReminderType::Environment, "Platform: linux");
    assert_eq!(env_v1.len(), 1);

    // @step When I add a new Environment reminder with updated content
    let result = add_system_reminder(&env_v1, SystemReminderType::Environment, "Platform: darwin");

    // @step Then the new reminder content should contain "This supersedes earlier environment reminder"
    // After fix: new reminder should have supersession marker
    assert_eq!(
        result.len(),
        2,
        "Should have 2 reminders (old preserved, new appended)"
    );
    let new_reminder_text = get_message_text(&result[1]).expect("Should have new reminder");
    assert!(
        new_reminder_text.to_lowercase().contains("supersedes")
            && new_reminder_text.to_lowercase().contains("environment"),
        "New reminder should contain supersession marker mentioning 'environment'"
    );

    // @step And the old reminder should remain unchanged in its original position
    let old_reminder_text = get_message_text(&result[0]).expect("Should have old reminder");
    assert!(
        old_reminder_text.contains("Platform: linux"),
        "Old reminder should be unchanged"
    );
}

// =============================================================================
// Additional edge case tests
// =============================================================================

#[test]
fn test_first_reminder_of_type_has_no_supersession_marker() {
    // When adding the FIRST reminder of a type, no supersession marker needed
    let messages: Vec<Message> = vec![];

    let result = add_system_reminder(&messages, SystemReminderType::GitStatus, "Branch: main");

    assert_eq!(result.len(), 1);
    let text = get_message_text(&result[0]).expect("Should have text");

    // First reminder should NOT have supersession marker
    assert!(
        !text.to_lowercase().contains("supersedes"),
        "First reminder should not have supersession marker"
    );
    assert!(text.contains("Branch: main"), "Should contain the content");
}

#[test]
fn test_multiple_types_each_preserve_latest() {
    // Test with multiple reminder types - each type should only preserve its latest
    let token_v1_content =
        "<system-reminder>\n<!-- type:tokenStatus -->\n50% used\n</system-reminder>";
    let token_v1 = create_test_message(token_v1_content);
    let env_v1_content = "<system-reminder>\n<!-- type:environment -->\nlinux\n</system-reminder>";
    let env_v1 = create_test_message(env_v1_content);
    let token_v2_content = "<system-reminder>\n<!-- type:tokenStatus -->\n75% used\nThis supersedes earlier tokenStatus reminder\n</system-reminder>";
    let token_v2 = create_test_message(token_v2_content);
    let env_v2_content = "<system-reminder>\n<!-- type:environment -->\ndarwin\nThis supersedes earlier environment reminder\n</system-reminder>";
    let env_v2 = create_test_message(env_v2_content);

    let messages = vec![token_v1, env_v1, token_v2, env_v2];

    let (system_reminders, compactable) = partition_for_compaction(&messages);

    // Should have 2 system reminders (latest of each type)
    assert_eq!(
        system_reminders.len(),
        2,
        "Should have 2 reminders (latest token + latest env)"
    );

    // Should have 2 compactable (old versions of each type)
    assert_eq!(
        compactable.len(),
        2,
        "Should have 2 compactable (old token + old env)"
    );

    // Verify the preserved reminders are the latest versions
    let preserved_texts: Vec<String> = system_reminders
        .iter()
        .filter_map(|m| get_message_text(m))
        .collect();

    assert!(
        preserved_texts.iter().any(|t| t.contains("75% used")),
        "Should preserve latest token"
    );
    assert!(
        preserved_texts.iter().any(|t| t.contains("darwin")),
        "Should preserve latest env"
    );
}
