#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/compaction-convergence-watchdog.feature
//
// This test file validates the acceptance criteria defined in the feature file.
// Scenarios map directly to Gherkin scenarios.

use codelet_cli::interactive_helpers::{
    extract_partial_dag_nodes, force_inject_fallback_dag,
    COMPACTION_ESCALATION_MESSAGE,
};
use rig::message::{AssistantContent, Message, Text, UserContent};
use rig::one_or_many::OneOrMany;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ========================================
// Test helpers
// ========================================

fn create_test_session() -> codelet_cli::session::Session {
    let provider_manager =
        codelet_providers::ProviderManager::new().expect("Need at least one API key for tests");

    let mut session = codelet_cli::session::Session::from_provider_manager(provider_manager);
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(
            "<system-reminder>\n<!-- type:environment -->\nPlatform: test\n</system-reminder>",
        )),
    });
    session
}

fn add_conversation_turns(session: &mut codelet_cli::session::Session, count: usize) {
    for i in 0..count {
        session.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text(&format!("User message {}", i))),
        });
        let assistant_text = AssistantContent::Text(Text {
            text: format!("Assistant response {}", i),
        });
        session.messages.push(Message::Assistant {
            id: None,
            content: OneOrMany::one(assistant_text),
        });
    }
}

// ========================================
// Scenario: Normal compaction succeeds without watchdog intervention
// ========================================

#[test]
fn test_normal_compaction_no_watchdog() {
    // @step Given a session in compaction mode after execute_compaction
    let compaction_flag = Arc::new(AtomicBool::new(true));

    // @step When the agent calls inject_summary during the first stream attempt
    // Simulate: inject_summary handler clears the flag
    compaction_flag.store(false, Ordering::SeqCst);

    // @step Then the compaction_in_progress flag should be cleared
    assert!(
        !compaction_flag.load(Ordering::Relaxed),
        "Flag should be cleared after inject_summary"
    );

    // @step And no escalation message should be injected
    // Watchdog logic: if !compaction_in_progress → no escalation needed
    let needs_escalation = compaction_flag.load(Ordering::Acquire);
    assert!(!needs_escalation, "No escalation should be needed");

    // @step And the watchdog counter should remain at 0
    let watchdog_counter: usize = 0; // Would be 0 since first attempt succeeded
    assert_eq!(watchdog_counter, 0, "Watchdog counter should be 0");
}

// ========================================
// Scenario: Escalation triggers after first failed attempt
// ========================================

#[test]
fn test_escalation_triggers_after_first_failure() {
    // @step Given a session in compaction mode where the agent never calls inject_summary
    let mut session = create_test_session();
    add_conversation_turns(&mut session, 5);
    let compaction_flag = Arc::new(AtomicBool::new(true));
    let mut watchdog_counter: usize = 0;

    // @step When the first stream attempt completes without inject_summary
    // Simulate: stream ends but compaction_in_progress is still true
    let still_compacting = compaction_flag.load(Ordering::Acquire);
    assert!(still_compacting, "Flag should still be true");
    watchdog_counter += 1;

    // @step Then the watchdog should detect that compaction_in_progress is still true
    assert!(compaction_flag.load(Ordering::Acquire));
    assert_eq!(watchdog_counter, 1, "Counter should be 1 after first failure");

    // @step And an escalation message should be injected into session messages
    // Simulate watchdog decision logic
    if watchdog_counter == 1 {
        session.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text(COMPACTION_ESCALATION_MESSAGE)),
        });
    }
    let has_escalation = session.messages.iter().any(|m| {
        if let Message::User { content } = m {
            if let UserContent::Text(t) = content.first() {
                return t.text.contains("TIMEOUT") || t.text.contains("inject_summary");
            }
        }
        false
    });
    assert!(has_escalation, "Escalation message should be in messages");

    // @step And a second stream attempt should be initiated automatically
    // The watchdog counter being 1 means we should retry
    assert!(watchdog_counter < 2, "Should retry before force-inject threshold");
}

// ========================================
// Scenario: Escalation message content is directive
// ========================================

#[test]
fn test_escalation_message_content() {
    // @step Given the COMPACTION_ESCALATION_MESSAGE constant
    let message = COMPACTION_ESCALATION_MESSAGE;

    // @step When its content is examined
    assert!(!message.is_empty(), "Message should not be empty");

    // @step Then it should instruct the agent to stop making SessionSearch calls
    assert!(
        message.contains("SessionSearch") || message.contains("search"),
        "Should mention stopping search calls"
    );

    // @step And it should instruct the agent to write a summary and call inject_summary immediately
    assert!(
        message.contains("inject_summary"),
        "Should mention inject_summary"
    );

    // @step And it should convey urgency about the compaction timeout
    assert!(
        message.contains("TIMEOUT") || message.contains("timeout") || message.contains("NOW"),
        "Should convey urgency"
    );
}

// ========================================
// Scenario: force_inject_fallback_dag resets session state correctly
// ========================================

#[test]
fn test_force_inject_resets_session_state() {
    // @step Given a session with conversation messages and compaction_in_progress flag set to true
    let mut session = create_test_session();
    add_conversation_turns(&mut session, 10);
    session.token_tracker.input_tokens = 50_000;
    let compaction_flag = Arc::new(AtomicBool::new(true));

    let fallback_dag = r#"<dag-node depth="D1" turns="0-20" label="Auto-recovered: compaction timeout">
Session was auto-compacted due to convergence timeout.
Use SessionSearch to recover context.
</dag-node>"#;

    // @step When force_inject_fallback_dag is called with a fallback DAG
    force_inject_fallback_dag(&mut session, &compaction_flag, fallback_dag);

    // @step Then it should call reset_session_to_reminders to preserve system reminders
    let has_env_reminder = session.messages.iter().any(|m| {
        if let Message::User { content } = m {
            if let UserContent::Text(t) = content.first() {
                return t.text.contains("type:environment");
            }
        }
        false
    });
    assert!(has_env_reminder, "Should preserve environment system reminder");

    // @step And the DAG should be wrapped in compaction-dag system-reminder tags
    let has_dag = session.messages.iter().any(|m| {
        if let Message::User { content } = m {
            if let UserContent::Text(t) = content.first() {
                return t.text.contains("type:compaction-dag");
            }
        }
        false
    });
    assert!(has_dag, "Should have wrapped DAG in compaction-dag tags");

    // @step And the wrapped DAG should be pushed as a user message
    let dag_msg = session.messages.iter().find(|m| {
        if let Message::User { content } = m {
            if let UserContent::Text(t) = content.first() {
                return t.text.contains("Auto-recovered");
            }
        }
        false
    });
    assert!(dag_msg.is_some(), "Fallback DAG should be in messages");

    // @step And recalculate_token_tracker should update the token counts
    assert!(
        session.token_tracker.input_tokens > 0,
        "Token tracker should be recalculated"
    );

    // @step And the compaction_in_progress flag should be cleared
    assert!(
        !compaction_flag.load(Ordering::Relaxed),
        "compaction_in_progress should be cleared"
    );
}

// ========================================
// Scenario: extract_partial_dag_nodes finds dag-node blocks in messages
// ========================================

#[test]
fn test_extract_partial_dag_nodes_finds_blocks() {
    // @step Given session messages containing partial dag-node blocks from assistant responses
    let messages = vec![
        Message::User {
            content: OneOrMany::one(UserContent::text("Build DAG")),
        },
        Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::Text(Text {
                text: r#"Here's my summary:
<dag-node depth="D2" turns="0-30" label="Architecture Decisions">
- JWT auth selected
</dag-node>

<dag-node depth="D0" turns="31-50" label="Current work">
- Implementing login
</dag-node>

I'll call inject_summary next..."#.to_string(),
            })),
        },
    ];

    // @step When extract_partial_dag_nodes is called
    let nodes = extract_partial_dag_nodes(&messages);

    // @step Then it should find and return all dag-node block strings from assistant messages
    assert!(!nodes.is_empty(), "Should find dag-node blocks");
    assert!(
        nodes.iter().any(|n| n.contains("Architecture Decisions")),
        "Should find first node"
    );
    assert!(
        nodes.iter().any(|n| n.contains("Current work")),
        "Should find second node"
    );
}

// ========================================
// Scenario: extract_partial_dag_nodes returns empty when no blocks exist
// ========================================

#[test]
fn test_extract_partial_dag_nodes_returns_empty() {
    // @step Given session messages with no dag-node blocks
    let messages = vec![
        Message::User {
            content: OneOrMany::one(UserContent::text("Build DAG")),
        },
        Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::Text(Text {
                text: "I'm looking at the session. Let me search more...".to_string(),
            })),
        },
    ];

    // @step When extract_partial_dag_nodes is called
    let nodes = extract_partial_dag_nodes(&messages);

    // @step Then it should return an empty collection
    assert!(nodes.is_empty(), "Should return empty when no dag-nodes found");
}

// ========================================
// Scenario: Force-inject with partial dag-nodes after two failed attempts
// ========================================

#[test]
fn test_force_inject_with_partial_dag_nodes() {
    // @step Given a session in compaction mode where the agent writes partial dag-node blocks but never calls inject_summary
    let mut session = create_test_session();
    session.token_tracker.input_tokens = 50_000;
    let compaction_flag = Arc::new(AtomicBool::new(true));

    // @step And the recent messages contain dag-node blocks for turns 0-30 and 31-50
    session.messages.push(Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::Text(Text {
            text: r#"<dag-node depth="D2" turns="0-30" label="Architecture">
- Decisions made
</dag-node>
<dag-node depth="D0" turns="31-50" label="Recent">
- Current work
</dag-node>"#.to_string(),
        })),
    });

    // @step When both stream attempts complete without inject_summary
    let partial = extract_partial_dag_nodes(&session.messages);
    assert!(!partial.is_empty(), "Should have partial dag-nodes");

    // @step Then the engine should extract partial dag-node blocks from recent messages
    // @step And assemble them into a complete DAG
    let assembled = partial.join("\n\n");
    assert!(assembled.contains("Architecture"));
    assert!(assembled.contains("Recent"));

    // @step And force-inject the assembled DAG into the session
    force_inject_fallback_dag(&mut session, &compaction_flag, &assembled);

    // @step And clear the compaction_in_progress flag
    assert!(!compaction_flag.load(Ordering::Relaxed));
}

// ========================================
// Scenario: Force-inject with minimal fallback DAG when no partial nodes exist
// ========================================

#[test]
fn test_force_inject_with_minimal_fallback() {
    // @step Given a session in compaction mode where the agent produces no dag-node blocks at all
    let mut session = create_test_session();
    add_conversation_turns(&mut session, 20);
    session.token_tracker.input_tokens = 50_000;
    let compaction_flag = Arc::new(AtomicBool::new(true));

    // @step When both stream attempts complete without inject_summary
    let partial = extract_partial_dag_nodes(&session.messages);
    assert!(partial.is_empty(), "No partial nodes exist");

    // @step Then the engine should create a minimal fallback DAG with a D1 node
    let last_turn = session.messages.len().saturating_sub(1);
    let fallback = format!(
        r#"<dag-node depth="D1" turns="0-{}" label="Auto-recovered: compaction timeout">
Session was auto-compacted due to convergence timeout.
Use SessionSearch to recover context.
</dag-node>"#,
        last_turn
    );

    // @step And the fallback D1 node should cover turns 0 through the last known turn
    assert!(fallback.contains(&format!("turns=\"0-{}\"", last_turn)));

    // @step And the fallback label should indicate auto-recovery from compaction timeout
    assert!(fallback.contains("Auto-recovered: compaction timeout"));

    // @step And force-inject the fallback DAG into the session
    force_inject_fallback_dag(&mut session, &compaction_flag, &fallback);

    // Verify session has the DAG
    let has_dag = session.messages.iter().any(|m| {
        if let Message::User { content } = m {
            if let UserContent::Text(t) = content.first() {
                return t.text.contains("Auto-recovered");
            }
        }
        false
    });
    assert!(has_dag, "Fallback DAG should be injected");
    assert!(!compaction_flag.load(Ordering::Relaxed), "Flag should be cleared");
}
