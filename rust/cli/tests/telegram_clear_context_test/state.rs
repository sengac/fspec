//! Feature: spec/features/telegram-clear-context-reset.feature
use super::*;

/// Scenario: Token counters reset after clear
///
/// @step Given I have an active conversation with accumulated tokens
/// @step And the session shows input and output token counts
/// @step When I send "/clear" via Telegram
/// @step Then the token counters should show "0↓ 0↑"
/// @step And the session.messages should be empty
/// @step And the session.turns should be empty
#[test]
fn test_telegram_clear_resets_tokens() {
    let mut session = offline_session::session().expect("offline session");

    // @step Given I have an active conversation with accumulated tokens
    // @step And the session shows input and output token counts
    session.token_tracker.input_tokens = 5000;
    session.token_tracker.output_tokens = 2000;
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Accumulated conversation")),
    });

    // @step When I send "/clear" via Telegram
    simulate_correct_clear_action(&mut session);

    // @step Then the token counters should show "0↓ 0↑"
    assert_eq!(
        session.token_tracker.input_tokens, 0,
        "Input tokens should be 0"
    );
    assert_eq!(
        session.token_tracker.output_tokens, 0,
        "Output tokens should be 0"
    );

    // @step And the session.messages should be empty (except system reminders)
    // Only system reminders should remain
    let has_user_messages = session.messages.iter().any(|msg| {
        if let Message::User { content, .. } = msg {
            !content.iter().any(|item| {
                if let UserContent::Text(text) = item {
                    text.text.contains("<system-reminder>")
                } else {
                    false
                }
            })
        } else {
            true // Non-user messages count as user content
        }
    });
    assert!(
        !has_user_messages,
        "Non-system-reminder messages should be cleared"
    );

    // @step And the session.turns should be empty
    assert!(session.turns.is_empty(), "Turns should be cleared");
}

/// Scenario: Clear resets session state not just output buffer
///
/// @step Given I have an active conversation via Telegram bridge
/// @step And the session has messages, turns, and token_tracker state
/// @step When I send "/clear" via Telegram
/// @step Then session.messages should be cleared
/// @step And session.turns should be cleared
/// @step And token_tracker should be reset to default
/// @step And inject_context_reminders() should be called to restore system context
#[test]
fn test_telegram_clear_resets_full_session_state() {
    let mut session = offline_session::session().expect("offline session");

    // @step Given I have an active conversation via Telegram bridge
    // @step And the session has messages, turns, and token_tracker state
    session.inject_context_reminders("/work".as_ref());
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Test message 1")),
    });
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Test message 2")),
    });
    session.token_tracker.input_tokens = 3000;
    session.token_tracker.output_tokens = 1500;

    // Verify pre-conditions
    assert!(!session.messages.is_empty(), "Should have messages");
    assert!(
        session.token_tracker.input_tokens > 0,
        "Should have input tokens"
    );

    // @step When I send "/clear" via Telegram
    simulate_correct_clear_action(&mut session);

    // @step Then session.messages should be cleared (except reinjected reminders)
    // @step And session.turns should be cleared
    assert!(session.turns.is_empty(), "Turns should be empty");

    // @step And token_tracker should be reset to default
    assert_eq!(session.token_tracker.input_tokens, 0, "Input tokens reset");
    assert_eq!(
        session.token_tracker.output_tokens, 0,
        "Output tokens reset"
    );

    // @step And inject_context_reminders() should be called to restore system context
    let has_system_reminders = count_system_reminder_messages(&session) > 0;
    assert!(
        has_system_reminders,
        "System reminders should be reinjected"
    );
}
