//! Feature: spec/features/mandatory-tool-work-unit-admission.feature
#[path = "tool026_admission/auxiliary.rs"]
mod auxiliary;
#[path = "tool026_admission/delegation.rs"]
mod delegation;
#[path = "bug231_support/mod.rs"]
pub mod fixture;
#[path = "tool026_admission/parser_properties.rs"]
mod parser_properties;
#[path = "tool026_admission/provider_execution.rs"]
mod provider_execution;
#[path = "tool026_admission/routes.rs"]
mod routes;
#[path = "tool026_admission/scenarios.rs"]
mod scenarios;
#[path = "tool026_admission/support.rs"]
mod support;
use codelet_core::session_manager_handle::SessionManagerHandle;
use fixture::Fixture;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use support::{args, server, Result};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_valid_attribution_changes_caller_focus_before_execution_and_survives_tool_failure(
) -> Result {
    // @step Given two sessions focused on different cards
    let fixture = Fixture::new().await?;
    let other = fixture.manager.get_work_unit_focus(&fixture.second)?;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let actor = scenarios::valid_probe(&fixture, calls.clone()).await?;
    // @step When one invokes a tool with valid attribution for another project card
    let mut input: Value = serde_json::from_str(&args("AUTH-003"))?;
    input["fail"] = json!(true);
    let error = actor
        .call_tool("probe", &input.to_string())
        .await
        .err()
        .ok_or("expected execution failure")?;
    // @step Then the executor observes that caller's updated focus before performing work
    let recorded = calls.lock().map_err(|_| "calls")?;
    let receipt = recorded[0].1.as_ref().ok_or("missing invoking receipt")?;
    assert_eq!(receipt.session_id.to_string(), fixture.first.value);
    assert_eq!(
        receipt.focus_revision,
        fixture
            .manager
            .get_work_unit_focus(&fixture.first)?
            .revision
    );
    assert_eq!(fixture.focus(&fixture.first)?.as_deref(), Some("AUTH-003"));
    // @step And the other session remains unchanged
    assert_eq!(fixture.manager.get_work_unit_focus(&fixture.second)?, other);
    // @step And a subsequent execution error does not roll back admitted activity
    assert!(error.to_string().contains("execution failed"), "{error}");
    assert_eq!(fixture.focus(&fixture.first)?.as_deref(), Some("AUTH-003"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_invalid_attribution_cannot_mutate_focus_or_execute_a_tool() -> Result {
    // @step Given an attributed tool executor with recorded calls and existing focus
    let fixture = Fixture::new().await?;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let actor = server(&fixture.first.value, calls.clone(), None).await?;
    let before = fixture.manager.get_work_unit_focus(&fixture.first)?;
    let mut events = fixture.manager.work_unit_focus_rx();
    // @step When calls omit metadata or provide null empty unknown or cross-project attribution
    let mut outcomes = Vec::new();
    for input in scenarios::invalid_inputs() {
        outcomes.push(actor.call_tool("probe", &input.to_string()).await);
    }
    for input in [
        r#"{"work_unit_id":"NONE","work_unit_id":"AUTH-003"}"#,
        r#"{"work_unit_id":"NONE","work_unit_id":"NONE"}"#,
    ] {
        outcomes.push(actor.call_tool("probe", input).await);
    }
    // @step Then each call returns an actionable validation error
    for outcome in outcomes {
        let error = outcome.err().ok_or("invalid attribution executed")?;
        assert!(error.to_string().contains("work_unit_id"), "{error}");
    }
    parser_properties::verify()?;
    // @step And no underlying tool is invoked and no focus event is emitted
    assert!(calls.lock().map_err(|_| "calls")?.is_empty());
    assert!(events.try_recv().is_err());
    assert_eq!(fixture.manager.get_work_unit_focus(&fixture.first)?, before);
    // @step And a user-supplied session or project target cannot redirect attribution ownership
    let mut input: Value = serde_json::from_str(&args("AUTH-003"))?;
    input["session_id"] = json!(fixture.second.value);
    input["project_root"] = json!("/untrusted");
    actor.call_tool("probe", &input.to_string()).await?;
    assert_eq!(fixture.focus(&fixture.first)?.as_deref(), Some("AUTH-003"));
    assert_eq!(fixture.focus(&fixture.second)?.as_deref(), Some("AUTH-002"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_none_clears_activity_without_weakening_permissions() -> Result {
    // @step Given a session with stage restrictions and isolated-worktree boundaries
    let fixture = Fixture::new().await?;
    let actor = scenarios::restricted_writer(&fixture).await?;
    let output = fixture.project.path().join("production.rs");
    // @step When it invokes a tool attributed to "NONE"
    let input = json!({"work_unit_id":"NONE","file_path":output,"content":"forbidden"});
    let result = actor.call_tool("Write", &input.to_string()).await;
    // @step Then its activity is explicitly cleared and the session remains alive
    assert!(fixture.focus(&fixture.first)?.is_none());
    let session = fixture.manager.get_session(&fixture.first.value)?;
    // @step And existing stage and path restrictions remain enforced
    assert!(result.is_err());
    assert!(!output.exists());
    assert_eq!(
        codelet_sessions::session_tool_callbacks::work_unit_stage(fixture.first.value.clone())
            .as_deref(),
        Some("specifying")
    );
    scenarios::verify_path_restriction(&fixture, &actor, &output).await?;
    // @step And selecting a more permissive unrelated card does not replace its authorization binding
    assert_eq!(
        session
            .get_work_unit_context()
            .and_then(|context| context.id)
            .as_deref(),
        Some("AUTH-001")
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_metadata_is_removed_before_strict_delegation_while_audit_arguments_are_retained(
) -> Result {
    // @step Given strict MCP typed dynamic and Rhai-mapped tools
    let fixture = Fixture::new().await?;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let actor = server(&fixture.first.value, calls.clone(), None).await?;
    let original = args("AUTH-003");
    // @step When each receives a valid attributed call with nested business arguments
    let output: Value = serde_json::from_str(&actor.call_tool("probe", &original).await?)?;
    routes::strict_delegation(&fixture).await?;
    // @step Then each executor receives exactly its original business payload without top-level "work_unit_id"
    let mut expected: Value = serde_json::from_str(&original)?;
    expected
        .as_object_mut()
        .ok_or("object")?
        .remove("work_unit_id");
    assert_eq!(output, expected);
    assert_eq!(calls.lock().map_err(|_| "calls")?[0].0, expected);
    // @step And nested business "work_unit_id" and "target_work_unit_id" arguments are unchanged
    assert_eq!(output["target_work_unit_id"], "business-target");
    assert_eq!(output["payload"]["nested"][1]["work_unit_id"], "business");
    // @step And original attributed arguments remain available for call history and correlation
    assert_eq!(
        serde_json::from_str::<Value>(&original)?["work_unit_id"],
        "AUTH-003"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_invocation_order_rather_than_result_completion_determines_activity() -> Result {
    // @step Given calls admitted for "AUTH-001" then "AUTH-002" followed by "NONE"
    let fixture = Fixture::new().await?;
    let pending = routes::admit_ordered_calls(&fixture).await?;
    // @step When the earlier calls finish late fail or return duplicate results
    pending.gate.notify_one();
    assert!(pending.task.await?.is_err());
    // @step Then the final activity remains explicitly cleared
    assert!(fixture.focus(&fixture.first)?.is_none());
    // @step And late status-command reconciliation cannot resurrect an older card
    fixture
        .manager
        .reconcile_status_success(&pending.receipt, "AUTH-001")?;
    fixture
        .manager
        .reconcile_status_success(&pending.receipt, "AUTH-001")?;
    assert!(fixture.focus(&fixture.first)?.is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_child_targets_and_auxiliary_agents_do_not_steal_another_sessions_focus() -> Result
{
    // @step Given a parent session a managed child and an ephemeral read-only research agent
    let fixture = Fixture::new().await?;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let actor = server(&fixture.first.value, calls.clone(), None).await?;
    // @step When the parent invokes a tool targeting its child and the research agent invokes its tools
    let mut input: Value = serde_json::from_str(&args("AUTH-003"))?;
    input["session_id"] = json!(fixture.second.value);
    actor.call_tool("probe", &input.to_string()).await?;
    let before = fixture.manager.get_work_unit_focus(&fixture.first)?;
    auxiliary::verify(&fixture).await?;
    // @step Then attribution belongs to the trusted invoking context rather than the target argument
    assert_eq!(fixture.focus(&fixture.first)?.as_deref(), Some("AUTH-003"));
    assert_eq!(fixture.focus(&fixture.second)?.as_deref(), Some("AUTH-002"));
    // @step And research activity does not change the owner's visible card
    assert_eq!(fixture.manager.get_work_unit_focus(&fixture.first)?, before);
    // @step And unknown invoking contexts cannot silently bypass required validation
    let unknown = server(&uuid::Uuid::new_v4().to_string(), calls, None).await?;
    assert!(unknown.call_tool("probe", &args("NONE")).await.is_err());
    assert_eq!(fixture.manager.get_work_unit_focus(&fixture.first)?, before);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_hot_added_tools_and_completion_tools_enforce_admission_as_ordinary_tools(
) -> Result {
    // @step Given an agent with dynamically registered tools and a completion contract
    let fixture = Fixture::new().await?;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let actor = routes::hot_tools(&fixture, calls.clone()).await?;
    // @step When a new tool or completion call omits required attribution
    let invalid = actor.call_tool("probe", "{}").await;
    let completion = actor
        .call_tool("done", r#"{"summary":"must not signal completion"}"#)
        .await;
    // @step Then admission rejects the call before its side effects or completion signal
    assert!(invalid.is_err());
    assert!(completion.is_err());
    assert!(
        codelet_tools::take_done_acceptance(uuid::Uuid::parse_str(&fixture.first.value)?).is_none()
    );
    assert!(calls.lock().map_err(|_| "calls")?.is_empty());
    // @step And valid attributed calls use the same enforcement after a provider switch
    let rebuilt = server(&fixture.first.value, calls, None).await?;
    rebuilt.call_tool("probe", &args("NONE")).await?;
    assert!(fixture.focus(&fixture.first)?.is_none());
    provider_execution::verify(&fixture).await?;
    Ok(())
}
