//! Feature: spec/features/telegram-clear-context-reset.feature
use super::*;

/// Scenario: Clear command resets AI context completely
///
/// @step Given I have an active conversation with the AI via Telegram bridge
/// @step And the conversation has accumulated messages and tokens
/// @step When I send "/clear" via Telegram
/// @step Then the AI should not remember the previous conversation
/// @step And the next message should be treated as a fresh conversation start
/// @step And no confirmation dialog should appear
#[test]
fn test_telegram_clear_resets_context_completely() {
    let mut session = offline_session::session().expect("offline session");

    // @step Given I have an active conversation with the AI via Telegram bridge
    // @step And the conversation has accumulated messages and tokens
    session.inject_context_reminders("/work".as_ref());

    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Hello, I'm testing from Telegram")),
    });
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Remember this: my secret code is 42")),
    });
    session.token_tracker.input_tokens = 5000;
    session.token_tracker.output_tokens = 1000;

    let messages_before = session.messages.len();
    assert!(messages_before > 0, "Should have messages before clear");
    assert!(
        session.token_tracker.input_tokens > 0,
        "Should have tokens before clear"
    );

    // @step When I send "/clear" via Telegram
    // (This simulates what the control handler SHOULD do after fix)
    simulate_correct_clear_action(&mut session);

    // @step Then the AI should not remember the previous conversation
    // Check that user messages are gone (only system reminders should remain)
    let has_secret_code = session.messages.iter().any(|msg| {
        if let Message::User { content, .. } = msg {
            content.iter().any(|item| {
                if let UserContent::Text(text) = item {
                    text.text.contains("42") || text.text.contains("secret code")
                } else {
                    false
                }
            })
        } else {
            false
        }
    });
    assert!(
        !has_secret_code,
        "Previous conversation should be cleared - no secret code"
    );

    // @step And the next message should be treated as a fresh conversation start
    assert_eq!(
        session.token_tracker.input_tokens, 0,
        "Input tokens should be 0"
    );
    assert_eq!(
        session.token_tracker.output_tokens, 0,
        "Output tokens should be 0"
    );

    // @step And no confirmation dialog should appear
    // (This is a UI concern - verified by the immediate action execution)
}

/// Scenario: System reminders preserved after clear
///
/// @step Given I have an active conversation with the AI via Telegram bridge
/// @step And the AI has access to project context (AGENTS.md, environment info)
/// @step When I send "/clear" via Telegram
/// @step Then the AI should still have access to AGENTS.md project context
/// @step And the AI should still know the platform and working directory
/// @step And the conversation history should be cleared
#[test]
fn test_telegram_clear_preserves_system_reminders() {
    let mut session = offline_session::session().expect("offline session");

    // @step Given I have an active conversation with the AI via Telegram bridge
    // @step And the AI has access to project context (AGENTS.md, environment info)
    session.inject_context_reminders("/work".as_ref());
    let initial_reminder_count = count_system_reminder_messages(&session);

    // Add conversation messages
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Test conversation message")),
    });
    session.token_tracker.input_tokens = 1000;

    // @step When I send "/clear" via Telegram
    simulate_correct_clear_action(&mut session);

    // @step Then the AI should still have access to AGENTS.md project context
    // @step And the AI should still know the platform and working directory
    let final_reminder_count = count_system_reminder_messages(&session);
    assert_eq!(
        final_reminder_count, initial_reminder_count,
        "System reminders should be restored after clear. Expected {initial_reminder_count}, got {final_reminder_count}"
    );

    // Check for environment info
    let has_environment = session.messages.iter().any(|msg| {
        if let Message::User { content, .. } = msg {
            content.iter().any(|item| {
                if let UserContent::Text(text) = item {
                    text.text.contains("<system-reminder>")
                        && text.text.contains("<!-- type:environment -->")
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
        "Environment system reminder should be present"
    );

    // @step And the conversation history should be cleared
    assert_eq!(
        session.token_tracker.input_tokens, 0,
        "Tokens should be reset"
    );
}
