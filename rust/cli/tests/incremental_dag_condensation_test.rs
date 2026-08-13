#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/incremental-dag-condensation.feature
//
// This test file validates the acceptance criteria defined in the feature file.
// Scenarios map directly to Gherkin scenarios.

use codelet_cli::compaction_dag::{
    detect_existing_dag, COMPACTION_INSTRUCTION_FRESH, COMPACTION_INSTRUCTION_INCREMENTAL,
};
use codelet_cli::interactive_helpers::execute_compaction;
use rig::message::{AssistantContent, Message, Text, UserContent};
use rig::one_or_many::OneOrMany;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

// ========================================
// Test helpers
// ========================================

fn create_test_session() -> codelet_cli::session::Session {
    let provider_manager =
        codelet_providers::ProviderManager::new().expect("Need at least one API key for tests");

    let mut session = codelet_cli::session::Session::from_provider_manager(provider_manager);
    // Add environment system reminder (always present)
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
            content: OneOrMany::one(UserContent::text(format!("User message {i}"))),
        });
        let assistant_text = AssistantContent::Text(Text {
            text: format!("Assistant response {i}"),
        });
        session.messages.push(Message::Assistant {
            id: None,
            content: OneOrMany::one(assistant_text),
        });
    }
}

fn add_compaction_dag(session: &mut codelet_cli::session::Session, dag_content: &str) {
    let wrapped = format!(
        "<system-reminder>\n<!-- type:compaction-dag -->\n{dag_content}\n</system-reminder>"
    );
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(&wrapped)),
    });
}

// ========================================
// Scenario: First compaction uses FRESH instruction when no existing DAG
// ========================================

#[tokio::test]
async fn test_first_compaction_uses_fresh_instruction() {
    // @step Given a session with conversation messages but no compaction-dag system-reminder
    let mut session = create_test_session();
    add_conversation_turns(&mut session, 20);
    session.token_tracker.input_tokens = 50_000;
    let compaction_flag = Arc::new(AtomicBool::new(false));

    // @step When execute_compaction is called
    execute_compaction(&mut session, compaction_flag.clone(), None)
        .await
        .expect("execute_compaction should succeed");

    // Get the last user message (the injected instruction)
    let last_user = session
        .messages
        .iter()
        .rev()
        .find(|m| matches!(m, Message::User { .. }))
        .expect("Should have a user message");
    let instruction_text = match last_user {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => t.text,
            _ => panic!("Expected text content"),
        },
        _ => panic!("Expected user message"),
    };

    // @step Then the injected user message should contain the FRESH compaction instruction
    assert!(
        instruction_text.contains("Build a hierarchical summary DAG"),
        "FRESH instruction should guide agent to build full DAG"
    );

    // @step And the FRESH instruction should mention SessionSearch for strategic searching
    assert!(
        instruction_text.contains("SessionSearch"),
        "Should mention SessionSearch"
    );

    // @step And the FRESH instruction should explain D0, D1, and D2 depth semantics
    assert!(instruction_text.contains("D0"), "Should mention D0");
    assert!(instruction_text.contains("D1"), "Should mention D1");
    assert!(instruction_text.contains("D2"), "Should mention D2");

    // @step And the FRESH instruction should tell the agent to call inject_summary
    assert!(
        instruction_text.contains("inject_summary"),
        "Should tell agent to call inject_summary"
    );
}

// ========================================
// Scenario: Second compaction uses INCREMENTAL instruction when existing DAG found
// ========================================

#[tokio::test]
async fn test_second_compaction_uses_incremental_instruction() {
    // @step Given a session with conversation messages and an existing compaction-dag system-reminder
    let mut session = create_test_session();
    let dag = r#"<dag-node depth="D2" turns="0-40" label="Architecture Decisions">
- JWT + Redis for auth
</dag-node>

<dag-node depth="D1" turns="41-70" label="Auth Implementation">
- Completed login handler
</dag-node>

<dag-node depth="D0" turns="71-82" label="Current: rate-limit tests">
- Working on rate limiting
</dag-node>"#;

    // @step And the existing DAG contains dag-node blocks with max turn_end of 82
    add_compaction_dag(&mut session, dag);
    add_conversation_turns(&mut session, 20);
    session.token_tracker.input_tokens = 50_000;
    let compaction_flag = Arc::new(AtomicBool::new(false));

    // @step When execute_compaction is called
    execute_compaction(&mut session, compaction_flag.clone(), None)
        .await
        .expect("execute_compaction should succeed");

    let last_user = session
        .messages
        .iter()
        .rev()
        .find(|m| matches!(m, Message::User { .. }))
        .expect("Should have a user message");
    let instruction_text = match last_user {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => t.text,
            _ => panic!("Expected text content"),
        },
        _ => panic!("Expected user message"),
    };

    // @step Then the injected user message should contain the INCREMENTAL compaction instruction
    assert!(
        instruction_text.contains("PRESERVE"),
        "INCREMENTAL instruction should mention PRESERVE"
    );

    // @step And the instruction should include the existing DAG content
    assert!(
        instruction_text.contains("Architecture Decisions"),
        "Should include existing DAG content"
    );

    // @step And the instruction should reference start_turn 83 for searching only new turns
    assert!(
        instruction_text.contains("83"),
        "Should reference start_turn 83 (max turn_end 82 + 1)"
    );
}

// ========================================
// Scenario: detect_existing_dag finds DAG in messages
// ========================================

#[test]
fn test_detect_existing_dag_finds_dag() {
    // @step Given a session messages list containing a user message with compaction-dag marker
    let mut messages = vec![Message::User {
        content: OneOrMany::one(UserContent::text(
            "<system-reminder>\n<!-- type:environment -->\nPlatform: test\n</system-reminder>",
        )),
    }];

    // @step And the DAG content has dag-node blocks with turns 0-20 and 21-50
    let dag = r#"<dag-node depth="D2" turns="0-20" label="Decisions">
- Choice A
</dag-node>
<dag-node depth="D0" turns="21-50" label="Recent work">
- Current task
</dag-node>"#;
    let wrapped =
        format!("<system-reminder>\n<!-- type:compaction-dag -->\n{dag}\n</system-reminder>");
    messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(&wrapped)),
    });

    // Add some conversation after the DAG
    messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Some conversation")),
    });

    // @step When detect_existing_dag is called with those messages
    let result = detect_existing_dag(&messages);

    // @step Then it should return Some with the DAG content string
    assert!(result.is_some(), "Should detect existing DAG");
    let (content, turn_end) = result.unwrap();
    assert!(content.contains("Decisions"), "Should contain DAG content");

    // @step And the returned max_turn_end should be 50
    assert_eq!(turn_end, 50, "max_turn_end should be 50");
}

// ========================================
// Scenario: detect_existing_dag returns None when no DAG exists
// ========================================

#[test]
fn test_detect_existing_dag_returns_none_without_dag() {
    // @step Given a session messages list with only regular conversation messages
    let messages = vec![
        Message::User {
            content: OneOrMany::one(UserContent::text(
                "<system-reminder>\n<!-- type:environment -->\nPlatform: test\n</system-reminder>",
            )),
        },
        Message::User {
            content: OneOrMany::one(UserContent::text("Hello, I need help")),
        },
        Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::Text(Text {
                text: "Sure, how can I help?".to_string(),
            })),
        },
    ];

    // @step When detect_existing_dag is called with those messages
    let result = detect_existing_dag(&messages);

    // @step Then it should return None
    assert!(result.is_none(), "Should return None when no DAG exists");
}

// ========================================
// Scenario: Incremental template substitution replaces placeholders
// ========================================

#[test]
fn test_incremental_template_substitution() {
    // @step Given a compaction-dag exists with content containing architecture decisions
    let dag_content = "# Architecture\n- JWT auth\n- Redis sessions";

    // @step And the parsed dag-nodes have a max turn_end of 95
    let max_turn_end: usize = 95;

    // @step When the incremental instruction is constructed
    let instruction = COMPACTION_INSTRUCTION_INCREMENTAL
        .replace("{existing_dag_content}", dag_content)
        .replace("{last_compacted_turn}", &max_turn_end.to_string());

    // @step Then the placeholder {existing_dag_content} should be replaced with the actual DAG content
    assert!(
        instruction.contains("JWT auth"),
        "Should contain actual DAG content"
    );
    assert!(
        !instruction.contains("{existing_dag_content}"),
        "Placeholder should be replaced"
    );

    // @step And the placeholder {last_compacted_turn} should be replaced with 95
    assert!(instruction.contains("95"), "Should contain turn number 95");
    assert!(
        !instruction.contains("{last_compacted_turn}"),
        "Placeholder should be replaced"
    );
}

// ========================================
// Scenario: execute_compaction appends resume prompt for both modes
// ========================================

#[tokio::test]
async fn test_execute_compaction_appends_resume_prompt_fresh() {
    // @step Given a session with no existing DAG
    let mut session = create_test_session();
    add_conversation_turns(&mut session, 10);
    session.token_tracker.input_tokens = 50_000;
    let compaction_flag = Arc::new(AtomicBool::new(false));

    // @step When execute_compaction is called with a last_user_message of "implement the login feature"
    execute_compaction(
        &mut session,
        compaction_flag.clone(),
        Some("implement the login feature"),
    )
    .await
    .expect("should succeed");

    let last_user = session
        .messages
        .iter()
        .rev()
        .find(|m| matches!(m, Message::User { .. }))
        .unwrap();
    let text = match last_user {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => t.text,
            _ => panic!("Expected text"),
        },
        _ => panic!("Expected user"),
    };

    // @step Then the injected message should contain the FRESH instruction
    assert!(text.contains("Build a hierarchical summary DAG"));

    // @step And the injected message should contain "implement the login feature" as the resume prompt
    assert!(text.contains("implement the login feature"));
}

#[tokio::test]
async fn test_execute_compaction_appends_resume_prompt_incremental() {
    // @step Given a session with an existing DAG
    let mut session = create_test_session();
    let dag = r#"<dag-node depth="D2" turns="0-30" label="Arch">
- Decisions
</dag-node>"#;
    add_compaction_dag(&mut session, dag);
    add_conversation_turns(&mut session, 10);
    session.token_tracker.input_tokens = 50_000;
    let compaction_flag = Arc::new(AtomicBool::new(false));

    // @step When execute_compaction is called with a last_user_message of "fix the test"
    execute_compaction(&mut session, compaction_flag.clone(), Some("fix the test"))
        .await
        .expect("should succeed");

    let last_user = session
        .messages
        .iter()
        .rev()
        .find(|m| matches!(m, Message::User { .. }))
        .unwrap();
    let text = match last_user {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => t.text,
            _ => panic!("Expected text"),
        },
        _ => panic!("Expected user"),
    };

    // @step Then the injected message should contain the INCREMENTAL instruction
    assert!(text.contains("PRESERVE"));

    // @step And the injected message should contain "fix the test" as the resume prompt
    assert!(text.contains("fix the test"));
}

// ========================================
// Scenario: Existing DAG with no parseable dag-node blocks uses fallback turn_end
// ========================================

#[test]
fn test_detect_existing_dag_fallback_turn_end() {
    // @step Given a session with a compaction-dag system-reminder containing only plain text (no dag-node blocks)
    let plain_dag = "# Summary\n- Some notes without structured dag-node blocks";
    let wrapped =
        format!("<system-reminder>\n<!-- type:compaction-dag -->\n{plain_dag}\n</system-reminder>");
    let messages = vec![Message::User {
        content: OneOrMany::one(UserContent::text(&wrapped)),
    }];

    // @step When detect_existing_dag is called
    let result = detect_existing_dag(&messages);

    // @step Then it should return Some with the DAG content
    assert!(result.is_some(), "Should still detect the DAG message");

    // @step And the returned max_turn_end should be 0 as a fallback
    let (_, turn_end) = result.unwrap();
    assert_eq!(turn_end, 0, "Fallback turn_end should be 0");
}

// ========================================
// Scenario: FRESH instruction preserves current behavior
// ========================================

#[test]
fn test_fresh_instruction_content() {
    // @step Given the COMPACTION_INSTRUCTION_FRESH constant
    let instruction = COMPACTION_INSTRUCTION_FRESH;

    // @step When its content is examined
    assert!(!instruction.is_empty(), "Should not be empty");

    // @step Then it should contain guidance for SessionSearch strategic searching
    assert!(
        instruction.contains("SessionSearch"),
        "Should mention SessionSearch"
    );

    // @step And it should contain the dag-node XML block format with depth, turns, and label attributes
    assert!(
        instruction.contains("dag-node"),
        "Should mention dag-node format"
    );
    assert!(
        instruction.contains("depth"),
        "Should mention depth attribute"
    );
    assert!(
        instruction.contains("turns"),
        "Should mention turns attribute"
    );
    assert!(
        instruction.contains("label"),
        "Should mention label attribute"
    );

    // @step And it should contain D0, D1, and D2 depth semantics
    assert!(instruction.contains("D0"), "Should mention D0");
    assert!(instruction.contains("D1"), "Should mention D1");
    assert!(instruction.contains("D2"), "Should mention D2");

    // @step And it should instruct the agent to call inject_summary
    assert!(
        instruction.contains("inject_summary"),
        "Should mention inject_summary"
    );
}

// ========================================
// Scenario: INCREMENTAL instruction contains promotion guidance
// ========================================

#[test]
fn test_incremental_instruction_content() {
    // @step Given the COMPACTION_INSTRUCTION_INCREMENTAL constant template
    let instruction = COMPACTION_INSTRUCTION_INCREMENTAL;

    // @step When its content is examined
    assert!(!instruction.is_empty(), "Should not be empty");

    // @step Then it should instruct to PRESERVE existing D2 nodes unchanged
    assert!(
        instruction.contains("PRESERVE") && instruction.contains("D2"),
        "Should instruct to preserve D2 nodes"
    );

    // @step And it should instruct to REVIEW existing D1 nodes
    assert!(
        instruction.contains("REVIEW") || instruction.contains("D1"),
        "Should mention reviewing D1 nodes"
    );

    // @step And it should instruct to PROMOTE existing D0 nodes to D1
    assert!(
        instruction.contains("PROMOTE") && instruction.contains("D0"),
        "Should instruct to promote D0 to D1"
    );

    // @step And it should instruct to search ONLY for turns since last compaction
    assert!(
        instruction.contains("{last_compacted_turn}"),
        "Should contain last_compacted_turn placeholder"
    );

    // @step And it should contain placeholders {existing_dag_content} and {last_compacted_turn}
    assert!(
        instruction.contains("{existing_dag_content}"),
        "Should contain existing_dag_content placeholder"
    );
    assert!(
        instruction.contains("{last_compacted_turn}"),
        "Should contain last_compacted_turn placeholder"
    );

    // @step And it should instruct to call inject_summary with the updated DAG
    assert!(
        instruction.contains("inject_summary"),
        "Should mention inject_summary"
    );
}
