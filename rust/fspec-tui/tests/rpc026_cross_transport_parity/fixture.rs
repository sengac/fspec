use codelet_test_helpers::OrFail;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use codelet_core::persistence::SessionManifest;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_types::SessionInfo;
use codelet_sessions::SessionManager;

pub const SESSION_IDS: [&str; 3] = [
    "11111111-1111-1111-1111-111111111111",
    "22222222-2222-2222-2222-222222222222",
    "33333333-3333-3333-3333-333333333333",
];

pub struct DeletionFixture {
    pub service: Arc<SharedFspecService>,
    pub sessions_dir: PathBuf,
    pub expected: Vec<SessionInfo>,
}

impl DeletionFixture {
    pub fn new(data_dir: &Path, workspace: &Path) -> Result<Self, codelet_test_helpers::TestError> {
        let sessions_dir = data_dir.join("sessions");
        std::fs::create_dir_all(&sessions_dir).or_fail("create isolated persistence directory")?;
        codelet_test_helpers::isolated_user_directory()?;
        codelet_common::set_data_directory(data_dir.to_path_buf())
            .or_fail("select isolated persistence directory")?;
        codelet_core::persistence::reset_session_store_for_tests();
        let timestamp = chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .or_fail("fixed fixture timestamp")?
            .with_timezone(&chrono::Utc);
        let mut expected = Vec::new();
        for (index, identifier) in SESSION_IDS.iter().enumerate() {
            let mut manifest = SessionManifest::with_provider(
                &format!("s-{}", index + 1),
                workspace.to_path_buf(),
                "anthropic/claude-sonnet-4",
            );
            manifest.id = uuid::Uuid::parse_str(identifier).or_fail("fixed fixture UUID")?;
            manifest.created_at = timestamp;
            manifest.updated_at = timestamp;
            std::fs::write(
                sessions_dir.join(format!("{}.json", manifest.id)),
                serde_json::to_vec(&manifest).or_fail("serialize complete seeded manifest")?,
            )
            .or_fail("write complete seeded manifest")?;
            expected.push(SessionInfo {
                id: identifier.to_string(),
                name: manifest.name,
                status: "idle".to_string(),
                project: workspace.to_string_lossy().to_string(),
                message_count: 0,
                provider_id: Some("anthropic".to_string()),
                model_id: Some("claude-sonnet-4".to_string()),
                is_isolated: false,
                worktree_path: None,
                role: None,
                updated_at_ms: Some(timestamp.timestamp_millis()),
                work_unit_focus: Default::default(),
            });
        }
        let watcher = Arc::new(WorkUnitsWatcher::new(workspace).or_fail("independent watcher")?);
        let manager = SessionManager::new();
        let service = Arc::new(
            SharedFspecService::with_session_manager(watcher, manager)
                .with_cwd(workspace.to_path_buf()),
        );
        Ok(Self {
            service,
            sessions_dir,
            expected,
        })
    }

    pub fn assert_middle_deleted(&self) {
        assert!(!self
            .sessions_dir
            .join(format!("{}.json", SESSION_IDS[1]))
            .exists());
        assert!(self
            .sessions_dir
            .join(format!("{}.json", SESSION_IDS[0]))
            .exists());
        assert!(self
            .sessions_dir
            .join(format!("{}.json", SESSION_IDS[2]))
            .exists());
    }
}

impl Drop for DeletionFixture {
    fn drop(&mut self) {
        codelet_core::persistence::reset_session_store_for_tests();
    }
}
