//! Features: spec/features/generate-compaction-tool.feature,
//! spec/features/compaction-inference-handoff.feature

use codelet_tools::{
    set_generate_compaction_handler, GenerateCompactionArgs, GenerateCompactionHandler,
    GenerateCompactionTool, ToolError,
};
use rig::tool::Tool;
use std::sync::Arc;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
#[serial_test::serial]
async fn self_target_generation_reports_queued_application() -> TestResult {
    let caller = Uuid::new_v4();
    let dag = "<dag-node depth=\"D1\" turns=\"0-0\" label=\"work\">memory</dag-node>";
    let handler: GenerateCompactionHandler = Arc::new(move |target| {
        assert_eq!(target, caller);
        Box::pin(async move { Ok(dag.into()) })
    });
    set_generate_compaction_handler(caller, Some(handler));
    for session_id in [None, Some(caller.to_string())] {
        let result = GenerateCompactionTool::new(caller)
            .call(GenerateCompactionArgs { session_id })
            .await;
        let encoded = result?;
        let output: serde_json::Value = serde_json::from_str(&encoded)?;
        assert_eq!(output["status"], "queued");
        assert_eq!(output["content"], dag);
        assert!(output.get("remaining_budget").is_none());
    }
    set_generate_compaction_handler(caller, None);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn cross_target_dispatch_uses_callers_handler_and_preserves_result() -> TestResult {
    let caller = Uuid::new_v4();
    let target = Uuid::new_v4();
    let dag = "<dag-node depth=\"D1\" turns=\"0-0\" label=\"target\">memory</dag-node>";
    let handler: GenerateCompactionHandler = Arc::new(move |resolved| {
        assert_eq!(resolved, target);
        Box::pin(async move { Ok(dag.into()) })
    });
    set_generate_compaction_handler(caller, Some(handler));
    let result = GenerateCompactionTool::new(caller)
        .call(GenerateCompactionArgs {
            session_id: Some(target.to_string()),
        })
        .await;
    set_generate_compaction_handler(caller, None);
    assert_eq!(result?, dag);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn invalid_target_never_dispatches_to_the_handler() {
    let caller = Uuid::new_v4();
    let dispatched = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observed = dispatched.clone();
    let handler: GenerateCompactionHandler = Arc::new(move |_| {
        observed.store(true, std::sync::atomic::Ordering::SeqCst);
        Box::pin(async { Ok("unexpected dispatch".into()) })
    });
    set_generate_compaction_handler(caller, Some(handler));
    let result = GenerateCompactionTool::new(caller)
        .call(GenerateCompactionArgs {
            session_id: Some("not-a-uuid".into()),
        })
        .await;
    set_generate_compaction_handler(caller, None);
    assert!(matches!(result, Err(ToolError::Validation { .. })));
    assert!(!dispatched.load(std::sync::atomic::Ordering::SeqCst));
}

#[tokio::test]
#[serial_test::serial]
async fn handler_rejection_and_missing_handler_are_not_queued_success() {
    let caller = Uuid::new_v4();
    let tool = GenerateCompactionTool::new(caller);
    let missing = tool.call(GenerateCompactionArgs { session_id: None }).await;
    assert!(matches!(missing, Err(ToolError::Execution { .. })));
    let handler: GenerateCompactionHandler =
        Arc::new(|_| Box::pin(async { Err("captured candidate rejected".into()) }));
    set_generate_compaction_handler(caller, Some(handler));
    let rejected = tool.call(GenerateCompactionArgs { session_id: None }).await;
    set_generate_compaction_handler(caller, None);
    assert!(matches!(
        rejected,
        Err(ToolError::Execution { message, .. }) if message == "captured candidate rejected"
    ));
}
