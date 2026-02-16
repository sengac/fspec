#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: System Reminder Preservation Through Compaction and Session Restore
//!
//! These tests verify that system reminders (environment, claudeMd, fspecWorkflow)
//! are properly preserved through compaction and restored on session resume.
//!
//! CRITICAL BEHAVIOR:
//! 1. On session creation: inject_context_reminders() adds fresh reminders
//! 2. During compaction: partition_for_compaction() preserves reminders at start
//! 3. On session restore: Old reminders skipped (fresh ones injected on creation)
//!
//! This ensures the LLM always has access to:
//! - Platform/architecture/shell/user/working directory (environment)
//! - Project documentation (claudeMd)
//! - Workflow guidance (fspecWorkflow)

use codelet_cli::session::system_reminders::{
    add_system_reminder, is_system_reminder, partition_for_compaction, SystemReminderType,
};
use rig::message::{AssistantContent, Message, UserContent};
use rig::OneOrMany;

/// Helper to create a user message
fn create_user_message(content: &str) -> Message {
    Message::User {
        content: OneOrMany::one(UserContent::text(content)),
    }
}

/// Helper to create an assistant message
fn create_assistant_message(content: &str) -> Message {
    Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::text(content)),
    }
}

/// Helper to create a system reminder message with proper format
fn create_system_reminder_message(reminder_type: SystemReminderType, content: &str) -> Message {
    let messages: Vec<Message> = vec![];
    let result = add_system_reminder(&messages, reminder_type, content);
    result.into_iter().next().unwrap()
}

/// Helper to check if a message is a system reminder of a specific type
fn is_reminder_of_type(msg: &Message, type_marker: &str) -> bool {
    match msg {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => {
                t.text.contains("<system-reminder>") && t.text.contains(type_marker)
            }
            _ => false,
        },
        _ => false,
    }
}

/// Helper to extract text from a message
fn get_message_text(msg: &Message) -> Option<String> {
    match msg {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => Some(t.text.clone()),
            _ => None,
        },
        Message::Assistant { content, .. } => match content.first() {
            AssistantContent::Text(t) => Some(t.text.clone()),
            _ => None,
        },
    }
}

// =============================================================================
// Scenario: System reminders preserved at start of messages after compaction
// =============================================================================

#[test]
fn test_system_reminders_preserved_at_start_after_compaction() {
    // @step Given a session with environment and claudeMd system reminders
    let env_reminder = create_system_reminder_message(
        SystemReminderType::Environment,
        "Platform: linux\nArchitecture: aarch64\nWorking directory: /home/user/project",
    );
    let claude_md_reminder = create_system_reminder_message(
        SystemReminderType::ClaudeMd,
        "# Project\nThis is the CLAUDE.md content",
    );

    // @step And multiple conversation turns
    let user1 = create_user_message("First user message");
    let assistant1 = create_assistant_message("First assistant response");
    let user2 = create_user_message("Second user message");
    let assistant2 = create_assistant_message("Second assistant response");

    let messages = vec![
        env_reminder,
        claude_md_reminder,
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
        "Should extract both environment and claudeMd reminders"
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
// Scenario: All system reminder types (environment, claudeMd, fspecWorkflow) preserved
// =============================================================================

#[test]
fn test_all_system_reminder_types_preserved_through_compaction() {
    // @step Given a session with all three system reminder types
    let env_reminder = create_system_reminder_message(
        SystemReminderType::Environment,
        "Platform: linux\nArchitecture: aarch64",
    );
    let claude_md_reminder = create_system_reminder_message(
        SystemReminderType::ClaudeMd,
        "# fspec - Acceptance Criteria Driven Development",
    );
    // Note: fspecWorkflow is delivered via claudeMd type with specific content
    // The actual fspec workflow guidance is injected as claudeMd type

    // @step And multiple conversation turns
    let user1 = create_user_message("Hello");
    let assistant1 = create_assistant_message("Hi there");

    let messages = vec![env_reminder, claude_md_reminder, user1, assistant1];

    // @step When I partition for compaction
    let (system_reminders, compactable) = partition_for_compaction(&messages);

    // @step Then both environment and claudeMd reminders should be preserved
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

    // @step And claudeMd reminder should be present
    let has_claude_md = system_reminders
        .iter()
        .any(|msg| is_reminder_of_type(msg, "<!-- type:claudeMd -->"));
    assert!(has_claude_md, "ClaudeMd reminder should be preserved");

    // @step And conversation messages should be in compactable
    assert_eq!(compactable.len(), 2, "Should have 2 conversation messages");
}

// =============================================================================
// Scenario: Only latest reminder per type preserved during compaction
// =============================================================================

#[test]
fn test_only_latest_reminder_per_type_preserved() {
    // @step Given a session where environment reminder was updated (old + new)
    let env_old = create_system_reminder_message(
        SystemReminderType::Environment,
        "OLD: Platform: darwin",
    );
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
    let should_skip =
        text.contains("<system-reminder>") && text.contains("<!-- type:");
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
    let claude_md_reminder = create_system_reminder_message(
        SystemReminderType::ClaudeMd,
        "# My Project\nThis is CLAUDE.md content for the project",
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
        claude_md_reminder,
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
    reconstructed.extend(system_reminders.clone());

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
    let env_present = reconstructed[..2].iter().any(|msg| {
        is_reminder_of_type(msg, "<!-- type:environment -->")
    });
    assert!(env_present, "Environment reminder should be in first 2 messages");

    // @step And claudeMd reminder should be first or second
    let claude_md_present = reconstructed[..2].iter().any(|msg| {
        is_reminder_of_type(msg, "<!-- type:claudeMd -->")
    });
    assert!(claude_md_present, "ClaudeMd reminder should be in first 2 messages");

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
        "Summary should be after system reminders (idx={})",
        summary_idx
    );

    // @step And total reconstructed should be smaller than original
    assert!(
        reconstructed.len() < original_messages.len(),
        "Reconstructed should be smaller: {} < {}",
        reconstructed.len(),
        original_messages.len()
    );
}

// =============================================================================
// Scenario: Multiple reminder types each preserve only latest
// =============================================================================

#[test]
fn test_multiple_reminder_types_each_preserve_only_latest() {
    // @step Given a session where both env and claudeMd were updated
    let env_old = create_system_reminder_message(
        SystemReminderType::Environment,
        "OLD ENV",
    );
    let claude_old = create_system_reminder_message(
        SystemReminderType::ClaudeMd,
        "OLD CLAUDE.md",
    );
    let user1 = create_user_message("conversation");
    let env_new_messages = add_system_reminder(
        std::slice::from_ref(&env_old),
        SystemReminderType::Environment,
        "NEW ENV",
    );
    let claude_new_messages = add_system_reminder(
        std::slice::from_ref(&claude_old),
        SystemReminderType::ClaudeMd,
        "NEW CLAUDE.md",
    );

    // Build message list with old and new reminders interleaved
    let messages = vec![
        env_old,
        claude_old,
        user1,
        env_new_messages[1].clone(),     // new env
        claude_new_messages[1].clone(),  // new claude
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
    let env_reminder = create_system_reminder_message(
        SystemReminderType::Environment,
        "Platform: linux",
    );
    let claude_md_reminder = create_system_reminder_message(
        SystemReminderType::ClaudeMd,
        "# Project",
    );

    let messages = vec![env_reminder, claude_md_reminder];

    // @step When I partition for compaction
    let (system_reminders, compactable) = partition_for_compaction(&messages);

    // @step Then all messages should be in system_reminders
    assert_eq!(
        system_reminders.len(),
        2,
        "Both reminders should be preserved"
    );

    // @step And compactable should be empty
    assert!(
        compactable.is_empty(),
        "No conversation to compact"
    );
}
