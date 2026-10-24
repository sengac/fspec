use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use codelet_test_helpers::setup_temp_directory;
use rig::tool::Tool;
use serde_json::json;
use std::{
    path::Path,
    sync::{Arc, OnceLock},
    time::Duration,
};

pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
pub mod agent_turn;
pub mod callback;
pub mod metadata;
pub mod observer;
static DATA: OnceLock<tempfile::TempDir> = OnceLock::new();

pub struct Fixture {
    pub project: tempfile::TempDir,
    pub manager: Arc<SessionManager>,
    pub first: SessionId,
    pub second: SessionId,
}

impl Fixture {
    pub async fn new() -> Result<Self> {
        if DATA.get().is_none() {
            let _ = DATA.set(setup_temp_directory()?);
        }
        let data = DATA.get().ok_or("test data")?;
        std::fs::create_dir_all(data.path().join("cache"))?;
        std::fs::write(
            data.path().join("cache/models.json"),
            include_str!("../fixtures/rpc422_models.json"),
        )?;
        let _ = codelet_common::set_data_directory(data.path().to_owned());
        std::env::set_var("ANTHROPIC_API_KEY", "offline-test");
        let project = setup_temp_directory()?;
        std::fs::create_dir_all(project.path().join("spec"))?;
        let cards = ["AUTH-001", "AUTH-002", "AUTH-003"]
            .into_iter()
            .map(|id| {
                (
                    id.to_owned(),
                    json!({"id":id,"title":id,"type":"task","status":"implementing"}),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        std::fs::write(
            project.path().join("spec/work-units.json"),
            serde_json::to_vec(
                &json!({"workUnits":cards, "states":{"implementing":["AUTH-001","AUTH-002","AUTH-003"]}}),
            )?,
        )?;
        let manager = SessionManager::new_arc();
        let first = create(&manager, project.path()).await?;
        let second = create(&manager, project.path()).await?;
        manager.set_session_focus(&first, Some("AUTH-001"), "setup")?;
        manager.set_session_focus(&second, Some("AUTH-002"), "setup")?;
        let session = manager.get_session(&first.value)?;
        *session
            .worktree_path
            .write()
            .map_err(|error| error.to_string())? = Some(project.path().to_owned());
        let handler = codelet_agent_loop::fspec_handler::create_handler(session, false);
        codelet_tools::set_fspec_handler_for_session(
            uuid::Uuid::parse_str(&first.value)?,
            Some(handler),
        );
        Ok(Self {
            project,
            manager,
            first,
            second,
        })
    }

    pub async fn status(&self, id: &str, status: &str) -> Result<String> {
        let tool =
            codelet_tools::facade::claude_fspec_tool(uuid::Uuid::parse_str(&self.first.value)?);
        Ok(tool
            .call(codelet_tools::facade::wrapper::FacadeArgs(
                json!({"command":"update-work-unit-status",
            "args":json!({"workUnitId":id,"status":status}).to_string(),
            "project_root":self.project.path().to_string_lossy()}),
            ))
            .await?)
    }

    pub fn shell(&self, id: &str, status: &str) -> Result {
        let result = codelet_fspec_core::dispatch_command(codelet_fspec_core::DispatchRequest {
            command: "update-work-unit-status".into(),
            args_json: json!({"workUnitId":id,"status":status}).to_string(),
            project_root: self.project.path().to_owned(),
        });
        assert!(result.success, "{:?}", result.error);
        Ok(())
    }

    pub fn focus(&self, id: &SessionId) -> Result<Option<String>> {
        if id == &self.first {
            assert_eq!(
                self.manager.get_session(&id.value)?.worktree_path(),
                Some(self.project.path().to_owned())
            );
        }
        Ok(self
            .manager
            .get_work_unit_focus(id)?
            .context
            .map(|context| context.id))
    }

    pub async fn wait_focus(&self, id: &SessionId, expected: Option<&str>) -> Result {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if self.focus(id)?.as_deref() == expected {
                    return Ok::<(), Box<dyn std::error::Error>>(());
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await??;
        Ok(())
    }

    pub fn observe(&self) -> Result {
        self.manager.reconcile_work_units(self.project.path())?;
        Ok(())
    }
}

pub async fn create(manager: &SessionManager, path: &Path) -> Result<SessionId> {
    let id = uuid::Uuid::new_v4().to_string();
    manager
        .create_session_with_id(
            &id,
            "anthropic/claude-sonnet-4",
            &path.to_string_lossy(),
            "test",
        )
        .await?;
    Ok(SessionId::from(id))
}
