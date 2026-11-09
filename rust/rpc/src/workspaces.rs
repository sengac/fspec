//! RPC-442: per-workspace daemon state.
//!
//! Feature: spec/features/per-workspace-daemon-state.feature
//!
//! The start workspace keeps the service's existing watcher, git-state
//! watcher and cwd. Any other workspace a request names is opened once
//! through the workspace registry and cached by canonical path with its own
//! watchers. RPC-445: each opened workspace forwards its work-unit
//! snapshots, tagged with its path, onto one service-wide broadcast.

use crate::SharedFspecService;
use codelet_common::workspace_registry::{self, Workspace, WorkspaceRegistryError};
use codelet_core::git_state::GitStateWatcher;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc_types::{
    GitState, WorkUnitInfo, WorkspaceEntry, WorkspaceGitState, WorkspaceWorkUnits,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::sync::broadcast;

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error(transparent)]
    Registry(#[from] WorkspaceRegistryError),
    #[error("cannot watch workspace {path}: {reason}")]
    Watcher { path: PathBuf, reason: String },
    #[error("workspace state lock poisoned")]
    Poisoned,
    /// RPC-457: the daemon's start workspace cannot be removed.
    #[error("{0} is the daemon's start workspace and cannot be removed")]
    StartWorkspace(PathBuf),
}

/// RPC-445: broadcast capacity for workspace-keyed work-unit snapshots.
const WORKSPACE_UPDATES_CAPACITY: usize = 256;

/// The opened workspaces plus the keyed stream their watchers feed.
pub(crate) struct OpenedWorkspaces {
    states: Mutex<HashMap<PathBuf, Arc<WorkspaceState>>>,
    updates_tx: broadcast::Sender<WorkspaceWorkUnits>,
    /// RPC-454: git-state snapshots of the opened workspaces.
    git_updates_tx: broadcast::Sender<WorkspaceGitState>,
}

impl Default for OpenedWorkspaces {
    fn default() -> Self {
        Self {
            states: Mutex::new(HashMap::new()),
            updates_tx: broadcast::channel(WORKSPACE_UPDATES_CAPACITY).0,
            git_updates_tx: broadcast::channel(WORKSPACE_UPDATES_CAPACITY).0,
        }
    }
}

/// A workspace opened in addition to the daemon's start workspace.
pub struct WorkspaceState {
    workspace: Workspace,
    watcher: WorkUnitsWatcher,
    git_state: GitStateWatcher,
    forwarder: Option<tokio::task::JoinHandle<()>>,
    git_forwarder: Option<tokio::task::JoinHandle<()>>,
}

impl Drop for WorkspaceState {
    fn drop(&mut self) {
        for forwarder in [self.forwarder.take(), self.git_forwarder.take()]
            .into_iter()
            .flatten()
        {
            forwarder.abort();
        }
    }
}

/// RPC-454: forward `git_state` snapshots onto `updates_tx` tagged with
/// `workspace`. `None` outside a tokio runtime (no live pushes).
fn spawn_git_forwarder(
    git_state: &GitStateWatcher,
    workspace: &Path,
    updates_tx: broadcast::Sender<WorkspaceGitState>,
) -> Option<tokio::task::JoinHandle<()>> {
    let runtime = tokio::runtime::Handle::try_current().ok()?;
    // BUG-657: the watcher's first capture runs off-thread and may land
    // before this subscription; the backfill forwards it exactly once.
    let (backfill, mut rx) = git_state.subscribe_backfilled();
    let workspace = workspace.to_string_lossy().into_owned();
    Some(runtime.spawn(async move {
        if let Some(state) = backfill {
            let _ = updates_tx.send(WorkspaceGitState {
                workspace: workspace.clone(),
                state,
            });
        }
        loop {
            match rx.recv().await {
                Ok(state) => {
                    // No subscribers is not an error: the push is best-effort.
                    let _ = updates_tx.send(WorkspaceGitState {
                        workspace: workspace.clone(),
                        state,
                    });
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    }))
}

/// Forward `watcher` snapshots onto `updates_tx` tagged with `workspace`.
/// `None` outside a tokio runtime (no live pushes, reads still work).
fn spawn_forwarder(
    watcher: &WorkUnitsWatcher,
    workspace: &Path,
    updates_tx: broadcast::Sender<WorkspaceWorkUnits>,
) -> Option<tokio::task::JoinHandle<()>> {
    let runtime = tokio::runtime::Handle::try_current().ok()?;
    let mut rx = watcher.subscribe();
    let workspace = workspace.to_string_lossy().into_owned();
    Some(runtime.spawn(async move {
        loop {
            match rx.recv().await {
                Ok(units) => {
                    let _ = updates_tx.send(WorkspaceWorkUnits {
                        workspace: workspace.clone(),
                        units,
                    });
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    }))
}

/// The workspace a request is served from.
pub enum WorkspaceScope<'a> {
    Start(&'a SharedFspecService),
    Opened(Arc<WorkspaceState>),
}

impl WorkspaceScope<'_> {
    pub fn cwd(&self) -> Option<&Path> {
        match self {
            Self::Start(service) => service.cwd().map(PathBuf::as_path),
            Self::Opened(state) => Some(&state.workspace.path),
        }
    }

    pub fn work_units(&self) -> Vec<WorkUnitInfo> {
        match self {
            Self::Start(service) => service.watcher_snapshot(),
            Self::Opened(state) => state.watcher.snapshot(),
        }
    }

    pub fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        match self {
            Self::Start(service) => service.watcher_rx(),
            Self::Opened(state) => state.watcher.subscribe(),
        }
    }

    pub fn git_state(&self) -> GitState {
        match self {
            Self::Start(service) => service.git_state_snapshot(),
            Self::Opened(state) => state.git_state.snapshot(),
        }
    }
}

impl SharedFspecService {
    /// Resolve the workspace a request names; `None` is the start workspace.
    pub fn workspace(&self, path: Option<&Path>) -> Result<WorkspaceScope<'_>, WorkspaceError> {
        let Some(path) = path else {
            return Ok(WorkspaceScope::Start(self));
        };
        let identity = workspace_registry::identify(path)?;
        let start = self.cwd().and_then(|cwd| std::fs::canonicalize(cwd).ok());
        if start.as_deref() == Some(identity.path.as_path()) {
            return Ok(WorkspaceScope::Start(self));
        }
        let mut opened = self
            .opened_workspaces
            .states
            .lock()
            .map_err(|_| WorkspaceError::Poisoned)?;
        if let Some(state) = opened.get(&identity.path) {
            return Ok(WorkspaceScope::Opened(Arc::clone(state)));
        }
        let workspace = workspace_registry::open(&identity.path)?;
        let watcher =
            WorkUnitsWatcher::new(&workspace.path).map_err(|error| WorkspaceError::Watcher {
                path: workspace.path.clone(),
                reason: error.to_string(),
            })?;
        let forwarder = spawn_forwarder(
            &watcher,
            &workspace.path,
            self.opened_workspaces.updates_tx.clone(),
        );
        let git_state = GitStateWatcher::new(&workspace.path);
        let git_forwarder = spawn_git_forwarder(
            &git_state,
            &workspace.path,
            self.opened_workspaces.git_updates_tx.clone(),
        );
        let state = Arc::new(WorkspaceState {
            git_state,
            watcher,
            workspace,
            forwarder,
            git_forwarder,
        });
        opened.insert(state.workspace.path.clone(), Arc::clone(&state));
        Ok(WorkspaceScope::Opened(state))
    }

    /// RPC-443: the start workspace first, then every other remembered
    /// workspace whose directory still exists.
    pub fn workspace_entries(&self) -> Result<Vec<WorkspaceEntry>, WorkspaceError> {
        let entry = |workspace: &Workspace, start: bool| WorkspaceEntry {
            path: workspace.path.to_string_lossy().into_owned(),
            name: workspace.name.clone(),
            start,
        };
        let start = self
            .cwd()
            .map(|cwd| workspace_registry::identify(cwd))
            .transpose()?;
        let mut entries: Vec<WorkspaceEntry> = start.iter().map(|w| entry(w, true)).collect();
        for workspace in workspace_registry::list()? {
            if start.as_ref().map(|start| &start.path) != Some(&workspace.path) {
                entries.push(entry(&workspace, false));
            }
        }
        Ok(entries)
    }

    /// RPC-457: explicitly add the fspec project at `path`.
    pub fn add_workspace(&self, path: &Path) -> Result<WorkspaceEntry, WorkspaceError> {
        let workspace = workspace_registry::add(path)?;
        let start = self
            .cwd()
            .map(|cwd| workspace_registry::identify(cwd))
            .transpose()?;
        Ok(WorkspaceEntry {
            path: workspace.path.to_string_lossy().into_owned(),
            name: workspace.name,
            start: start.map(|start| start.path) == Some(workspace.path),
        })
    }

    /// RPC-457: forget the workspace at `path` and drop its cached scope
    /// (dropping the state aborts its watchers' forwarders). The start
    /// workspace cannot be removed.
    pub fn forget_workspace(&self, path: &Path) -> Result<Workspace, WorkspaceError> {
        if let Some(cwd) = self.cwd() {
            let start = workspace_registry::identify(cwd)?;
            if std::fs::canonicalize(path).ok().as_deref() == Some(start.path.as_path()) {
                return Err(WorkspaceError::StartWorkspace(start.path));
            }
        }
        let removed = workspace_registry::remove(path)?;
        self.opened_workspaces
            .states
            .lock()
            .map_err(|_| WorkspaceError::Poisoned)?
            .remove(&removed.path);
        Ok(removed)
    }

    /// Workspaces opened besides the start workspace, in path order.
    pub fn opened_workspaces(&self) -> Vec<Workspace> {
        let mut workspaces: Vec<Workspace> = self
            .opened_workspaces
            .states
            .lock()
            .map(|opened| {
                opened
                    .values()
                    .map(|state| state.workspace.clone())
                    .collect()
            })
            .unwrap_or_default();
        workspaces.sort_by(|left, right| left.path.cmp(&right.path));
        workspaces
    }

    /// RPC-445: work-unit snapshots of every opened (non-start)
    /// workspace, tagged with its canonical path.
    pub fn workspace_work_units_rx(&self) -> broadcast::Receiver<WorkspaceWorkUnits> {
        self.opened_workspaces.updates_tx.subscribe()
    }

    /// RPC-454: git-state snapshots of every opened (non-start) workspace,
    /// tagged with its canonical path.
    pub fn workspace_git_state_rx(&self) -> broadcast::Receiver<WorkspaceGitState> {
        self.opened_workspaces.git_updates_tx.subscribe()
    }
}
