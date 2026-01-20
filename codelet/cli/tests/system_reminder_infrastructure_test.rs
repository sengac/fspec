/**
 * Feature: spec/features/implement-system-reminder-infrastructure.feature
 *
 * Comprehensive tests for system-reminder infrastructure that persists through compaction.
 * System-reminders are Messages added to the messages array but explicitly
 * EXCLUDED from compaction - they stay in position while messages around them are compacted.
 *
 * This file combines:
 * - Session integration tests (add_system_reminder method)
 * - Helper function tests (is_system_reminder, partition_for_compaction)
 * - Full compaction persistence test (ignored - requires real LLM)
 */
use codelet_cli::session::{
    system_reminders::{is_system_reminder, partition_for_compaction, SystemReminderType},
    Session,
};
use rig::message::{AssistantContent, Message, UserContent};
use rig::OneOrMany;

// ============================================================================
// Session Integration Tests
// ============================================================================

#[test]
fn test_add_system_reminder_to_messages_array() {
    // Feature: spec/features/implement-system-reminder-infrastructure.feature
    // Scenario: Add system-reminder to messages array

    // @step Given I have a Session with empty messages Vec
    let mut session = Session::new(None).expect("Failed to create session");
    assert_eq!(
        session.messages.len(),
        0,
        "Session should start with empty messages"
    );

    // @step When I call add_system_reminder(tokenStatus, "50% tokens used")
    session.add_system_reminder(SystemReminderType::TokenStatus, "50% tokens used");

    // @step Then the messages Vec should contain 1 message
    assert_eq!(
        session.messages.len(),
        1,
        "Messages Vec should contain 1 message"
    );

    // @step And the message should be user role with content wrapped in <system-reminder> tags
    if let Message::User { content } = &session.messages[0] {
        if let UserContent::Text(t) = content.first() {
            let text = &t.text;
            assert!(
                text.contains("<system-reminder>"),
                "Content should contain <system-reminder> tag"
            );
            assert!(
                text.contains("</system-reminder>"),
                "Content should contain </system-reminder> tag"
            );

            // @step And the message content should include type marker "<!-- type:tokenStatus -->"
            assert!(
                text.contains("<!-- type:tokenStatus -->"),
                "Content should include type marker"
            );

            // @step And the message content should include "50% tokens used"
            assert!(
                text.contains("50% tokens used"),
                "Content should include the reminder text"
            );
        } else {
            panic!("Expected text content");
        }
    } else {
        panic!("Expected User message");
    }
}

#[test]
fn test_multiple_reminder_types_coexist() {
    // Feature: spec/features/implement-system-reminder-infrastructure.feature
    // Scenario: Multiple reminder types coexist in messages array

    // @step Given I have a Session with empty messages Vec
    let mut session = Session::new(None).expect("Failed to create session");

    // @step When I call add_system_reminder(environment, "Platform: linux")
    session.add_system_reminder(SystemReminderType::Environment, "Platform: linux");

    // @step And I call add_system_reminder(tokenStatus, "60% tokens used")
    session.add_system_reminder(SystemReminderType::TokenStatus, "60% tokens used");

    // @step Then the messages Vec should contain 2 system-reminder messages
    assert_eq!(
        session.messages.len(),
        2,
        "Messages Vec should contain 2 messages"
    );

    // Note: add_system_reminder APPENDS to preserve prompt cache prefix stability
    // First added (environment) is at index 0, later added (tokenStatus) at index 1

    // @step And the first system-reminder should have type marker "<!-- type:environment -->" (append order)
    if let Message::User { content } = &session.messages[0] {
        if let UserContent::Text(t) = content.first() {
            assert!(
                t.text.contains("<!-- type:environment -->"),
                "First should be environment type (appended first)"
            );
        } else {
            panic!("Expected text content");
        }
    } else {
        panic!("Expected User message");
    }

    // @step And the second system-reminder should have type marker "<!-- type:tokenStatus -->" (append order)
    if let Message::User { content } = &session.messages[1] {
        if let UserContent::Text(t) = content.first() {
            assert!(
                t.text.contains("<!-- type:tokenStatus -->"),
                "Second should be tokenStatus type (appended last)"
            );
        } else {
            panic!("Expected text content");
        }
    } else {
        panic!("Expected User message");
    }
}

#[test]
fn test_deduplication_via_retain_and_push() {
    // Feature: spec/features/implement-system-reminder-infrastructure.feature
    // Scenario: Deduplication via retain and push pattern
    //
    // CLI-013 UPDATE: Deduplication now happens ONLY during compaction, not during add.
    // During normal operation, old reminders are PRESERVED to maintain prompt cache prefix.
    // New reminders are APPENDED with supersession markers.

    // @step Given I have a Session with messages: [user_msg_1, system-reminder(tokenStatus, "50% used")]
    let mut session = Session::new(None).expect("Failed to create session");

    // Add user message
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("user message 1")),
    });

    // Add tokenStatus reminder
    session.add_system_reminder(SystemReminderType::TokenStatus, "50% used");

    assert_eq!(
        session.messages.len(),
        2,
        "Should have 2 messages initially"
    );

    // @step When I call add_system_reminder(tokenStatus, "60% used")
    session.add_system_reminder(SystemReminderType::TokenStatus, "60% used");

    // @step Then the messages Vec should contain 2 tokenStatus system-reminders (old preserved + new appended)
    // CLI-013: Old reminder is preserved for prompt cache stability
    let token_status_count = session
        .messages
        .iter()
        .filter(|msg| match msg {
            Message::User { content } => match content.first() {
                UserContent::Text(t) => t.text.contains("<!-- type:tokenStatus -->"),
                _ => false,
            },
            _ => false,
        })
        .count();

    assert_eq!(
        token_status_count, 2,
        "CLI-013: Should have 2 tokenStatus reminders (old preserved, new appended)"
    );
    assert_eq!(
        session.messages.len(),
        3,
        "CLI-013: Total messages should be 3 (user msg + old reminder + new reminder)"
    );

    // @step And the new tokenStatus system-reminder content should contain "60% used" with supersession marker
    let has_new_content_with_supersession = session.messages.iter().any(|msg| match msg {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => {
                t.text.contains("<!-- type:tokenStatus -->")
                    && t.text.contains("60% used")
                    && t.text.contains("supersedes")
            }
            _ => false,
        },
        _ => false,
    });

    assert!(
        has_new_content_with_supersession,
        "CLI-013: New reminder should have content and supersession marker"
    );

    // @step And the old "50% used" tokenStatus should still be present (for prompt cache)
    let has_old_content = session.messages.iter().any(|msg| match msg {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => t.text.contains("50% used"),
            _ => false,
        },
        _ => false,
    });

    assert!(
        has_old_content,
        "CLI-013: Old tokenStatus reminder should be preserved for prompt cache"
    );
}

// ============================================================================
// Helper Function Tests (Compaction Support)
// ============================================================================

#[test]
fn test_is_system_reminder_identifies_reminder_messages() {
    // Feature: spec/features/implement-system-reminder-infrastructure.feature
    // Scenario: Identify system-reminder messages

    // @step Given I have a message with system-reminder tags and type marker
    let reminder_msg = Message::User {
        content: OneOrMany::one(UserContent::text(
            "<system-reminder>\n<!-- type:tokenStatus -->\n50% tokens used\n</system-reminder>",
        )),
    };

    // @step When I check if it's a system-reminder
    let is_reminder = is_system_reminder(&reminder_msg);

    // @step Then it should be identified as a system-reminder
    assert!(is_reminder, "Should identify system-reminder message");

    // @step Given I have a regular user message without system-reminder tags
    let regular_msg = Message::User {
        content: OneOrMany::one(UserContent::text("Hello, how are you?")),
    };

    // @step When I check if it's a system-reminder
    let is_regular = is_system_reminder(&regular_msg);

    // @step Then it should NOT be identified as a system-reminder
    // @step And regular user messages should NOT be identified as system-reminders
    assert!(
        !is_regular,
        "Regular message should not be identified as system-reminder"
    );

    // @step Given I have an assistant message
    let assistant_msg = Message::Assistant {
        id: Some("1".to_string()),
        content: OneOrMany::one(AssistantContent::text("I'm doing well!")),
    };

    // @step When I check if it's a system-reminder
    let is_assistant = is_system_reminder(&assistant_msg);

    // @step Then it should NOT be identified as a system-reminder
    // @step And assistant messages should NOT be identified as system-reminders
    assert!(
        !is_assistant,
        "Assistant message should not be system-reminder"
    );
}

#[test]
fn test_partition_for_compaction_separates_reminders() {
    // Feature: spec/features/implement-system-reminder-infrastructure.feature
    // Scenario: Partition messages for compaction

    // @step Given I have a Session with mixed messages: [user_msg, system-reminder(env), assistant_msg, system-reminder(token)]
    let mut session = Session::new(None).expect("Failed to create session");

    // Add regular user message
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("What's the weather?")),
    });

    // Add system-reminder
    session.add_system_reminder(SystemReminderType::Environment, "Platform: linux");

    // Add assistant message
    session.messages.push(Message::Assistant {
        id: Some("1".to_string()),
        content: OneOrMany::one(AssistantContent::text("Let me check the weather for you.")),
    });

    // Add another system-reminder
    session.add_system_reminder(SystemReminderType::TokenStatus, "60% tokens used");

    // Add another regular message
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Thanks!")),
    });

    assert_eq!(session.messages.len(), 5, "Should have 5 total messages");

    // @step When I partition messages using partition_for_compaction helper
    let (system_reminders, compactable) = partition_for_compaction(&session.messages);

    // @step Then system-reminders should be in separate partition
    assert_eq!(system_reminders.len(), 2, "Should have 2 system-reminders");

    // @step And compactable messages should be in separate partition
    assert_eq!(compactable.len(), 3, "Should have 3 compactable messages");

    // @step And all system-reminders should have type markers
    for reminder in &system_reminders {
        assert!(
            is_system_reminder(reminder),
            "All reminders should be identified as system-reminders"
        );
    }

    // @step And no compactable messages should be system-reminders
    for msg in &compactable {
        assert!(
            !is_system_reminder(msg),
            "Compactable messages should not be system-reminders"
        );
    }
}

#[test]
fn test_reconstruction_after_compaction_preserves_reminders() {
    // Feature: spec/features/implement-system-reminder-infrastructure.feature
    // Scenario: Reconstruct messages after compaction preserves reminders

    // @step Given I have messages partitioned into system-reminders and compactable
    let mut session = Session::new(None).expect("Failed to create session");

    // Setup: 3 old messages + 1 system-reminder + 2 recent messages
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("old message 1")),
    });
    session.messages.push(Message::Assistant {
        id: Some("1".to_string()),
        content: OneOrMany::one(AssistantContent::text("old response 1")),
    });
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("old message 2")),
    });

    session.add_system_reminder(SystemReminderType::Environment, "Platform: linux");

    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("recent message")),
    });
    session.messages.push(Message::Assistant {
        id: Some("2".to_string()),
        content: OneOrMany::one(AssistantContent::text("recent response")),
    });

    let initial_count = session.messages.len();
    assert_eq!(initial_count, 6, "Should start with 6 messages");

    // @step When I partition for compaction
    let (system_reminders, compactable) = partition_for_compaction(&session.messages);

    assert_eq!(system_reminders.len(), 1, "Should have 1 system-reminder");
    assert_eq!(compactable.len(), 5, "Should have 5 compactable messages");

    // @step When I simulate compaction by replacing compactable with summary
    // Add summary (simulating LLM summarization)
    let mut reconstructed = vec![];
    reconstructed.push(Message::User {
        content: OneOrMany::one(UserContent::text("Summary of old messages")),
    });

    // @step And I reconstruct: summary + system-reminders + recent messages
    // Add system-reminders back
    reconstructed.extend(system_reminders);

    // Add recent messages (last 2 compactable messages)
    reconstructed.extend(compactable[3..].to_vec());

    // @step Then the reconstructed messages should contain the summary
    let has_summary = reconstructed.iter().any(|msg| match msg {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => t.text.contains("Summary"),
            _ => false,
        },
        _ => false,
    });
    assert!(has_summary, "Should have summary message");

    // @step And the reconstructed messages should contain system-reminders
    let has_env_reminder = reconstructed.iter().any(is_system_reminder);
    assert!(has_env_reminder, "Should have system-reminder");

    // @step And system-reminder content should be unchanged
    let env_content = reconstructed.iter().find_map(|msg| match msg {
        Message::User { content } => match content.first() {
            UserContent::Text(t) if is_system_reminder(msg) => Some(t.text.clone()),
            _ => None,
        },
        _ => None,
    });
    assert!(env_content.is_some(), "Should find system-reminder");
    assert!(
        env_content.unwrap().contains("Platform: linux"),
        "System-reminder content should be unchanged"
    );

    // @step And the reconstructed messages should contain recent messages
    let has_recent = reconstructed.iter().any(|msg| match msg {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => t.text.contains("recent message"),
            _ => false,
        },
        _ => false,
    });
    assert!(has_recent, "Should have recent messages");

    // @step And the message count should be reduced (compaction happened)
    assert!(
        reconstructed.len() < initial_count,
        "Message count should be reduced: {} < {}",
        reconstructed.len(),
        initial_count
    );
}

// ============================================================================
// Full Integration Test (Ignored - Requires Real LLM)
// ============================================================================

#[tokio::test]
async fn test_system_reminders_persist_through_compaction() {
    // Feature: spec/features/implement-system-reminder-infrastructure.feature
    // Scenario: System-reminders persist through compaction

    use codelet_core::compaction::ConversationTurn;
    use std::time::SystemTime;

    // @step Given I have a Session with messages: [user_msg_1, assistant_msg_1, system-reminder(environment), user_msg_2, assistant_msg_2]
    let mut session = Session::new(None).expect("Failed to create session");

    // Add user and assistant messages
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("user message 1")),
    });
    session.messages.push(Message::Assistant {
        id: Some("1".to_string()),
        content: OneOrMany::one(AssistantContent::text("assistant response 1")),
    });

    // Add system-reminder
    session.add_system_reminder(SystemReminderType::Environment, "Platform: linux");

    // Add more messages
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("user message 2")),
    });
    session.messages.push(Message::Assistant {
        id: Some("2".to_string()),
        content: OneOrMany::one(AssistantContent::text("assistant response 2")),
    });

    // @step And the Session has turns tracking the conversation
    // Add turns with enough tokens to trigger compaction (need substantial content)
    let long_message_1 = "This is a long user message that contains enough content to meet the minimum token requirement for compaction. ".repeat(10);
    let long_response_1 = "This is a long assistant response with detailed information that provides substantial content for the compaction process. ".repeat(10);

    session.turns.push(ConversationTurn {
        user_message: long_message_1.clone(),
        tool_calls: vec![],
        tool_results: vec![],
        assistant_response: long_response_1.clone(),
        tokens: 2000, // Original content has 2000 tokens
        timestamp: SystemTime::now(),
        previous_error: None,
    });

    // Add multiple turns to ensure compaction happens
    for i in 2..=5 {
        session.turns.push(ConversationTurn {
            user_message: format!("user message {}", i),
            tool_calls: vec![],
            tool_results: vec![],
            assistant_response: format!("assistant response {}", i),
            tokens: 300, // More tokens per turn
            timestamp: SystemTime::now(),
            previous_error: None,
        });
    }

    let initial_count = session.messages.len();
    let initial_turn_count = session.turns.len();
    assert_eq!(initial_count, 5, "Should have 5 messages before compaction");
    assert_eq!(
        initial_turn_count, 5,
        "Should have 5 turns before compaction"
    );

    // Verify system-reminder exists before compaction
    let has_env_before = session.messages.iter().any(|msg| match msg {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => t.text.contains("<!-- type:environment -->"),
            _ => false,
        },
        _ => false,
    });
    assert!(
        has_env_before,
        "Environment reminder should exist before compaction"
    );

    // @step When I perform compaction that summarizes user_msg_1 and assistant_msg_1
    // Mock LLM summarizer - returns shorter summary to meet compression ratio
    let mock_summarizer = |_prompt: String| async move {
        // Return brief summary to achieve >60% compression (10 chars vs ~2000 tokens)
        Ok("Brief".to_string())
    };

    // Use Session.compact_messages() which automatically preserves system-reminders
    session
        .compact_messages(mock_summarizer)
        .await
        .expect("Compaction should succeed");

    // @step Then the messages Vec should contain the compacted summary
    // Note: The compactor now uses template-based summaries (not LLM-generated)
    // so we check for "Key outcomes" which is always in the template
    let has_summary = session.messages.iter().any(|msg| match msg {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => t.text.contains("Key outcomes"), // Template-based summary
            _ => false,
        },
        _ => false,
    });
    assert!(has_summary, "Messages should contain compacted summary");

    // @step And the system-reminder(environment) should still exist in messages
    let has_env_after = session.messages.iter().any(|msg| match msg {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => t.text.contains("<!-- type:environment -->"),
            _ => false,
        },
        _ => false,
    });
    assert!(
        has_env_after,
        "Environment reminder should persist after compaction"
    );

    // @step And the system-reminder should be at the START of messages (for prompt caching)
    // This is critical: the prefix must be stable for LLM prompt caching to work
    let first_msg_is_reminder = match &session.messages[0] {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => t.text.contains("<!-- type:environment -->"),
            _ => false,
        },
        _ => false,
    };
    assert!(
        first_msg_is_reminder,
        "System-reminder must be at index 0 for prompt cache prefix stability"
    );

    // @step And the system-reminder content should be unchanged
    let env_content = session.messages.iter().find_map(|msg| match msg {
        Message::User { content } => match content.first() {
            UserContent::Text(t) if t.text.contains("<!-- type:environment -->") => {
                Some(t.text.clone())
            }
            _ => None,
        },
        _ => None,
    });
    assert!(env_content.is_some(), "Should find environment reminder");
    assert!(
        env_content.unwrap().contains("Platform: linux"),
        "Environment content should be unchanged"
    );

    // @step And recent messages should be preserved
    // Some recent turns should be in the kept_turns and reconstructed as messages
    let has_recent_user = session.messages.iter().any(|msg| match msg {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => t.text.contains("user message"),
            _ => false,
        },
        _ => false,
    });
    let has_recent_assistant = session.messages.iter().any(|msg| match msg {
        Message::Assistant { content, .. } => match content.first() {
            AssistantContent::Text(t) => t.text.contains("assistant response"),
            _ => false,
        },
        _ => false,
    });
    assert!(has_recent_user, "Recent user messages should be preserved");
    assert!(
        has_recent_assistant,
        "Recent assistant messages should be preserved"
    );

    // @step And the message count should reflect compaction (summary + reminders + kept turns)
    // We should have: 1 summary + 1 system-reminder + (kept turns * 2 messages per turn)
    // With 5 turns and keeping recent ones, we expect fewer than 5*2 + 2 original messages
    let expected_max = (initial_turn_count * 2) + 2; // 2 messages per turn + summary + reminder
    assert!(
        session.messages.len() < expected_max,
        "Message count should be reduced after compaction: {} < {}",
        session.messages.len(),
        expected_max
    );
}
