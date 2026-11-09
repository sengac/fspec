//! RPC-455: @file search, input history and session lists for the board's
//! selected workspace.
//!
//! Feature: spec/features/file-search-history-and-session-lists-follow-the-selected-workspace.feature
//!
//! `ScopedCalls` calls the `*_in_workspace` RPC for the selected workspace
//! and the start workspace's call only until the workspace list has loaded.
//! BUG-541 / BUG-618: @file search never follows the selected workspace; it
//! asks the composer's session, which searches its own working directory
//! (its worktree when isolated, else its project). With no session open
//! there is nothing to search.

use std::sync::Arc;

use anyhow::Result;
use codelet_rpc_types::{HistoryMatch, SessionId};

use super::state::App;
use crate::FspecBackend;

/// The backend plus the workspace its history calls address, and the
/// composer's session, whose working directory file search addresses.
#[derive(Clone)]
pub(crate) struct ScopedCalls {
    backend: Arc<dyn FspecBackend>,
    workspace: Option<String>,
    session: Option<SessionId>,
}

/// BUG-618 (product owner, 2026-11-08): @file search with no session open.
pub(crate) const NO_SESSION_TO_SEARCH: &str = "no session is open, so there are no files to search";

impl App {
    /// RPC-455: file-search and history calls for the selected workspace.
    pub(crate) fn scoped_calls(&self) -> ScopedCalls {
        ScopedCalls {
            backend: self.backend.clone(),
            workspace: self.selected_workspace_path(),
            session: self.navigator.agent.composer_session().cloned(),
        }
    }

    /// RPC-455: the project the session lists are listed for — the
    /// selected workspace, or the TUI's directory until the workspace list
    /// has loaded. An unresolvable directory is an error.
    pub(crate) fn session_list_project(&self) -> Result<String, String> {
        match self.selected_workspace_path() {
            Some(workspace) => Ok(workspace),
            None => std::env::current_dir()
                .map(|path| path.to_string_lossy().into_owned())
                .map_err(|error| format!("the current directory cannot be resolved: {error}")),
        }
    }
}

impl ScopedCalls {
    /// BUG-618: the composer's session searches its own working directory;
    /// with no session open the search fails with [`NO_SESSION_TO_SEARCH`].
    pub(crate) async fn search_files(&self, prefix: String, limit: u32) -> Result<Vec<String>> {
        match &self.session {
            Some(session) => {
                self.backend
                    .search_files_in_session(session.clone(), prefix, limit)
                    .await
            }
            None => Err(anyhow::anyhow!(NO_SESSION_TO_SEARCH)),
        }
    }

    pub(crate) async fn add_history(&self, session: SessionId, text: String) -> Result<()> {
        match &self.workspace {
            Some(workspace) => {
                self.backend
                    .persistence_add_history_in_workspace(workspace.clone(), session, text)
                    .await
            }
            None => self.backend.persistence_add_history(session, text).await,
        }
    }

    pub(crate) async fn get_history(&self, session: SessionId, limit: u32) -> Result<Vec<String>> {
        match &self.workspace {
            Some(workspace) => {
                self.backend
                    .persistence_get_history_in_workspace(workspace.clone(), session, limit)
                    .await
            }
            None => self.backend.persistence_get_history(session, limit).await,
        }
    }

    pub(crate) async fn search_history(&self, query: String) -> Result<Vec<HistoryMatch>> {
        match &self.workspace {
            Some(workspace) => {
                self.backend
                    .persistence_search_history_in_workspace(workspace.clone(), query)
                    .await
            }
            None => self.backend.persistence_search_history(query).await,
        }
    }
}
