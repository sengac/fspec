//! Feature: System Reminder Preservation Through Compaction and Session Restore
use super::*;

// =============================================================================
// Scenario: System reminders preserved at start of messages after compaction
// =============================================================================

#[test]
fn test_system_reminders_preserved_at_start_after_compaction() {
    // @step Given a session with environment and systemPrompt system reminders
    let env_reminder = create_system_reminder_message(
        SystemReminderType::Environment,
        "Platform: linux\nArchitecture: aarch64\nWorking directory: /home/user/project",
    );
    let agents_md_reminder = create_system_reminder_message(
        SystemReminderType::SystemPrompt,
        "# Project\nThis is the AGENTS.md content",
    );

    // @step And multiple conversation turns
    let user1 = create_user_message("First user message");
    let assistant1 = create_assistant_message("First assistant response");
    let user2 = create_user_message("Second user message");
    let assistant2 = create_assistant_message("Second assistant response");

    let messages = vec![
        env_reminder,
        agents_md_reminder,
        user1,
        assistant1,
        user2,
        assistant2,
    ];

    // @step When I partition messages for compaction
    let (system_reminders, compactable) = partition_for_compaction(&messages);

    // @step Then system reminders should be extracted
    assert_eq!(
        system_reminders.len(),
        2,
        "Should extract both environment and systemPrompt reminders"
    );

    // @step And compactable messages should not include system reminders
    assert_eq!(
        compactable.len(),
        4,
        "Should have 4 compactable messages (2 user + 2 assistant)"
    );

    // @step And all system reminders should be identified correctly
    for reminder in &system_reminders {
        assert!(
            is_system_reminder(reminder),
            "Each extracted message should be a system reminder"
        );
    }

    // @step And no compactable message should be a system reminder
    for msg in &compactable {
        assert!(
            !is_system_reminder(msg),
            "Compactable messages should not be system reminders"
        );
    }
}

// =============================================================================
// Scenario: All system reminder types (environment, systemPrompt, fspecWorkflow) preserved
// =============================================================================

#[test]
fn test_all_system_reminder_types_preserved_through_compaction() {
    // @step Given a session with all three system reminder types
    let env_reminder = create_system_reminder_message(
        SystemReminderType::Environment,
        "Platform: linux\nArchitecture: aarch64",
    );
    let agents_md_reminder = create_system_reminder_message(
        SystemReminderType::SystemPrompt,
        "# fspec - Acceptance Criteria Driven Development",
    );
    // Note: fspecWorkflow is delivered via systemPrompt type with specific content
    // The actual fspec workflow guidance is injected as systemPrompt type

    // @step And multiple conversation turns
    let user1 = create_user_message("Hello");
    let assistant1 = create_assistant_message("Hi there");

    let messages = vec![env_reminder, agents_md_reminder, user1, assistant1];

    // @step When I partition for compaction
    let (system_reminders, compactable) = partition_for_compaction(&messages);

    // @step Then both environment and systemPrompt reminders should be preserved
    assert_eq!(
        system_reminders.len(),
        2,
        "Should preserve both reminder types"
    );

    // @step And environment reminder should be present
    let has_env = system_reminders
        .iter()
        .any(|msg| is_reminder_of_type(msg, "<!-- type:environment -->"));
    assert!(has_env, "Environment reminder should be preserved");

    // @step And systemPrompt reminder should be present
    let has_agents_md = system_reminders
        .iter()
        .any(|msg| is_reminder_of_type(msg, "<!-- type:systemPrompt -->"));
    assert!(has_agents_md, "SystemPrompt reminder should be preserved");

    // @step And conversation messages should be in compactable
    assert_eq!(compactable.len(), 2, "Should have 2 conversation messages");
}

// =============================================================================
// Scenario: Only latest reminder per type preserved during compaction
// =============================================================================

#[test]
fn test_only_latest_reminder_per_type_preserved() {
    // @step Given a session where environment reminder was updated (old + new)
    let env_old =
        create_system_reminder_message(SystemReminderType::Environment, "OLD: Platform: darwin");
    let user1 = create_user_message("Some conversation");
    let assistant1 = create_assistant_message("Response");
    // When environment is updated, a new reminder is appended (old preserved for cache)
    let env_messages = add_system_reminder(
        std::slice::from_ref(&env_old),
        SystemReminderType::Environment,
        "NEW: Platform: linux",
    );
    // env_messages now has [env_old, env_new]

    let messages = vec![
        env_messages[0].clone(), // old
        user1,
        assistant1,
        env_messages[1].clone(), // new (with supersession marker)
    ];

    // @step When I partition for compaction
    let (system_reminders, compactable) = partition_for_compaction(&messages);

    // @step Then only the latest environment reminder should be preserved
    assert_eq!(
        system_reminders.len(),
        1,
        "Should only preserve latest reminder"
    );

    // @step And it should be the NEW one
    let preserved_text = get_message_text(&system_reminders[0]).expect("Should have text");
    assert!(
        preserved_text.contains("NEW: Platform: linux"),
        "Should preserve the latest (NEW) reminder"
    );

    // @step And old reminder should be in compactable (will be summarized away)
    assert_eq!(
        compactable.len(),
        3,
        "Should have old reminder + 2 conversation messages"
    );
    let old_in_compactable = compactable.iter().any(|msg| {
        get_message_text(msg)
            .map(|t| t.contains("OLD: Platform: darwin"))
            .unwrap_or(false)
    });
    assert!(
        old_in_compactable,
        "Old reminder should be in compactable partition"
    );
}
