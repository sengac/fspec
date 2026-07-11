#![cfg(not(feature = "noop"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/inject-summary-handler.feature
//!
//! This test file validates the acceptance criteria for the inject_summary
//! NAPI handler. Tests verify the deferred DAG injection approach:
//! - Handler stores DAG content in pending_dag (no session lock)
//! - apply_pending_dag applies it when agent_loop holds the lock

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use codelet_cli::session::system_reminders::{
    add_system_reminder, is_system_reminder, SystemReminderType,
};
use codelet_common::token_estimator::count_tokens;
use codelet_napi::inject_summary_handler;
use codelet_tools::inject_summary::{
    set_inject_summary_handler, InjectSummaryHandler, InjectSummaryResult,
};
use rig::message::{AssistantContent, Message, UserContent};
use rig::OneOrMany;
use uuid::Uuid;

use codelet_cli::session::Session;

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

/// Helper to create a system reminder message
fn create_system_reminder(reminder_type: SystemReminderType, content: &str) -> Message {
    let messages: Vec<Message> = vec![];
    let result = add_system_reminder(&messages, reminder_type, content);
    result.into_iter().next().unwrap()
}

/// Helper to extract text content from a message
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

/// Helper to create a session with system reminders and conversation messages
fn create_session_with_messages(
    num_system_reminders: usize,
    num_conversation_pairs: usize,
) -> Session {
    // CMPCT-038 drive-by: fall back to an explicit provider so the test
    // also runs in multi-credential environments (ProviderManager::new()
    // errors when several providers are credentialed but none selected).
    let provider_manager = codelet_providers::ProviderManager::new()
        .or_else(|_| codelet_providers::ProviderManager::with_provider("gemini"))
        .or_else(|_| codelet_providers::ProviderManager::with_provider("zai"))
        .or_else(|_| codelet_providers::ProviderManager::with_provider("claude"))
        .expect("Need at least one API key for tests");
    let mut session = Session::from_provider_manager(provider_manager);

    let reminder_types = [
        SystemReminderType::Environment,
        SystemReminderType::ClaudeMd,
        SystemReminderType::FspecWorkflow,
        SystemReminderType::GitStatus,
        SystemReminderType::TokenStatus,
    ];

    for i in 0..num_system_reminders {
        let reminder_type = reminder_types[i % reminder_types.len()];
        let content = format!("System reminder content #{}", i + 1);
        let reminder = create_system_reminder(reminder_type, &content);
        session.messages.push(reminder);
    }

    for i in 0..num_conversation_pairs {
        session
            .messages
            .push(create_user_message(&format!("User message #{}", i + 1)));
        session.messages.push(create_assistant_message(&format!(
            "Assistant response #{}",
            i + 1
        )));
    }

    session
}

// =============================================================================
// Scenario: Handler stores DAG content in pending_dag without locking session
// =============================================================================

#[test]
fn test_handler_stores_dag_in_pending() {
    // @step Given a pending_dag slot and compaction_in_progress flag
    let pending_dag = Arc::new(std::sync::Mutex::new(None));
    let compaction_flag = Arc::new(AtomicBool::new(true));
    let context_window: u64 = 200_000;

    let handler = inject_summary_handler::create_handler(
        pending_dag.clone(),
        context_window,
        compaction_flag.clone(),
        None,
    );

    // @step When the handler is called with DAG content
    let result = handler(Uuid::new_v4(), "# D2: Architecture\n- JWT auth".to_string());
    assert!(result.is_ok(), "Handler should succeed, got: {:?}", result);

    // @step Then pending_dag should contain the wrapped DAG content
    let stored = pending_dag.lock().unwrap();
    assert!(stored.is_some(), "pending_dag should have content");
    let content = stored.as_ref().unwrap();
    assert!(content.contains("<system-reminder>"));
    assert!(content.contains("<!-- type:compaction-dag -->"));
    assert!(content.contains("JWT auth"));
}

// =============================================================================
// Scenario: Handler clears compaction_in_progress flag
// =============================================================================

#[test]
fn test_handler_clears_compaction_flag() {
    let pending_dag = Arc::new(std::sync::Mutex::new(None));
    let compaction_flag = Arc::new(AtomicBool::new(true));

    let handler =
        inject_summary_handler::create_handler(pending_dag, 200_000, compaction_flag.clone(), None);

    // @step When the handler is called
    let result = handler(Uuid::new_v4(), "# Summary".to_string());
    assert!(result.is_ok());

    // @step Then compaction_in_progress should be false
    assert!(
        !compaction_flag.load(Ordering::Relaxed),
        "compaction_in_progress should be cleared"
    );
}

// =============================================================================
// Scenario: Handler returns accurate token counts
// =============================================================================

#[test]
fn test_handler_returns_token_counts() {
    let pending_dag = Arc::new(std::sync::Mutex::new(None));
    let context_window: u64 = 200_000;

    let handler = inject_summary_handler::create_handler(
        pending_dag,
        context_window,
        Arc::new(AtomicBool::new(true)),
        None,
    );

    let dag_content =
        "# D2: Architecture\n".to_string() + "- Design pattern detail. ".repeat(50).as_str();

    let result = handler(Uuid::new_v4(), dag_content);
    assert!(result.is_ok());
    let result = result.unwrap();

    assert!(result.injected_tokens > 0, "injected_tokens should be > 0");
    assert!(
        result.remaining_budget > 0,
        "remaining_budget should be > 0"
    );
    assert!(
        result.remaining_budget < context_window,
        "remaining_budget should be less than context_window"
    );
}

// =============================================================================
// Scenario: apply_pending_dag partitions, clears, restores, and injects
// =============================================================================

#[test]
fn test_apply_pending_dag_full_cycle() {
    // @step Given a session with 5 system reminders and 97 conversation pairs
    let mut session = create_session_with_messages(5, 97);
    session
        .messages
        .push(create_user_message("Final user message"));
    assert_eq!(session.messages.len(), 200);

    // @step And a pending_dag with wrapped DAG content
    let pending_dag = Arc::new(std::sync::Mutex::new(Some(
        codelet_core::compaction::wrap_dag_content("# D2: Architecture\n- JWT auth"),
    )));

    // @step When apply_pending_dag is called
    let result = inject_summary_handler::apply_pending_dag(&mut session, &pending_dag);

    // @step Then it should return Some (DAG was applied)
    assert!(result.is_some(), "Should return Some when DAG was applied");

    // @step And session should have 5 system reminders + 1 DAG = 6 messages
    assert_eq!(
        session.messages.len(),
        6,
        "Should have 5 system reminders + 1 DAG = 6, got {}",
        session.messages.len()
    );

    // @step And the first 5 messages should be system reminders
    for i in 0..5 {
        assert!(
            is_system_reminder(&session.messages[i]),
            "Message {} should be a system reminder",
            i
        );
    }

    // @step And the last message should contain the DAG
    let text = get_message_text(&session.messages[5]).unwrap();
    assert!(text.contains("JWT auth"));
    assert!(text.contains("<system-reminder>"));
}

// =============================================================================
// Scenario: apply_pending_dag with no pending content is a no-op
// =============================================================================

#[test]
fn test_apply_pending_dag_no_content() {
    let pending_dag = Arc::new(std::sync::Mutex::new(None));
    let mut session = create_session_with_messages(3, 10);
    let original_count = session.messages.len();

    let applied = inject_summary_handler::apply_pending_dag(&mut session, &pending_dag);

    assert!(applied.is_none(), "Should return None when no pending DAG");
    assert_eq!(
        session.messages.len(),
        original_count,
        "Messages should be unchanged"
    );
}

// =============================================================================
// Scenario: apply_pending_dag resets turns and token tracker
// =============================================================================

#[test]
fn test_apply_pending_dag_resets_tracker() {
    let mut session = create_session_with_messages(3, 10);

    // Simulate accumulated state
    session
        .turns
        .push(codelet_core::compaction::ConversationTurn {
            user_message: "Hello".to_string(),
            assistant_response: "Hi".to_string(),
            tool_calls: vec![],
            tool_results: vec![],
            tokens: 50,
            timestamp: std::time::SystemTime::now(),
            previous_error: None,
        });
    session.token_tracker.input_tokens = 50_000;
    session.token_tracker.output_tokens = 10_000;

    let pending_dag = Arc::new(std::sync::Mutex::new(Some(
        codelet_core::compaction::wrap_dag_content("# Summary"),
    )));

    let applied = inject_summary_handler::apply_pending_dag(&mut session, &pending_dag);
    assert!(applied.is_some());

    // Turns should be cleared
    assert!(
        session.turns.is_empty(),
        "turns should be empty after injection"
    );

    // Token tracker should reflect only post-injection messages
    let actual_tokens: u64 = session
        .messages
        .iter()
        .map(|msg| count_tokens(&get_message_text(msg).unwrap_or_default()) as u64)
        .sum();
    assert_eq!(
        session.token_tracker.input_tokens, actual_tokens,
        "input_tokens should match actual message tokens"
    );
    assert_eq!(
        session.token_tracker.output_tokens, 0,
        "output_tokens should be 0"
    );
}

// =============================================================================
// Scenario: Handler registration and cleanup
// =============================================================================

#[test]
fn test_handler_registration_callable() {
    let session_id = Uuid::new_v4();
    let handler: InjectSummaryHandler = Arc::new(|_sid, _content| {
        Ok(InjectSummaryResult {
            injected_tokens: 100,
            remaining_budget: 199_900,
        })
    });

    set_inject_summary_handler(session_id, Some(handler));
    assert!(codelet_tools::inject_summary::has_inject_summary_handler(
        session_id
    ));

    set_inject_summary_handler(session_id, None);
    assert!(!codelet_tools::inject_summary::has_inject_summary_handler(
        session_id
    ));
}

// =============================================================================
// Scenario: create_handler returns correct InjectSummaryHandler type
// =============================================================================

#[test]
fn test_create_handler_returns_correct_type() {
    let pending_dag = Arc::new(std::sync::Mutex::new(None));
    let context_window: u64 = 200_000;

    let handler: InjectSummaryHandler = inject_summary_handler::create_handler(
        pending_dag,
        context_window,
        Arc::new(AtomicBool::new(false)),
        None,
    );

    // Callable and returns a Result
    let result = handler(Uuid::new_v4(), "test content".to_string());
    let _ = result;
}
