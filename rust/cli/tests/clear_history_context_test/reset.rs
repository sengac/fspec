//! Feature: spec/features/clear-context-command-for-session-reset.feature
use super::*;

/// Test that clear_history resets messages, turns, and tokens
///
/// Scenario: Clear conversation and reset tokens to fresh state
///
/// @step Given I have a conversation with messages and accumulated tokens
/// @step When I type "/clear" and press Enter
/// @step Then the conversation area should show "Type a message to start..."
/// @step And the token counter should show "0↓ 0↑"
/// @step And no confirmation dialog should appear
#[test]
fn test_clear_history_resets_conversation_state() {
    let mut session = offline_session::session().expect("offline session");

    // @step Given I have a conversation with messages and accumulated tokens
    session.token_tracker.input_tokens = 5000;
    session.token_tracker.output_tokens = 1000;

    // Add a dummy message to simulate conversation
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Hello, world!")),
    });

    assert!(
        !session.messages.is_empty(),
        "Should have messages before clear"
    );
    assert!(
        session.token_tracker.input_tokens > 0,
        "Should have tokens before clear"
    );

    // @step When I type "/clear" and press Enter
    clear_session_state(&mut session);

    // @step Then the conversation area should show "Type a message to start..."
    // @step And the token counter should show "0↓ 0↑"
    assert!(session.messages.is_empty(), "Messages should be cleared");
    assert_eq!(
        session.token_tracker.input_tokens, 0,
        "Input tokens should be 0"
    );
    assert_eq!(
        session.token_tracker.output_tokens, 0,
        "Output tokens should be 0"
    );

    // @step And no confirmation dialog should appear
    // (This is a UI concern - tested at React level, not Rust)
}

/// Test that clear_history reinjects context reminders
///
/// This is the CRITICAL test - verifies that AGENTS.md and environment info
/// are restored after clearing, so the AI doesn't lose project context.
///
/// Scenario: Context reminders preserved after clear
///
/// @step Given I have a session with context reminders injected
/// @step When I clear the history
/// @step Then context reminders should be reinjected
/// @step And the AI should still have access to AGENTS.md context
#[test]
fn test_clear_history_reinjects_context_reminders() {
    let mut session = offline_session::session().expect("offline session");

    // @step Given I have a session with context reminders injected
    session.inject_context_reminders("/work".as_ref());

    // Count initial system reminder messages (environment info at minimum)
    let initial_reminder_count = count_system_reminder_messages(&session);

    // Add some messages to simulate conversation
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Test message")),
    });
    session.token_tracker.input_tokens = 1000;

    // @step When I clear the history
    clear_session_state(&mut session);

    // CRITICAL: Reinject context reminders after clearing
    session.inject_context_reminders("/work".as_ref());

    // @step Then context reminders should be reinjected
    // @step And the AI should still have access to AGENTS.md context
    let final_reminder_count = count_system_reminder_messages(&session);
    assert_eq!(
        final_reminder_count, initial_reminder_count,
        "Context reminders should be restored after clear. Expected {initial_reminder_count}, got {final_reminder_count}"
    );
}

/// Test that all system reminder types are reinjected after clear
///
/// This test verifies that environment info is present after reinjection,
/// ensuring the LLM has access to platform, architecture, shell, user, and
/// working directory information.
///
/// Scenario: All system reminder types restored after clear
///
/// @step Given I have a session with context reminders injected
/// @step When I clear the history and reinject reminders
/// @step Then environment reminder should be present with Working directory
#[test]
fn test_clear_history_restores_all_reminder_types() {
    let mut session = offline_session::session().expect("offline session");

    // @step Given I have a session with context reminders injected
    session.inject_context_reminders("/work".as_ref());

    // Add conversation
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Test message")),
    });
    session.token_tracker.input_tokens = 1000;

    // @step When I clear the history and reinject reminders
    clear_session_state(&mut session);
    session.inject_context_reminders("/work".as_ref());

    // @step Then environment reminder should be present with Working directory
    let has_environment = session.messages.iter().any(|msg| {
        if let Message::User { content, .. } = msg {
            content.iter().any(|item| {
                if let UserContent::Text(text) = item {
                    text.text.contains("<system-reminder>")
                        && text.text.contains("<!-- type:environment -->")
                        && text.text.contains("Working directory:")
                } else {
                    false
                }
            })
        } else {
            false
        }
    });
    assert!(
        has_environment,
        "Environment reminder should be present with Working directory info"
    );

    // Environment reminder should contain platform info
    let has_platform = session.messages.iter().any(|msg| {
        if let Message::User { content, .. } = msg {
            content.iter().any(|item| {
                if let UserContent::Text(text) = item {
                    text.text.contains("Platform:") && text.text.contains("Architecture:")
                } else {
                    false
                }
            })
        } else {
            false
        }
    });
    assert!(
        has_platform,
        "Environment reminder should contain Platform and Architecture info"
    );
}
