use super::*;
use serial_test::serial;
use std::sync::atomic::{AtomicBool, Ordering};

#[tokio::test]
#[serial]
async fn test_tool_definition_name_and_schema() {
    // @step Given the InjectSummaryTool is compiled as part of the codelet-tools crate
    let tool = InjectSummaryTool::new(Uuid::new_v4());
    // @step When the tool definition is requested
    let definition = tool.definition(String::new()).await;
    // @step Then the tool name should be "inject_summary"
    assert_eq!(definition.name, "inject_summary");
    // @step And the JSON schema should have "content" as a required string parameter
    assert_eq!(
        definition.parameters["properties"]["content"]["type"],
        "string"
    );
    let required = definition.parameters["required"]
        .as_array()
        .expect("required schema parameters must be an array");
    assert!(required
        .iter()
        .any(|value| value.as_str() == Some("content")));
    // @step And the description should explain DAG pinning, builder turn dropping, and compaction persistence
    assert!(definition.description.contains("DAG summary"));
    assert!(definition.description.contains("builder turns"));
    assert!(definition.description.contains("compaction"));
}

#[tokio::test]
#[serial]
async fn test_successful_dispatch_to_handler() {
    clear_all_inject_summary_handlers();
    let session_id = Uuid::new_v4();
    let called = Arc::new(AtomicBool::new(false));
    let called_clone = called.clone();
    // @step Given a session with a registered InjectSummaryHandler
    let handler: InjectSummaryHandler = Arc::new(move |received_id, content| {
        called_clone.store(true, Ordering::SeqCst);
        assert_eq!(received_id, session_id);
        assert!(content.contains("D2: Architecture"));
        Ok(InjectSummaryResult {
            injected_tokens: 1250,
            remaining_budget: 185000,
        })
    });
    set_inject_summary_handler(session_id, Some(handler));
    // @step When the agent calls inject_summary with DAG content
    let output = InjectSummaryTool::new(session_id)
        .call(InjectSummaryArgs {
            content: "# D2: Architecture\n- Using JWT...".into(),
        })
        .await;
    // @step Then the handler receives the session_id and content string
    assert!(called.load(Ordering::SeqCst));
    // @step And the tool returns a queued summary estimate without a remaining request budget
    let output = output.expect("registered handler must queue the summary");
    let result: serde_json::Value =
        serde_json::from_str(&output).expect("queued summary result must contain valid JSON");
    assert_eq!(result["injected_tokens"], 1250);
    assert_eq!(result["status"], "queued");
    assert!(result.get("remaining_budget").is_none());
    clear_all_inject_summary_handlers();
}

#[tokio::test]
#[serial]
async fn test_no_handler_returns_error() {
    clear_all_inject_summary_handlers();
    // @step Given no InjectSummaryHandler is registered for the session
    let session_id = Uuid::new_v4();
    // @step When the agent calls inject_summary
    let result = InjectSummaryTool::new(session_id)
        .call(InjectSummaryArgs {
            content: "some content".into(),
        })
        .await;
    // @step Then the tool returns a ToolError with message containing "not configured"
    let error = result.expect_err("an unregistered session must reject summary injection");
    let message = error.to_string();
    assert!(message.contains("not configured"), "{message}");
    // @step And the error message includes the session UUID
    assert!(message.contains(&session_id.to_string()), "{message}");
    clear_all_inject_summary_handlers();
}

#[tokio::test]
#[serial]
async fn test_concurrent_sessions_isolated() {
    clear_all_inject_summary_handlers();
    // @step Given Session A and Session B each have a registered InjectSummaryHandler
    let session_a = Uuid::new_v4();
    let session_b = Uuid::new_v4();
    let a_called = Arc::new(AtomicBool::new(false));
    let b_called = Arc::new(AtomicBool::new(false));
    let a_called_clone = a_called.clone();
    let b_called_clone = b_called.clone();
    set_inject_summary_handler(
        session_a,
        Some(Arc::new(move |_, _| {
            a_called_clone.store(true, Ordering::SeqCst);
            Ok(InjectSummaryResult {
                injected_tokens: 100,
                remaining_budget: 200000,
            })
        })),
    );
    set_inject_summary_handler(
        session_b,
        Some(Arc::new(move |_, _| {
            b_called_clone.store(true, Ordering::SeqCst);
            Ok(InjectSummaryResult {
                injected_tokens: 200,
                remaining_budget: 180000,
            })
        })),
    );
    // @step When inject_summary is called on Session A
    let output = InjectSummaryTool::new(session_a)
        .call(InjectSummaryArgs {
            content: "content".into(),
        })
        .await
        .expect("session A's registered handler must queue its summary");
    // @step Then only Session A's handler is invoked
    assert!(a_called.load(Ordering::SeqCst));
    let result: serde_json::Value =
        serde_json::from_str(&output).expect("session A's queued result must contain valid JSON");
    assert_eq!(result["injected_tokens"], 100);
    // @step And Session B's handler is not affected
    assert!(!b_called.load(Ordering::SeqCst));
    clear_all_inject_summary_handlers();
}

#[tokio::test]
#[serial]
async fn test_handler_removal_with_none() {
    clear_all_inject_summary_handlers();
    let session_id = Uuid::new_v4();
    // @step Given a session with a registered InjectSummaryHandler
    set_inject_summary_handler(
        session_id,
        Some(Arc::new(|_, _| {
            Ok(InjectSummaryResult {
                injected_tokens: 500,
                remaining_budget: 190000,
            })
        })),
    );
    assert!(has_inject_summary_handler(session_id));
    // @step When set_inject_summary_handler is called with None for that session
    set_inject_summary_handler(session_id, None);
    // @step Then has_inject_summary_handler returns false for that session
    assert!(!has_inject_summary_handler(session_id));
    // @step And subsequent inject_summary calls return an error
    assert!(InjectSummaryTool::new(session_id)
        .call(InjectSummaryArgs {
            content: "content".into()
        })
        .await
        .is_err());
    clear_all_inject_summary_handlers();
}

#[tokio::test]
#[serial]
async fn test_tool_constructed_with_session_id() {
    clear_all_inject_summary_handlers();
    let session_id = Uuid::new_v4();
    let received_id = Arc::new(RwLock::new(None::<Uuid>));
    let received_id_clone = received_id.clone();
    // @step Given an InjectSummaryTool created with a specific session_id
    set_inject_summary_handler(
        session_id,
        Some(Arc::new(move |received, _| {
            *received_id_clone
                .write()
                .expect("captured session ID lock must be writable") = Some(received);
            Ok(InjectSummaryResult {
                injected_tokens: 300,
                remaining_budget: 195000,
            })
        })),
    );
    let tool = InjectSummaryTool::new(session_id);
    // @step When call() is invoked with InjectSummaryArgs
    let output = tool
        .call(InjectSummaryArgs {
            content: "test content".into(),
        })
        .await
        .expect("tool must dispatch to its session's registered handler");
    // @step Then the tool looks up the handler using its stored session_id
    assert_eq!(
        *received_id
            .read()
            .expect("captured session ID lock must be readable"),
        Some(session_id)
    );
    // @step And the correct per-session handler is dispatched
    let result: serde_json::Value = serde_json::from_str(&output)
        .expect("queued session-specific result must contain valid JSON");
    assert_eq!(result["injected_tokens"], 300);
    assert_eq!(result["status"], "queued");
    clear_all_inject_summary_handlers();
}

#[test]
fn test_types_exist_and_are_public() {
    // @step Given the codelet-tools crate lib.rs module declarations
    // @step When inject_summary module is declared
    // @step Then InjectSummaryTool is publicly re-exported
    let _tool = crate::InjectSummaryTool::new(Uuid::new_v4());
    // @step And InjectSummaryHandler type alias is publicly re-exported
    let _handler: crate::InjectSummaryHandler = Arc::new(|_, _| {
        Ok(InjectSummaryResult {
            injected_tokens: 0,
            remaining_budget: 0,
        })
    });
    // @step And set_inject_summary_handler function is publicly re-exported
    crate::set_inject_summary_handler(Uuid::new_v4(), None);
    // @step And has_inject_summary_handler function is publicly re-exported
    assert!(!crate::has_inject_summary_handler(Uuid::new_v4()));
    // @step And clear_all_inject_summary_handlers function is publicly re-exported
    let _clear: fn() = crate::clear_all_inject_summary_handlers;
}
