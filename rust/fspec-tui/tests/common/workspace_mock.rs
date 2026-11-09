//! RPC-445: MockBackend's scripted workspaces — the workspace list, each
//! non-start workspace's work units, keyed pushes and the workspace the
//! last `create_session_in_workspace` named.

use parking_lot::Mutex;
use std::collections::HashMap;

use codelet_rpc_types::{SessionInfo, WorkUnitInfo, WorkspaceEntry, WorkspaceWorkUnits};
use tokio::sync::broadcast;

use super::MockBackend;

pub struct WorkspaceMock {
    pub(super) entries: Mutex<Vec<WorkspaceEntry>>,
    pub(super) units: Mutex<HashMap<String, Vec<WorkUnitInfo>>>,
    pub(super) tx: broadcast::Sender<WorkspaceWorkUnits>,
    pub(super) created_in: Mutex<Option<String>>,
    /// BUG-427: the workspace the last isolated creation named.
    pub(super) isolated_created_in: Mutex<Option<String>>,
    /// BUG-431: the scripted `read_card_attachment` result and the
    /// (workspace, path) of every call.
    pub(super) attachment: Mutex<Option<Result<Vec<u8>, String>>>,
    pub(super) attachment_requests: Mutex<Vec<(String, String)>>,
    /// RPC-452: the workspace of every `changed_files_in_workspace` call
    /// and a scripted failure.
    pub(super) changed_files_requests: Mutex<Vec<String>>,
    pub(super) changed_files_error: Mutex<Option<String>>,
    /// RPC-453: every checkpoint call as "method:workspace".
    pub(super) checkpoint_requests: Mutex<Vec<String>>,
    /// RPC-454: every workspace info / git state call as "kind:workspace".
    pub(super) workspace_git_requests: Mutex<Vec<String>>,
    /// RPC-455: every workspace file-search / history call as "kind:workspace".
    pub(super) scope_requests: Mutex<Vec<String>>,
    /// BUG-541: a scripted `search_files_in_session` failure.
    pub(super) session_search_error: Mutex<Option<String>>,
    /// TUI-122: every add/remove call as "add:path" / "remove:path", and a
    /// scripted add failure.
    pub(super) admin_requests: Mutex<Vec<String>>,
    pub(super) add_error: Mutex<Option<String>>,
    /// TUI-129: a scripted `move_work_unit_up` / `_down` failure.
    pub(super) move_error: Mutex<Option<String>>,
}

impl Default for WorkspaceMock {
    fn default() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            units: Mutex::new(HashMap::new()),
            tx: broadcast::channel(64).0,
            created_in: Mutex::new(None),
            isolated_created_in: Mutex::new(None),
            attachment: Mutex::new(None),
            attachment_requests: Mutex::new(Vec::new()),
            changed_files_requests: Mutex::new(Vec::new()),
            changed_files_error: Mutex::new(None),
            checkpoint_requests: Mutex::new(Vec::new()),
            workspace_git_requests: Mutex::new(Vec::new()),
            scope_requests: Mutex::new(Vec::new()),
            session_search_error: Mutex::new(None),
            admin_requests: Mutex::new(Vec::new()),
            move_error: Mutex::new(None),
            add_error: Mutex::new(None),
        }
    }
}

impl MockBackend {
    /// Script `list_workspaces`.
    pub fn seed_workspaces(&self, entries: Vec<WorkspaceEntry>) {
        *self.workspaces.entries.lock() = entries;
    }

    /// Script `list_workspace_work_units(path)`.
    pub fn seed_workspace_work_units(&self, path: &str, units: Vec<WorkUnitInfo>) {
        self.workspaces.units.lock().insert(path.to_owned(), units);
    }

    /// Push a workspace-keyed work-units snapshot.
    pub fn push_workspace_work_units(&self, path: &str, units: Vec<WorkUnitInfo>) {
        let _ = self.workspaces.tx.send(WorkspaceWorkUnits {
            workspace: path.to_owned(),
            units,
        });
    }

    /// The workspace the last `create_session_in_workspace` call named.
    pub fn session_created_in(&self) -> Option<String> {
        self.workspaces.created_in.lock().clone()
    }

    /// BUG-427: the workspace the last `create_isolated_session_in_workspace`
    /// call named.
    pub fn isolated_session_created_in(&self) -> Option<String> {
        self.workspaces.isolated_created_in.lock().clone()
    }

    /// BUG-431: script what `read_card_attachment` returns.
    pub fn script_card_attachment(&self, result: Result<Vec<u8>, String>) {
        *self.workspaces.attachment.lock() = Some(result);
    }

    /// BUG-431: the (workspace, path) of every `read_card_attachment` call.
    pub fn card_attachment_requests(&self) -> Vec<(String, String)> {
        self.workspaces.attachment_requests.lock().clone()
    }

    /// Script the connect-time session snapshot with full infos.
    pub fn seed_session_snapshot_infos(&self, infos: Vec<SessionInfo>) {
        *self.core.scripted_session_snapshot.lock() = infos;
    }

    /// RPC-452: the workspace named by every `changed_files_in_workspace` call.
    pub fn changed_files_requests(&self) -> Vec<String> {
        self.workspaces.changed_files_requests.lock().clone()
    }

    /// RPC-453: every checkpoint call as "method:workspace".
    pub fn checkpoint_requests(&self) -> Vec<String> {
        self.workspaces.checkpoint_requests.lock().clone()
    }

    pub(super) fn record_checkpoint_request(&self, method: &str, workspace: &str) {
        self.workspaces
            .checkpoint_requests
            .lock()
            .push(format!("{method}:{workspace}"));
    }

    /// RPC-454: every workspace info / git state call as "kind:workspace".
    pub fn workspace_git_requests(&self) -> Vec<String> {
        self.workspaces.workspace_git_requests.lock().clone()
    }

    pub(super) fn record_workspace_git_request(&self, kind: &str, workspace: &str) {
        self.workspaces
            .workspace_git_requests
            .lock()
            .push(format!("{kind}:{workspace}"));
    }

    /// RPC-455: every workspace file-search / history call as "kind:workspace".
    pub fn workspace_scope_requests(&self) -> Vec<String> {
        self.workspaces.scope_requests.lock().clone()
    }

    pub(super) fn record_scope_request(&self, kind: &str, workspace: &str) {
        self.workspaces
            .scope_requests
            .lock()
            .push(format!("{kind}:{workspace}"));
    }

    /// TUI-122: every add/remove call as "add:path" / "remove:path".
    pub fn workspace_admin_requests(&self) -> Vec<String> {
        self.workspaces.admin_requests.lock().clone()
    }

    /// TUI-122: make the next `add_workspace` calls fail with `message`.
    pub fn fail_add_workspace(&self, message: &str) {
        *self.workspaces.add_error.lock() = Some(message.to_string());
    }

    /// TUI-129: make the next card moves fail with `message`.
    pub fn fail_move_work_unit(&self, message: &str) {
        *self.workspaces.move_error.lock() = Some(message.to_string());
    }

    /// TUI-129: the scripted card-move failure, if any.
    pub(super) fn scripted_move_error(&self) -> Result<(), anyhow::Error> {
        match self.workspaces.move_error.lock().clone() {
            Some(message) => Err(anyhow::anyhow!(message)),
            None => Ok(()),
        }
    }

    /// TUI-122: record an add, then register `path` (named after its last
    /// segment) unless an add failure is scripted.
    pub(super) fn mock_add_workspace(&self, path: &str) -> Result<WorkspaceEntry, String> {
        self.workspaces
            .admin_requests
            .lock()
            .push(format!("add:{path}"));
        if let Some(message) = self.workspaces.add_error.lock().clone() {
            return Err(message);
        }
        let name = path
            .rsplit('/')
            .next()
            .ok_or_else(|| format!("workspace path {path} has no last segment"))?
            .to_owned();
        let entry = WorkspaceEntry {
            path: path.to_owned(),
            name,
            start: false,
        };
        self.workspaces.entries.lock().push(entry.clone());
        Ok(entry)
    }

    /// TUI-122: record a removal and drop `path` from the listed workspaces.
    pub(super) fn mock_remove_workspace(&self, path: &str) {
        self.workspaces
            .admin_requests
            .lock()
            .push(format!("remove:{path}"));
        self.workspaces
            .entries
            .lock()
            .retain(|entry| entry.path != path);
    }

    /// RPC-452: make `changed_files_in_workspace` fail with `message`.
    pub fn fail_changed_files(&self, message: &str) {
        *self.workspaces.changed_files_error.lock() = Some(message.to_string());
    }

    /// BUG-541: make `search_files_in_session` fail with `message`.
    pub fn fail_session_file_search(&self, message: &str) {
        *self.workspaces.session_search_error.lock() = Some(message.to_string());
    }
}

/// A session snapshot entry for `id` in `project`.
pub fn session_in(id: &str, project: &str) -> SessionInfo {
    SessionInfo {
        id: id.to_owned(),
        name: String::new(),
        status: "idle".to_string(),
        project: project.to_owned(),
        message_count: 0,
        provider_id: None,
        model_id: None,
        is_isolated: false,
        worktree_path: None,
        role: None,
        updated_at_ms: None,
        work_unit_focus: Default::default(),
    }
}
