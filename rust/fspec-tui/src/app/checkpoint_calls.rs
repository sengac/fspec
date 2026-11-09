//! RPC-453: checkpoint calls for the board's selected workspace.
//!
//! Every checkpoint flow (list, diff, file diff, restore, delete, header
//! counts) goes through `CheckpointCalls`, which calls the
//! `*_in_workspace` RPC for the selected workspace and the start
//! workspace's call only until the workspace list has loaded.

use std::sync::Arc;

use anyhow::Result;
use codelet_rpc_types::{ChangedFile, CheckpointCounts, CheckpointInfo};

use super::state::App;
use crate::FspecBackend;

/// The backend plus the workspace its checkpoint calls address.
#[derive(Clone)]
pub(crate) struct CheckpointCalls {
    backend: Arc<dyn FspecBackend>,
    workspace: Option<String>,
}

impl App {
    /// RPC-453: checkpoint calls for the board's selected workspace.
    pub(crate) fn checkpoint_calls(&self) -> CheckpointCalls {
        CheckpointCalls {
            backend: self.backend.clone(),
            workspace: self.selected_workspace_path(),
        }
    }
}

impl CheckpointCalls {
    pub(crate) async fn counts(&self) -> Result<CheckpointCounts> {
        match &self.workspace {
            Some(workspace) => {
                self.backend
                    .checkpoint_counts_in_workspace(workspace.clone())
                    .await
            }
            None => self.backend.checkpoint_counts().await,
        }
    }

    pub(crate) async fn list(&self) -> Result<Vec<CheckpointInfo>> {
        match &self.workspace {
            Some(workspace) => {
                self.backend
                    .list_checkpoints_in_workspace(workspace.clone())
                    .await
            }
            None => self.backend.list_checkpoints().await,
        }
    }

    pub(crate) async fn diff_files(
        &self,
        work_unit_id: String,
        name: String,
    ) -> Result<Vec<ChangedFile>> {
        match &self.workspace {
            Some(workspace) => {
                self.backend
                    .checkpoint_diff_files_in_workspace(workspace.clone(), work_unit_id, name)
                    .await
            }
            None => self.backend.checkpoint_diff_files(work_unit_id, name).await,
        }
    }

    pub(crate) async fn file_diff(
        &self,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<Option<String>> {
        match &self.workspace {
            Some(workspace) => {
                self.backend
                    .checkpoint_file_diff_in_workspace(workspace.clone(), work_unit_id, name, path)
                    .await
            }
            None => {
                self.backend
                    .checkpoint_file_diff(work_unit_id, name, path)
                    .await
            }
        }
    }

    pub(crate) async fn restore_file(
        &self,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<()> {
        match &self.workspace {
            Some(workspace) => {
                self.backend
                    .restore_checkpoint_file_in_workspace(
                        workspace.clone(),
                        work_unit_id,
                        name,
                        path,
                    )
                    .await
            }
            None => {
                self.backend
                    .restore_checkpoint_file(work_unit_id, name, path)
                    .await
            }
        }
    }

    pub(crate) async fn restore_all(&self, work_unit_id: String, name: String) -> Result<()> {
        match &self.workspace {
            Some(workspace) => {
                self.backend
                    .restore_checkpoint_all_in_workspace(workspace.clone(), work_unit_id, name)
                    .await
            }
            None => {
                self.backend
                    .restore_checkpoint_all(work_unit_id, name)
                    .await
            }
        }
    }

    pub(crate) async fn delete(&self, work_unit_id: String, name: String) -> Result<()> {
        match &self.workspace {
            Some(workspace) => {
                self.backend
                    .delete_checkpoint_in_workspace(workspace.clone(), work_unit_id, name)
                    .await
            }
            None => self.backend.delete_checkpoint(work_unit_id, name).await,
        }
    }

    pub(crate) async fn delete_all(&self) -> Result<()> {
        match &self.workspace {
            Some(workspace) => {
                self.backend
                    .delete_all_checkpoints_in_workspace(workspace.clone())
                    .await
            }
            None => self.backend.delete_all_checkpoints().await,
        }
    }
}
