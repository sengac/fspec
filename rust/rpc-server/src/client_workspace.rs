//! RPC-445 / RPC-454: the workspace-keyed pushes a WebSocket client
//! receives (work units and git state of workspaces opened besides the
//! daemon's start workspace).

use codelet_rpc_types::{WorkspaceGitState, WorkspaceWorkUnits};
use tokio::sync::broadcast;

use crate::client::FspecWsClient;

/// Template receivers for the workspace-keyed pushes; each subscriber
/// resubscribes. The senders live in the pump task, so a dropped
/// connection closes every receiver.
pub(crate) struct WorkspacePushes {
    work_units: broadcast::Receiver<WorkspaceWorkUnits>,
    git_state: broadcast::Receiver<WorkspaceGitState>,
}

/// The pump-side senders paired with [`WorkspacePushes`].
pub(crate) struct WorkspacePushSenders {
    pub(crate) work_units: broadcast::Sender<WorkspaceWorkUnits>,
    pub(crate) git_state: broadcast::Sender<WorkspaceGitState>,
}

pub(crate) fn workspace_push_channels(capacity: usize) -> (WorkspacePushSenders, WorkspacePushes) {
    let (work_units_tx, work_units) = broadcast::channel(capacity);
    let (git_state_tx, git_state) = broadcast::channel(capacity);
    (
        WorkspacePushSenders {
            work_units: work_units_tx,
            git_state: git_state_tx,
        },
        WorkspacePushes {
            work_units,
            git_state,
        },
    )
}

impl FspecWsClient {
    /// RPC-445: subscribe to work-unit snapshots of workspaces opened
    /// besides the daemon's start workspace.
    pub fn workspace_work_units_rx(&self) -> broadcast::Receiver<WorkspaceWorkUnits> {
        self.workspace_pushes.work_units.resubscribe()
    }

    /// RPC-454: subscribe to git-state snapshots of workspaces opened
    /// besides the daemon's start workspace.
    pub fn workspace_git_state_rx(&self) -> broadcast::Receiver<WorkspaceGitState> {
        self.workspace_pushes.git_state.resubscribe()
    }
}
