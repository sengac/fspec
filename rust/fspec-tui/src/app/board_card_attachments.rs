//! BOARD-024 — attachment actions raised from complete card details.
//!
//! Feature: spec/features/board-card-details-attachments.feature
//!
//! The details dialog emits the existing `Action::OpenAttachment(path)` with
//! the exact stored path. While the card dialog is open the App resolves it
//! against the canonical daemon workspace reported by the details envelope
//! (never the client's current directory). BUG-431: the workspace may be on
//! another machine, so the bytes come from the daemon
//! (`FspecBackend::read_card_attachment`, which rejects escaping, missing,
//! directory and non-regular paths), are written into a per-app cache under
//! the workspace-relative path, and a viewer bound to that cache serves them.
//! Every failure is reported back to the dialog as
//! `Action::BoardCardAttachmentFailed` so details stay open.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use codelet_attachment_viewer::ViewerHandle;
use tokio::sync::Mutex as AsyncMutex;

use super::state::App;
use crate::components::Action;

/// Browser launcher seam: receives the viewer URL (TUI-126: the App's one
/// URL launcher type).
pub type AttachmentLauncher = crate::app::url_launcher::UrlLauncher;

/// App-owned state for card-details attachment actions.
pub(crate) struct BoardCardRuntime {
    /// Daemon workspace reported by the latest canonical details envelope.
    pub(crate) workspace: Option<PathBuf>,
    launcher: AttachmentLauncher,
    viewer_enabled: bool,
    /// BUG-431: one viewer per daemon workspace, serving its cache directory.
    viewers: Arc<AsyncMutex<HashMap<PathBuf, ViewerHandle>>>,
    /// BUG-431: where attachments read from the daemon are written.
    cache: Arc<AsyncMutex<AttachmentCache>>,
}

/// BUG-431: the per-app attachment cache — one directory per daemon
/// workspace inside a temp directory created on first use.
#[derive(Default)]
struct AttachmentCache {
    root: Option<tempfile::TempDir>,
    workspaces: HashMap<PathBuf, PathBuf>,
}

impl AttachmentCache {
    /// The cache directory for `workspace`, creating it on first use.
    fn directory(&mut self, workspace: &std::path::Path) -> Result<PathBuf, String> {
        if let Some(directory) = self.workspaces.get(workspace) {
            return Ok(directory.clone());
        }
        if self.root.is_none() {
            self.root = Some(
                tempfile::tempdir()
                    .map_err(|error| format!("attachment cache unavailable: {error}"))?,
            );
        }
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| "attachment cache unavailable".to_string())?;
        let directory = root
            .path()
            .join(format!("workspace-{}", self.workspaces.len()));
        std::fs::create_dir_all(&directory)
            .map_err(|error| format!("attachment cache unavailable: {error}"))?;
        self.workspaces
            .insert(workspace.to_path_buf(), directory.clone());
        Ok(directory)
    }
}

impl Default for BoardCardRuntime {
    fn default() -> Self {
        Self {
            workspace: None,
            launcher: crate::app::url_launcher::system_launcher(),
            viewer_enabled: true,
            viewers: Arc::new(AsyncMutex::new(HashMap::new())),
            cache: Arc::new(AsyncMutex::new(AttachmentCache::default())),
        }
    }
}

impl App {
    fn board_card_attachment_failed(&self, path: &str, message: String) {
        let action = Action::BoardCardAttachmentFailed {
            path: path.to_owned(),
            message,
        };
        if self.action_tx.send(action).is_err() {
            tracing::warn!("board card attachment: action bus closed");
        }
    }

    /// Open `path` (exact stored value) from the card details dialog.
    pub(crate) fn open_board_card_attachment(&mut self, path: &str) {
        let Some(workspace) = self.board_card.workspace.clone() else {
            let message = "attachment workspace unavailable (details not loaded)".to_owned();
            return self.board_card_attachment_failed(path, message);
        };
        self.spawn_workspace_file_open(workspace, path, |path, message| {
            Action::BoardCardAttachmentFailed { path, message }
        });
    }

    /// BUG-431 / TUI-120: read `path` of the daemon workspace `workspace`,
    /// cache it and open it in the viewer; every failure is reported as
    /// `failure(path, message)`.
    pub(crate) fn spawn_workspace_file_open(
        &mut self,
        workspace: PathBuf,
        path: &str,
        failure: fn(String, String) -> Action,
    ) {
        let report = |tx: &tokio::sync::mpsc::UnboundedSender<Action>, message: String| {
            if tx.send(failure(path.to_owned(), message)).is_err() {
                tracing::warn!("workspace file open: action bus closed");
            }
        };
        let Some(relative) = codelet_attachment_viewer::workspace_relative(&workspace, path) else {
            let message = "attachment path rejected: it escapes the card workspace".to_owned();
            return report(&self.action_tx, message);
        };
        if !self.board_card.viewer_enabled || tokio::runtime::Handle::try_current().is_err() {
            return report(&self.action_tx, "attachment viewer unavailable".to_owned());
        }
        let fetch = Fetch {
            backend: Arc::clone(&self.backend),
            viewers: Arc::clone(&self.board_card.viewers),
            cache: Arc::clone(&self.board_card.cache),
            launcher: Arc::clone(&self.board_card.launcher),
            workspace,
            relative,
        };
        let tx = self.action_tx.clone();
        let path = path.to_owned();
        let handle = tokio::spawn(async move {
            if let Err(message) = fetch.open(&path).await {
                if tx.send(failure(path, message)).is_err() {
                    tracing::warn!("workspace file open: action bus closed");
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// BUG-431 test-only seam: open `path` as the card dialog does, for a card
    /// whose details reported `workspace`.
    pub fn open_board_card_attachment_for_test(&mut self, workspace: PathBuf, path: &str) {
        self.board_card.workspace = Some(workspace);
        self.open_board_card_attachment(path);
    }

    /// BOARD-024 test-only seam: replace the browser launcher.
    pub fn set_board_attachment_launcher_for_test(&mut self, launcher: AttachmentLauncher) {
        self.board_card.launcher = launcher;
    }

    /// BOARD-024 test-only seam: simulate an unavailable attachment viewer.
    pub fn set_board_attachment_viewer_enabled_for_test(&mut self, enabled: bool) {
        self.board_card.viewer_enabled = enabled;
    }
}

/// BUG-431: one attachment open — read from the daemon, cache, serve.
struct Fetch {
    backend: Arc<dyn crate::transport::FspecBackend>,
    viewers: Arc<AsyncMutex<HashMap<PathBuf, ViewerHandle>>>,
    cache: Arc<AsyncMutex<AttachmentCache>>,
    launcher: AttachmentLauncher,
    workspace: PathBuf,
    relative: PathBuf,
}

impl Fetch {
    async fn open(self, path: &str) -> Result<(), String> {
        let bytes = self
            .backend
            .read_card_attachment(
                self.workspace.to_string_lossy().into_owned(),
                path.to_owned(),
            )
            .await
            .map_err(|error| error.to_string())?;
        let directory = self.cache.lock().await.directory(&self.workspace)?;
        let target = directory.join(&self.relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("attachment cache unavailable: {error}"))?;
        }
        std::fs::write(&target, bytes)
            .map_err(|error| format!("attachment cache unavailable: {error}"))?;
        let relative = self.relative.to_string_lossy().into_owned();
        launch(self.viewers, self.launcher, directory, &relative).await
    }
}

async fn launch(
    viewers: Arc<AsyncMutex<HashMap<PathBuf, ViewerHandle>>>,
    launcher: AttachmentLauncher,
    workspace: PathBuf,
    path: &str,
) -> Result<(), String> {
    let port = {
        let mut viewers = viewers.lock().await;
        match viewers.get(&workspace) {
            Some(viewer) => viewer.port,
            None => {
                let viewer = codelet_attachment_viewer::start_viewer(workspace.clone())
                    .await
                    .map_err(|error| format!("attachment viewer unavailable: {error}"))?;
                let port = viewer.port;
                viewers.insert(workspace, viewer);
                port
            }
        }
    };
    let url = App::attachment_url(port, path);
    // TUI-130: the error names the viewer URL, so it can be copied from the
    // error dialog and opened by hand.
    let shown = url.clone();
    tokio::task::spawn_blocking(move || launcher(&url))
        .await
        .map_err(|error| format!("browser launch failed: {error} (viewer URL: {shown})"))?
        .map_err(|error| format!("browser launch failed: {error} (viewer URL: {shown})"))
}
