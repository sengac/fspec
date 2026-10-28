//! Feature: spec/features/committed-compaction-watchdog-lifecycle.feature
//! All nine historical watchdog cases remain, individually adapted to the
//! CMPCT-057/079 no-fallback contract and actual durable runtime APIs.
//
// This test file validates the acceptance criteria defined in the feature file.
// Scenarios map directly to Gherkin scenarios.

#[path = "fixtures/compactor_contract.rs"]
pub mod contract;
use codelet_cli::compaction_operation::{CompactionFailure, CompactionOperation};
use codelet_cli::interactive::StreamEvent;
use contract::{Fixture, Output, TestResult};
use rig::message::{AssistantContent, Message, Text, UserContent};
use rig::one_or_many::OneOrMany;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

// ========================================
// Test helpers
// ========================================

// ========================================
// Scenario: Normal compaction succeeds without watchdog intervention
// ========================================
//
// The original CMPCT-030 version of this test was tautological (it stored
// `false` into a local AtomicBool and asserted the AtomicBool was false).
// Restored (CMPCT-047): the test now asserts the engine's actual non-escalation
// decision predicate — a successful inject_summary clears the compaction flag
// (the `swap(false)` safety net in the agent loops) before the watchdog check
// runs, so `was_compacting && !has_pending_dag` must be false, no escalation
// message may be injected, and the counter stays at 0.

#[tokio::test]
#[serial_test::serial]
async fn scenario_normal_compaction_no_watchdog_intervention() -> TestResult {
    let mut fixture = Fixture::new(3)?;
    fixture.valid_builder();
    let output = Output::default();
    fixture.run(&output).await?;
    assert!(
        !fixture.busy.load(Ordering::Acquire),
        "flag clears after committed success"
    );
    assert!(
        !fixture
            .tokens
            .lock()
            .map_err(|error| error.to_string())?
            .compaction_needed
    );
    let text = serde_json::to_string(&fixture.session.messages)?;
    assert!(
        !text.contains("TIMEOUT"),
        "no synthetic escalation instruction after normal success"
    );
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionFailed { .. })),
        0
    );
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionComplete(_))),
        1
    );
    let mut operation = CompactionOperation::new(Duration::from_secs(1), 2);
    let identity = operation.begin()?;
    operation.committed(identity)?;
    assert_eq!(
        operation.attempts(),
        0,
        "counter resets only after committed success"
    );
    assert!(!operation.is_blocked());
    Ok(())
}

// ========================================
// Scenario: Escalation triggers after first failed attempt
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn first_failure_blocks_without_automatic_escalation() -> TestResult {
    let mut fixture = Fixture::new(5)?;
    let before = fixture.snapshot()?;
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = calls.clone();
    fixture.handler(Arc::new(move |_, _| {
        observed.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Err("builder did not finish".into()) })
    }));
    let output = Output::default();
    assert!(matches!(
        fixture.run(&output).await,
        Err(CompactionFailure::Builder(_))
    ));
    fixture.assert_memory_preserved(&before)?;
    assert!(matches!(
        fixture.run(&output).await,
        Err(CompactionFailure::Blocked)
    ));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "no automatic second request after failure"
    );
    assert_eq!(
        fixture.snapshot()?,
        before,
        "no escalation message is injected"
    );
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionFailed { .. })),
        1
    );
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionComplete(_))),
        0
    );
    Ok(())
}

// ========================================
// Scenario: Escalation message content is directive
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn failed_operation_reports_explicit_resolution_without_model_instructions() -> TestResult {
    let mut fixture = Fixture::new(3)?;
    let before = fixture.snapshot()?;
    fixture.handler(Arc::new(|_, _| {
        Box::pin(async { Err("watchdog builder timed out".into()) })
    }));
    let output = Output::default();
    let error = fixture
        .run(&output)
        .await
        .err()
        .ok_or("builder failure must remain visible")?;
    assert!(error.to_string().contains("timed out"));
    let blocked = fixture
        .run(&output)
        .await
        .err()
        .ok_or("failed operation needs explicit resolution")?;
    assert!(matches!(blocked, CompactionFailure::Blocked));
    assert!(blocked.to_string().contains("explicit resolution"));
    fixture.assert_memory_preserved(&before)?;
    assert_eq!(output.count(|event|matches!(event,StreamEvent::CompactionFailed{reason} if reason.contains("timed out"))),1);
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionComplete(_))),
        0
    );
    Ok(())
}

// ========================================
// Scenario: force_inject_fallback_dag resets session state correctly
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn admitted_compaction_preserves_reminders_and_updates_state() -> TestResult {
    let mut fixture = Fixture::new(10)?;
    fixture.session.messages.insert(
        0,
        Message::User {
            content: OneOrMany::one(UserContent::text(
                "<system-reminder>\n<!-- type:environment -->\nPlatform: test\n</system-reminder>",
            )),
        },
    );
    fixture.session.token_tracker.input_tokens = 50_000;
    fixture
        .tokens
        .lock()
        .map_err(|error| error.to_string())?
        .input_tokens = 50_000;
    fixture.valid_builder();
    let output = Output::default();
    fixture.run(&output).await?;
    let session = &fixture.session;
    let has_env_reminder=session.messages.iter().any(|message|matches!(message,Message::User{content} if content.iter().any(|part|matches!(part,UserContent::Text(text) if text.text.contains("type:environment")))));
    assert!(
        has_env_reminder,
        "Should preserve environment system reminder"
    );
    fixture.assert_typed_summary()?;
    let dag_message=session.messages.iter().find(|message|matches!(message,Message::User{content} if content.iter().any(|part|matches!(part,UserContent::Text(text) if text.text.contains("Overflow arc")))));
    assert!(
        dag_message.is_some(),
        "the committed DAG must be in a user message"
    );
    assert!(
        session.token_tracker.input_tokens > 0,
        "Token tracker should be recalculated"
    );
    assert!(session.token_tracker.input_tokens < 50_000);
    assert!(
        !fixture.busy.load(Ordering::Acquire),
        "compaction_in_progress should be cleared"
    );
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionComplete(_))),
        1
    );
    Ok(())
}

// ========================================
// Scenario: extract_partial_dag_nodes finds dag-node blocks in messages
// ========================================

#[test]
fn test_extract_partial_dag_nodes_finds_blocks() {
    // @step Given session messages containing partial dag-node blocks from assistant responses
    let messages = [
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

I'll call inject_summary next..."#
                    .to_string(),
            })),
        },
    ];

    // @step When extract_partial_dag_nodes is called
    let nodes: Vec<_> = messages
        .iter()
        .flat_map(|message| {
            let Message::Assistant { content, .. } = message else {
                return Vec::new();
            };
            content
                .iter()
                .flat_map(|block| match block {
                    AssistantContent::Text(text) => {
                        codelet_core::compaction::parse_dag_nodes(&text.text, None)
                    }
                    _ => Vec::new(),
                })
                .collect::<Vec<_>>()
        })
        .collect();

    // @step Then it should find and return all dag-node block strings from assistant messages
    assert!(!nodes.is_empty(), "Should find dag-node blocks");
    assert!(
        nodes
            .iter()
            .any(|n| n.label.contains("Architecture Decisions")),
        "Should find first node"
    );
    assert!(
        nodes.iter().any(|n| n.label.contains("Current work")),
        "Should find second node"
    );
}

// ========================================
// Scenario: extract_partial_dag_nodes returns empty when no blocks exist
// ========================================

#[test]
fn test_extract_partial_dag_nodes_returns_empty() {
    // @step Given session messages with no dag-node blocks
    let messages = [
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
    let mut nodes = messages.iter().flat_map(|message| {
        let Message::Assistant { content, .. } = message else {
            return Vec::new();
        };
        content
            .iter()
            .flat_map(|block| match block {
                AssistantContent::Text(text) => {
                    codelet_core::compaction::parse_dag_nodes(&text.text, None)
                }
                _ => Vec::new(),
            })
            .collect::<Vec<_>>()
    });

    // @step Then it should return an empty collection
    assert!(
        nodes.next().is_none(),
        "Should return empty when no dag-nodes found"
    );
}

// ========================================
// Scenario: Force-inject with partial dag-nodes after two failed attempts
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn partial_nodes_require_complete_admitted_source_coverage() -> TestResult {
    let partial = r#"<dag-node depth="D2" turns="0-30" label="Architecture">
- Decisions made
</dag-node>
<dag-node depth="D0" turns="31-50" label="Recent">
- Current work
</dag-node>"#;
    let nodes = codelet_core::compaction::parse_dag_nodes(partial, None);
    assert!(!nodes.is_empty(), "Should have partial dag-nodes");
    assert!(partial.contains("Architecture"));
    assert!(partial.contains("Recent"));
    let mut fixture = Fixture::new(26)?;
    let before = fixture.snapshot()?;
    fixture.handler(Arc::new(move |_, _| {
        Box::pin(async move { Ok(partial.to_owned()) })
    }));
    let output = Output::default();
    let error = fixture
        .run(&output)
        .await
        .err()
        .ok_or("missing persisted turn 51 cannot be force-installed")?;
    assert!(matches!(error,CompactionFailure::Commit(reason) if reason.contains("not covered")));
    fixture.assert_memory_preserved(&before)?;
    assert!(!fixture.busy.load(Ordering::Acquire));
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionComplete(_))),
        0
    );
    // A complete, evidence-backed candidate still succeeds; rejection does not disable compaction.
    let complete=format!("{partial}\n<dag-node depth=\"D0\" turns=\"51-51\" label=\"Last original\">last retained evidence</dag-node>");
    let mut admitted = Fixture::new(26)?;
    let expected = complete.clone();
    admitted.handler(Arc::new(move |_, _| {
        let complete = complete.clone();
        Box::pin(async move { Ok(complete) })
    }));
    let output = Output::default();
    admitted.run(&output).await?;
    assert_eq!(
        codelet_core::persistence::load_session(admitted.id)?
            .compaction
            .ok_or("committed summary missing")?
            .summary,
        expected
    );
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionComplete(_))),
        1
    );
    Ok(())
}

// ========================================
// Scenario: Force-inject with minimal fallback DAG when no partial nodes exist
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn missing_nodes_never_create_minimal_fallback_memory() -> TestResult {
    let mut fixture = Fixture::new(20)?;
    let before = fixture.snapshot()?;
    let nodes = codelet_core::compaction::parse_dag_nodes("no DAG nodes", None);
    assert!(nodes.is_empty(), "No partial nodes exist");
    fixture.handler(Arc::new(|_, _| {
        Box::pin(async { Ok("no DAG nodes".into()) })
    }));
    let output = Output::default();
    assert!(matches!(
        fixture.run(&output).await,
        Err(CompactionFailure::Candidate(_))
    ));
    fixture.assert_memory_preserved(&before)?;
    assert!(!serde_json::to_string(&fixture.session.messages)?.contains("Auto-recovered"));
    assert!(!fixture.busy.load(Ordering::Acquire));
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionComplete(_))),
        0
    );
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionFailed { .. })),
        1
    );
    Ok(())
}

// ========================================
// Scenario: Level-3 fallback shape is built from the shared
// compaction_dag helpers (CMPCT-047)
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn runtime_has_no_level_three_fallback_or_diagnostic_salvage() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("Rust workspace root")?;
    let agent = std::fs::read_to_string(root.join("agent-loop/src/agent_loop.rs"))?;
    let compactor = std::fs::read_to_string(root.join("cli/src/compactor_sub_agent.rs"))?;
    for removed in [
        "extract_partial_dag_nodes(",
        "force_inject_fallback_dag(",
        "build_generic_fallback_dag_node(",
        "pin_fallback_dag(",
    ] {
        assert!(
            !agent.contains(removed),
            "removed fallback helper {removed} returned to the agent loop"
        );
        assert!(
            !compactor.contains(removed),
            "removed fallback helper {removed} returned to the compactor"
        );
    }
    assert!(
        !agent.contains("r#\"<dag-node"),
        "watchdog must not invent an inline generic DAG template"
    );
    let mut fixture = Fixture::new(3)?;
    let before = fixture.snapshot()?;
    let diagnostic = fixture.dag();
    fixture.handler(Arc::new(move |_, _| {
        let diagnostic = diagnostic.clone();
        Box::pin(async move { Err(format!("builder failed: {diagnostic}")) })
    }));
    let output = Output::default();
    assert!(matches!(
        fixture.run(&output).await,
        Err(CompactionFailure::Builder(_))
    ));
    fixture.assert_memory_preserved(&before)?;
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionComplete(_))),
        0
    );
    Ok(())
}
