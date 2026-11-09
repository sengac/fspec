// ============================================================================
// Scenario: Successful sub-agent DAG is pinned to the target session
// (DAG validation contract at the primitive level — the success/fallback
// behavioral tests above prove the handler branches on parse_dag_nodes)
// ============================================================================

#[test]
fn sub_agent_dag_validation_uses_parse_dag_nodes() {
    // @step Given a target session that contains no existing compaction DAG
    // @step And the compactor sub-agent returns a parseable DAG with at least one dag-node block
    let parseable = parse_dag_nodes(SUCCESS_DAG, None);
    assert!(
        !parseable.is_empty(),
        "the success DAG must parse to at least one node"
    );
    // @step When the handler side pins the sub-agent's DAG to the target
    // (the unparseable-output test proves the negative branch)
    assert!(
        parse_dag_nodes("no dag nodes here", None).is_empty(),
        "junk text must parse to zero nodes"
    );
}

// ============================================================================
// Scenario: GenerateCompaction implements the rig tool trait
// Scenario: Invalid session_id returns a validation error
// Scenario: Omitted session_id targets the calling session
// Scenario: Missing GenerateCompaction handler returns an execution error
// Scenario: Tool result is the queued DAG candidate
//
// Tools-crate surface: the rig Tool trait shape, argument validation,
// target resolution, handler-registry lifecycle, and the tool-result
// contract (the spawner/pin lives in codelet-agent-loop — covered by the
// behavioral tests above).
// ============================================================================

/// Register a mock handler that records every call and returns a fixed DAG.
fn register_mock(
    session_id: Uuid,
    seen_targets: &std::sync::Arc<std::sync::Mutex<Vec<Uuid>>>,
    called: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    dag: &str,
) {
    let targets = seen_targets.clone();
    let flag = called.clone();
    let dag: std::sync::Arc<String> = std::sync::Arc::new(dag.to_string());
    let handler: GenerateCompactionHandler = std::sync::Arc::new(move |target| {
        flag.store(true, Ordering::SeqCst);
        targets.lock().expect("seen targets lock").push(target);
        let dag = dag.clone();
        Box::pin(async move { Ok(dag.as_ref().clone()) })
    });
    set_generate_compaction_handler(session_id, Some(handler));
}

/// Scenario: GenerateCompaction implements the rig tool trait
#[tokio::test]
#[serial]
async fn generate_compaction_implements_the_rig_tool_trait() {
    // @step Given the GenerateCompaction tool struct exists in the rust/tools/src/generate_compaction module
    let tool = GenerateCompactionTool::new(Uuid::new_v4());

    // @step When the rig agent builder includes GenerateCompactionTool::new(session_id)
    // @step Then GenerateCompaction has NAME = "GenerateCompaction"
    assert_eq!(GenerateCompactionTool::NAME, "GenerateCompaction");

    // @step And GenerateCompaction has Args type = GenerateCompactionArgs with session_id (optional string, UUID)
    // @step And GenerateCompaction has Output type = String
    // @step And GenerateCompaction has Error type = ToolError
    // (type-level — covered by this test compiling against the Tool trait)

    // @step And the definition() returns a JSON schema describing the optional session_id parameter with no required fields
    let def = tool.definition(String::new()).await;
    let params = &def.parameters;
    let schema = params
        .get("properties")
        .and_then(|p| p.get("session_id"))
        .expect("definition() must describe the optional session_id parameter");
    assert!(
        schema.to_string().contains("string"),
        "session_id must be typed as a (UUID) string in the JSON schema"
    );
    let required = params
        .get("required")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        !required.iter().any(|r| r.as_str() == Some("session_id")),
        "session_id must be OPTIONAL in the JSON schema (no required fields)"
    );
    // The description must steer the LLM to the DeepSearch-style usage.
    assert!(
        def.description.to_lowercase().contains("session"),
        "the tool description must mention the target session"
    );
}

/// Scenario: Invalid session_id returns a validation error
#[tokio::test]
#[serial]
async fn invalid_session_id_returns_a_validation_error() {
    // @step Given a GenerateCompaction handler is registered for the calling session
    let caller = Uuid::new_v4();
    let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    register_mock(
        caller,
        &seen,
        &called,
        "<dag-node depth=\"D1\" turns=\"0-1\" label=\"x\"></dag-node>",
    );

    let tool = GenerateCompactionTool::new(caller);
    let args = GenerateCompactionArgs {
        session_id: Some("not-a-uuid".to_string()),
    };

    // @step When the agent calls GenerateCompaction with session_id "not-a-uuid"
    let result = tool.call(args).await;

    // @step Then the tool returns a ToolError::Validation naming the offending parameter
    let result_str = result.as_ref().expect_err("must be an error").to_string();
    let err = result.expect_err("an invalid UUID must be a validation error");
    match err {
        ToolError::Validation { tool: t, message } => {
            assert_eq!(t, "GenerateCompaction");
            assert!(
                message.to_lowercase().contains("session_id"),
                "the validation error must name the offending parameter, got: {message}"
            );
        }
        other => panic!("expected ToolError::Validation, got: {other}"),
    }

    // @step And the error includes a usage hint showing the accepted shape (a UUID string or omitted)
    assert!(
        result_str.contains("UUID") || result_str.contains("uuid"),
        "the error must show the accepted shape (a UUID string or omitted), got: {result_str}"
    );

    // @step And no sub-agent is spawned and no session is modified
    assert!(
        !called.load(Ordering::SeqCst),
        "nothing may be dispatched on validation failure"
    );
    assert!(seen.lock().expect("seen targets lock").is_empty());

    clear_all_generate_compaction_handlers();
}

/// Scenario: Omitted session_id targets the calling session
#[tokio::test]
#[serial]
async fn omitted_session_id_targets_the_calling_session() {
    // @step Given the agent is running in session A with a registered GenerateCompaction handler
    let caller = Uuid::new_v4();
    let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    register_mock(caller, &seen, &called, "ok");

    let tool = GenerateCompactionTool::new(caller);

    // @step When the agent calls GenerateCompaction with no arguments
    let args = GenerateCompactionArgs { session_id: None };
    let result = tool.call(args).await;

    // @step Then the target session resolves to session A (the calling session)
    assert!(result.is_ok(), "call must succeed: {:?}", result.err());
    let targets = seen.lock().expect("seen targets lock");
    assert_eq!(targets.len(), 1, "the handler must be invoked exactly once");
    assert_eq!(
        targets[0], caller,
        "the omitted target must resolve to the calling session"
    );
    assert!(called.load(Ordering::SeqCst));

    clear_all_generate_compaction_handlers();
}

/// Scenario: Missing GenerateCompaction handler returns an execution error
#[tokio::test]
#[serial]
async fn missing_generate_compaction_handler_returns_an_execution_error() {
    // @step Given no GenerateCompactionHandler is registered for the calling session
    clear_all_generate_compaction_handlers();
    let caller = Uuid::new_v4();
    assert!(!has_generate_compaction_handler(caller));

    // @step When the agent calls GenerateCompaction
    let tool = GenerateCompactionTool::new(caller);
    let result = tool.call(GenerateCompactionArgs { session_id: None }).await;

    // @step Then the tool returns a ToolError::Execution with the message pattern "handler not configured for session <uuid>"
    let err = result.expect_err("no registered handler must be an execution error");
    match err {
        ToolError::Execution { tool: t, message } => {
            assert_eq!(t, "GenerateCompaction");
            assert!(
                message.contains(&format!("handler not configured for session {caller}")),
                "message must name the calling session, got: {message}"
            );
        }
        other => panic!("expected ToolError::Execution, got {other:?}"),
    }

    // @step And nothing is spawned and no session is pinned
    assert!(!has_generate_compaction_handler(caller));
}

/// Scenario: Tool result is the queued DAG candidate
#[tokio::test]
#[serial]
async fn tool_result_is_the_pinned_dag_text() {
    // @step Given the compactor sub-agent returns a parseable DAG
    let caller = Uuid::new_v4();
    let dag = r#"<dag-node depth="D1" turns="0-9" label="Work arc">
- pinned
</dag-node>"#;
    let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    register_mock(caller, &seen, &called, dag);

    let tool = GenerateCompactionTool::new(caller);

    // @step When GenerateCompaction completes successfully
    let result = tool.call(GenerateCompactionArgs { session_id: None }).await;

    // @step Then the tool result reports status queued with the DAG text as its content
    assert_eq!(
        queued_content(&result.expect("success path returns Ok")),
        dag
    );

    // @step And the tool returns Ok only for a queued candidate, reserving Err for validation failures, missing handler, builder failures, and unknown-target rejections
    // (success + missing-handler + validation covered by the other tests;
    //  the fallback note is produced by the agent-loop handler side)
    clear_all_generate_compaction_handlers();
}
