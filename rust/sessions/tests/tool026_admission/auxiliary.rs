use super::{
    fixture::Fixture,
    support::{args, server, Result},
};
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_tools::{
    execution_context::{scope_invocation, WorkUnitAttribution},
    tool_admission::{authorization_identity, register_auxiliary, register_auxiliary_for_owner},
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub async fn verify(fixture: &Fixture) -> Result {
    let parent = fixture
        .manager
        .admit_tool_invocation(&fixture.first, WorkUnitAttribution::Card("AUTH-003".into()))?;
    let id = Uuid::new_v4();
    let registration = scope_invocation(parent.clone(), async {
        register_auxiliary(id, fixture.project.path().to_path_buf())
    })
    .await?;
    assert_eq!(authorization_identity(id).to_string(), fixture.first.value);
    let before = fixture.manager.get_work_unit_focus(&fixture.first)?;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let actor = server(&id.to_string(), calls.clone(), None).await?;
    assert!(actor.call_tool("probe", "{}").await.is_err());
    actor.call_tool("probe", &args("AUTH-002")).await?;
    actor.call_tool("probe", &args("NONE")).await?;
    {
        let recorded = calls.lock().map_err(|_| "calls")?;
        let selected = recorded[0].1.as_ref().ok_or("auxiliary receipt")?;
        let cleared = recorded[1].1.as_ref().ok_or("clear receipt")?;
        assert_eq!(selected.session_id, id);
        assert_eq!(
            selected.attribution,
            WorkUnitAttribution::Card("AUTH-002".into())
        );
        assert_eq!(cleared.attribution, WorkUnitAttribution::None);
        assert!(cleared.admission > selected.admission);
    }
    assert_eq!(fixture.manager.get_work_unit_focus(&fixture.first)?, before);
    let other = codelet_test_helpers::setup_temp_directory()?;
    let foreign = scope_invocation(parent, async {
        register_auxiliary(Uuid::new_v4(), other.path().to_path_buf())
    })
    .await;
    assert!(
        foreign.is_err(),
        "auxiliary context accepted a project different from its trusted owner"
    );
    drop(registration);
    assert!(actor.call_tool("probe", &args("NONE")).await.is_err());
    let automatic = Uuid::new_v4();
    let _automatic_registration = register_auxiliary_for_owner(
        automatic,
        fixture.project.path().to_path_buf(),
        Uuid::parse_str(&fixture.first.value)?,
    )?;
    assert_eq!(
        authorization_identity(automatic).to_string(),
        fixture.first.value
    );
    let automatic_actor = server(&automatic.to_string(), calls.clone(), None).await?;
    automatic_actor.call_tool("probe", &args("NONE")).await?;
    assert_eq!(fixture.manager.get_work_unit_focus(&fixture.first)?, before);
    let session = fixture.manager.get_session(&fixture.first.value)?;
    session.set_work_unit_context(
        Some("AUTH-003".into()),
        Some("bound".into()),
        Some("specifying".into()),
    );
    codelet_sessions::session_tool_callbacks::register_manager(fixture.manager.clone());
    codelet_tools::facade::set_get_effective_cwd_callback(
        codelet_sessions::session_tool_callbacks::isolation_context,
    );
    assert_eq!(
        codelet_sessions::session_tool_callbacks::work_unit_stage(automatic.to_string()).as_deref(),
        Some("specifying")
    );
    let isolated = codelet_test_helpers::setup_temp_directory()?;
    *session.worktree_path.write().map_err(|_| "worktree")? = Some(isolated.path().to_path_buf());
    let protected = fixture.project.path().join("protected.txt");
    std::fs::write(&protected, "do not expose the main worktree")?;
    let reader = rig::tool::server::ToolServer::new()
        .tool(codelet_tools::ReadTool::new(automatic))
        .run();
    codelet_tools::tool_attribution::configure_tool_server(&reader, automatic.to_string()).await?;
    assert!(reader
        .call_tool(
            "Read",
            &serde_json::json!({"work_unit_id":"NONE","file_path":protected}).to_string()
        )
        .await
        .is_err());
    *session.worktree_path.write().map_err(|_| "worktree")? =
        Some(fixture.project.path().to_path_buf());
    let third = super::fixture::create(&fixture.manager, fixture.project.path()).await?;
    let child = Uuid::new_v4();
    let _child_registration = register_auxiliary_for_owner(
        child,
        fixture.project.path().to_path_buf(),
        Uuid::parse_str(&third.value)?,
    )?;
    let child_actor = server(&child.to_string(), calls.clone(), None).await?;
    fixture.manager.destroy_session(&third.value)?;
    let count = calls.lock().map_err(|_| "calls")?.len();
    assert!(child_actor.call_tool("probe", &args("NONE")).await.is_err());
    assert_eq!(calls.lock().map_err(|_| "calls")?.len(), count);
    Ok(())
}
