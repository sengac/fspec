use super::{
    fixture::Fixture,
    support::{server_with_callback, Calls, Result},
};
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_tools::execution_context::current_invocation;
use rig::tool::{server::ToolServerHandle, ToolError};
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};

pub async fn valid_probe(fixture: &Fixture, calls: Calls) -> Result<ToolServerHandle> {
    let manager = fixture.manager.clone();
    let invoking = fixture.first.clone();
    let callback = Arc::new(move || {
        let focus = manager
            .get_work_unit_focus(&invoking)
            .map_err(|error| ToolError::ToolCallError(error.into()))?;
        assert_eq!(
            focus.context.as_ref().map(|context| context.id.as_str()),
            Some("AUTH-003")
        );
        let receipt = current_invocation()
            .ok_or_else(|| ToolError::ToolCallError("missing pre-execution receipt".into()))?;
        assert_eq!(receipt.session_id.to_string(), invoking.value);
        assert_eq!(receipt.focus_revision, focus.revision);
        Ok(())
    });
    server_with_callback(&fixture.first.value, calls, None, Some(callback)).await
}

pub fn invalid_inputs() -> [Value; 9] {
    [
        json!({}),
        json!({"work_unit_id":null}),
        json!({"work_unit_id":""}),
        json!({"work_unit_id":4}),
        json!({"work_unit_id":"FOREIGN-001"}),
        json!({"work_unit_id":"AUTH-999"}),
        json!({"work_unit_id":{"extra":true}}),
        json!({"work_unit_id":["NONE"]}),
        json!({"work_unit_id":true}),
    ]
}

pub async fn restricted_writer(fixture: &Fixture) -> Result<ToolServerHandle> {
    use codelet_tools::WriteTool;
    use rig::tool::server::ToolServer;

    codelet_sessions::session_tool_callbacks::register_manager(fixture.manager.clone());
    codelet_tools::facade::set_get_work_unit_stage_callback(
        codelet_sessions::session_tool_callbacks::work_unit_stage,
    );
    codelet_tools::facade::set_get_effective_cwd_callback(
        codelet_sessions::session_tool_callbacks::isolation_context,
    );
    let session = fixture.manager.get_session(&fixture.first.value)?;
    session.set_work_unit_context(
        Some("AUTH-001".into()),
        Some("bound".into()),
        Some("specifying".into()),
    );
    let actor = ToolServer::new().tool(WriteTool::new(session.id)).run();
    codelet_tools::tool_attribution::configure_tool_server(&actor, &fixture.first.value).await?;
    Ok(actor)
}

pub async fn verify_path_restriction(
    fixture: &Fixture,
    actor: &ToolServerHandle,
    output: &Path,
) -> Result {
    let session = fixture.manager.get_session(&fixture.first.value)?;
    let outside = codelet_test_helpers::setup_temp_directory()?;
    session.set_work_unit_context(
        Some("AUTH-001".into()),
        Some("bound".into()),
        Some("implementing".into()),
    );
    *session.worktree_path.write().map_err(|_| "worktree")? = Some(outside.path().to_path_buf());
    let input = json!({"work_unit_id":"AUTH-002","file_path":output,"content":"forbidden"});
    assert!(actor.call_tool("Write", &input.to_string()).await.is_err());
    assert!(!output.exists());
    Ok(())
}
