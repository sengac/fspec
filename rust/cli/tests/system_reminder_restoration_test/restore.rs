//! Feature: System Reminder Preservation Through Compaction and Session Restore
use super::*;

// =============================================================================
// Scenario: System reminders skipped during restore (fresh ones already injected)
// =============================================================================

#[test]
fn test_system_reminders_should_be_identifiable_for_skip_during_restore() {
    // @step Given a message that looks like a system reminder
    let env_reminder = create_system_reminder_message(
        SystemReminderType::Environment,
        "Platform: linux\nWorking directory: /old/path",
    );

    // @step When checking if it's a system reminder
    let text = get_message_text(&env_reminder).expect("Should have text");

    // @step Then it should have both required markers (for skip logic during restore)
    // The restore logic checks: contains("<system-reminder>") && contains("<!-- type:")
    assert!(
        text.contains("<system-reminder>"),
        "Should have system-reminder tag"
    );
    assert!(text.contains("<!-- type:"), "Should have type marker");

    // This is exactly what session_restore_messages checks to skip old reminders
    let should_skip = text.contains("<system-reminder>") && text.contains("<!-- type:");
    assert!(
        should_skip,
        "System reminder should be identifiable for skip during restore"
    );
}

// =============================================================================
// Scenario: Simulated compaction flow preserves reminders correctly
// =============================================================================

#[test]
fn test_simulated_compaction_flow_preserves_reminders() {
    // This simulates what execute_compaction does

    // @step Given a session with system reminders and conversation
    let env_reminder = create_system_reminder_message(
        SystemReminderType::Environment,
        "Platform: linux\nArchitecture: aarch64\nShell: /bin/bash\nUser: testuser\nWorking directory: /home/testuser/project",
    );
    let agents_md_reminder = create_system_reminder_message(
        SystemReminderType::SystemPrompt,
        "# My Project\nThis is AGENTS.md content for the project",
    );

    // Conversation turns
    let user1 = create_user_message("Please read the README");
    let assistant1 = create_assistant_message("I'll read the README file for you.");
    let user2 = create_user_message("Now update the config");
    let assistant2 = create_assistant_message("I've updated the configuration.");
    let user3 = create_user_message("Run the tests");
    let assistant3 = create_assistant_message("Tests are passing.");

    let original_messages = vec![
        env_reminder,
        agents_md_reminder,
        user1,
        assistant1,
        user2,
        assistant2,
        user3,
        assistant3,
    ];

    // @step When I perform compaction (partition + reconstruct)
    let (system_reminders, _compactable) = partition_for_compaction(&original_messages);

    // Simulate reconstruction (what execute_compaction does)
    let mut reconstructed: Vec<Message> = Vec::new();

    // Add system reminders FIRST (maintains stable prefix for prompt caching)
    reconstructed.extend(system_reminders);

    // Add a simulated summary (would come from LLM in real compaction)
    reconstructed.push(create_user_message(
        "Summary: User asked to read README, update config, and run tests. All completed.",
    ));

    // Add continuation message
    reconstructed.push(create_user_message(
        "This session is being continued from a previous conversation that ran out of context.",
    ));

    // @step Then system reminders should be at the START of reconstructed messages
    assert!(
        is_system_reminder(&reconstructed[0]),
        "First message should be a system reminder"
    );
    assert!(
        is_system_reminder(&reconstructed[1]),
        "Second message should be a system reminder"
    );

    // @step And environment reminder should be first or second
    let env_present = reconstructed[..2]
        .iter()
        .any(|msg| is_reminder_of_type(msg, "<!-- type:environment -->"));
    assert!(
        env_present,
        "Environment reminder should be in first 2 messages"
    );

    // @step And systemPrompt reminder should be first or second
    let agents_md_present = reconstructed[..2]
        .iter()
        .any(|msg| is_reminder_of_type(msg, "<!-- type:systemPrompt -->"));
    assert!(
        agents_md_present,
        "SystemPrompt reminder should be in first 2 messages"
    );

    // @step And the summary should come AFTER system reminders
    let summary_idx = reconstructed
        .iter()
        .position(|msg| {
            get_message_text(msg)
                .map(|t| t.contains("Summary:"))
                .unwrap_or(false)
        })
        .expect("Should have summary message");
    assert!(
        summary_idx >= 2,
        "Summary should be after system reminders (idx={summary_idx})"
    );

    // @step And total reconstructed should be smaller than original
    assert!(
        reconstructed.len() < original_messages.len(),
        "Reconstructed should be smaller: {} < {}",
        reconstructed.len(),
        original_messages.len()
    );
}
