//! Feature: spec/features/agent-manager-caller-attribution.feature

#[path = "bug231_support/mod.rs"]
mod fixture;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use fixture::{Fixture, Result};
use rig::tool::server::ToolServerHandle;
use serde_json::{json, Value};
use std::sync::Arc;

#[path = "support/bug240_management.rs"]
mod support;
use support::{server, Calls};

async fn manage(server: &ToolServerHandle, input: Value) -> Result<Value> {
    let serialized: String =
        serde_json::from_str(&server.call_tool("AgentManager", &input.to_string()).await?)?;
    let output: Value = serde_json::from_str(&serialized)?;
    assert_ne!(output["error"], true, "{output}");
    Ok(output)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_spawning_a_worker_does_not_assign_caller_attribution_to_the_child() -> Result {
    // @step Given a caller focused on AUTH-003 with authorization bound to AUTH-001
    let fixture = Fixture::new().await?;
    let parent = fixture.manager.get_session(&fixture.first.value)?;
    parent.set_work_unit_context(
        Some("AUTH-001".into()),
        Some("bound".into()),
        Some("implementing".into()),
    );
    fixture
        .manager
        .set_session_focus(&fixture.first, Some("AUTH-003"), "setup")?;
    let (actor, _parent_handler) = server(&fixture, &fixture.first, Arc::default()).await?;
    // @step When it spawns a worker with attribution AUTH-001 and role text delegating AUTH-002
    let spawned = manage(
        &actor,
        json!({"action":"spawn","work_unit_id":"AUTH-001",
        "role":"Work on AUTH-002"}),
    )
    .await?;
    let child = SessionId::from(spawned["session_id"].as_str().ok_or("child ID")?.to_owned());
    // @step Then the caller focuses AUTH-001 and the new worker has cleared activity
    assert_eq!(fixture.focus(&fixture.first)?.as_deref(), Some("AUTH-001"));
    assert!(fixture.focus(&child)?.is_none());
    assert_eq!(
        fixture.manager.get_session(&child.value)?.get_role(),
        Some("Work on AUTH-002".into())
    );
    // @step And the worker selects AUTH-002 only through its own admitted call without changing the caller
    let before = fixture.manager.get_work_unit_focus(&fixture.first)?;
    let (worker, _child_handler) = server(&fixture, &child, Arc::default()).await?;
    manage(&worker, json!({"action":"list","work_unit_id":"AUTH-002"})).await?;
    assert_eq!(fixture.focus(&child)?.as_deref(), Some("AUTH-002"));
    assert_eq!(fixture.manager.get_work_unit_focus(&fixture.first)?, before);
    // @step And caller authorization and worktree bindings remain unchanged
    assert_eq!(
        parent
            .get_work_unit_context()
            .and_then(|context| context.id)
            .as_deref(),
        Some("AUTH-001")
    );
    assert_eq!(
        parent.worktree_path(),
        Some(fixture.project.path().to_owned())
    );
    manage(
        &actor,
        json!({"action":"close","session_id":child.value,"work_unit_id":"AUTH-001"}),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn scenario_published_definitions_distinguish_caller_attribution_from_delegation() -> Result {
    use codelet_tools::{
        agent_manager::AgentManagerTool, tool_attribution::configure_tool_server, ReadTool,
    };
    use rig::tool::server::ToolServer;
    use uuid::Uuid;

    // @step Given an agent requesting ordinary and AgentManager tool definitions
    let caller = Uuid::new_v4();
    let server = ToolServer::new()
        .tool(ReadTool::new(caller))
        .tool(AgentManagerTool::new(caller))
        .run();
    configure_tool_server(&server, caller.to_string()).await?;
    // @step When the model-visible definitions are published
    let definitions = server.get_tool_defs(None).await?;
    assert_eq!(definitions.len(), 2);
    // @step Then root attribution explicitly names the calling session and excludes spawned or targeted sessions
    for definition in &definitions {
        let description = definition.parameters["properties"]["work_unit_id"]["description"]
            .as_str()
            .ok_or("attribution description")?;
        for instruction in ["calling session", "never", "spawned", "targeted"] {
            assert!(description.contains(instruction), "{description}");
        }
    }
    // @step And AgentManager explains orchestration on AUTH-001 while delegating AUTH-002 and workers attributing their own calls
    let manager = definitions
        .iter()
        .find(|definition| definition.name == "AgentManager")
        .ok_or("AgentManager definition")?;
    for instruction in [
        "AUTH-001",
        "AUTH-002",
        "role",
        "message",
        "own calls",
        "does not assign",
    ] {
        assert!(
            manager.description.contains(instruction),
            "{}",
            manager.description
        );
    }
    assert!(manager.parameters["properties"]
        .get("child_work_unit_id")
        .is_none());
    // @step And required validation NONE clearing and permission separation remain documented
    for definition in definitions {
        assert!(definition.parameters["required"]
            .as_array()
            .is_some_and(|required| required.contains(&json!("work_unit_id"))));
        let property = &definition.parameters["properties"]["work_unit_id"];
        assert_eq!(property["type"], "string");
        assert_eq!(property["minLength"], 1);
        let description = property["description"].as_str().ok_or("description")?;
        for instruction in [
            "NONE",
            "clear",
            "not retain",
            "null",
            "invalid",
            "permission",
        ] {
            assert!(description.contains(instruction), "{description}");
        }
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_targeted_management_preserves_session_isolation_and_strict_admission() -> Result {
    // @step Given a caller and worker focused on different project cards
    let fixture = Fixture::new().await?;
    let parent = fixture.manager.get_session(&fixture.first.value)?;
    parent.set_work_unit_context(
        Some("AUTH-001".into()),
        Some("bound".into()),
        Some("implementing".into()),
    );
    let calls = Calls::default();
    let (actor, _handler) = server(&fixture, &fixture.first, calls.clone()).await?;
    let target_before = fixture.manager.get_work_unit_focus(&fixture.second)?;
    // @step When the caller manages the worker using explicit attribution and NONE
    for attribution in ["AUTH-003", "NONE"] {
        for mut input in [
            json!({"action":"get_status"}),
            json!({"action":"set_role","role":"Work on AUTH-003"}),
            json!({"action":"message","message":"Work on AUTH-003"}),
        ] {
            input["session_id"] = json!(fixture.second.value);
            input["work_unit_id"] = json!(attribution);
            let is_message = input["action"] == "message";
            manage(&actor, input).await?;
            // @step Then only caller activity changes and worker focus is unchanged
            assert_eq!(
                fixture.focus(&fixture.first)?.as_deref(),
                (attribution != "NONE").then_some(attribution)
            );
            assert_eq!(
                fixture.manager.get_work_unit_focus(&fixture.second)?,
                target_before
            );
            let target = fixture.manager.get_session(&fixture.second.value)?;
            if is_message {
                let incoming = target.incoming_message_rx.lock().await.try_recv()?;
                assert_eq!(incoming.message, "Work on AUTH-003");
                target
                    .incoming_message_pending
                    .fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
                target.set_status(codelet_rpc_types::SessionStatus::Idle);
            }
        }
    }
    // @step And invalid missing null empty and foreign card attribution never reaches the management handler
    let admitted = calls.lock().map_err(|_| "calls lock")?.len();
    let before = fixture.manager.get_work_unit_focus(&fixture.first)?;
    for attribution in [
        None,
        Some(Value::Null),
        Some(json!("")),
        Some(json!("FOREIGN-999")),
    ] {
        let mut input =
            json!({"action":"set_role","session_id":fixture.second.value,"role":"forbidden"});
        if let Some(attribution) = attribution {
            input["work_unit_id"] = attribution;
        }
        let error = actor
            .call_tool("AgentManager", &input.to_string())
            .await
            .err()
            .ok_or("invalid call executed")?;
        assert!(error.to_string().contains("work_unit_id"), "{error}");
    }
    assert_eq!(calls.lock().map_err(|_| "calls lock")?.len(), admitted);
    assert_eq!(fixture.manager.get_work_unit_focus(&fixture.first)?, before);
    assert_eq!(
        fixture.manager.get_work_unit_focus(&fixture.second)?,
        target_before
    );
    assert_eq!(
        fixture
            .manager
            .get_session(&fixture.second.value)?
            .get_role(),
        Some("Work on AUTH-003".into())
    );
    // @step And the business handler receives no root attribution and authorization remains unchanged
    assert_eq!(admitted, 6);
    for action in calls.lock().map_err(|_| "calls lock")?.iter() {
        assert!(action.get("work_unit_id").is_none());
        assert_eq!(action["session_id"], fixture.second.value);
    }
    assert_eq!(
        parent
            .get_work_unit_context()
            .and_then(|context| context.id)
            .as_deref(),
        Some("AUTH-001")
    );
    assert_eq!(
        parent.worktree_path(),
        Some(fixture.project.path().to_owned())
    );
    Ok(())
}
