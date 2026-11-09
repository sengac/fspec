//! Feature: System Reminder Preservation Through Compaction and Session Restore
use super::*;

// =============================================================================
// Scenario: Multiple reminder types each preserve only latest
// =============================================================================

#[test]
fn test_multiple_reminder_types_each_preserve_only_latest() {
    // @step Given a session where both env and systemPrompt were updated
    let env_old = create_system_reminder_message(SystemReminderType::Environment, "OLD ENV");
    let claude_old =
        create_system_reminder_message(SystemReminderType::SystemPrompt, "OLD AGENTS.md");
    let user1 = create_user_message("conversation");
    let env_new_messages = add_system_reminder(
        std::slice::from_ref(&env_old),
        SystemReminderType::Environment,
        "NEW ENV",
    );
    let claude_new_messages = add_system_reminder(
        std::slice::from_ref(&claude_old),
        SystemReminderType::SystemPrompt,
        "NEW AGENTS.md",
    );

    // Build message list with old and new reminders interleaved
    let messages = vec![
        env_old,
        claude_old,
        user1,
        env_new_messages[1].clone(),    // new env
        claude_new_messages[1].clone(), // new claude
    ];

    // @step When I partition for compaction
    let (system_reminders, compactable) = partition_for_compaction(&messages);

    // @step Then should have 2 reminders (latest of each type)
    assert_eq!(
        system_reminders.len(),
        2,
        "Should preserve latest of each type"
    );

    // @step And both should be the NEW versions
    for reminder in &system_reminders {
        let text = get_message_text(reminder).expect("Should have text");
        assert!(
            text.contains("NEW"),
            "Should be NEW version: {}",
            text.chars().take(100).collect::<String>()
        );
    }

    // @step And old versions should be in compactable
    assert_eq!(
        compactable.len(),
        3,
        "Should have 2 old reminders + 1 conversation message"
    );
}

// =============================================================================
// Scenario: Empty session handles gracefully
// =============================================================================

#[test]
fn test_empty_session_partition_handles_gracefully() {
    // @step Given an empty message list
    let messages: Vec<Message> = vec![];

    // @step When I partition for compaction
    let (system_reminders, compactable) = partition_for_compaction(&messages);

    // @step Then both partitions should be empty
    assert!(
        system_reminders.is_empty(),
        "No system reminders in empty session"
    );
    assert!(compactable.is_empty(), "No compactable in empty session");
}

// =============================================================================
// Scenario: Session with only reminders (no conversation) handles correctly
// =============================================================================

#[test]
fn test_session_with_only_reminders_partition_correctly() {
    // @step Given a fresh session with only system reminders (just created, no conversation yet)
    let env_reminder =
        create_system_reminder_message(SystemReminderType::Environment, "Platform: linux");
    let agents_md_reminder =
        create_system_reminder_message(SystemReminderType::SystemPrompt, "# Project");

    let messages = vec![env_reminder, agents_md_reminder];

    // @step When I partition for compaction
    let (system_reminders, compactable) = partition_for_compaction(&messages);

    // @step Then all messages should be in system_reminders
    assert_eq!(
        system_reminders.len(),
        2,
        "Both reminders should be preserved"
    );

    // @step And compactable should be empty
    assert!(compactable.is_empty(), "No conversation to compact");
}
