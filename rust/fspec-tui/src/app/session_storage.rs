use codelet_rpc_types::SessionManifestIssue;
use tokio::sync::mpsc::UnboundedSender;

use crate::components::session_storage_dialog::{SessionStorageDialog, SESSION_STORAGE_DIALOG_ID};
use crate::components::Action;
use crate::transport::FspecBackend;

use super::state::App;

#[derive(Default)]
pub(crate) struct SessionStorageState {
    issues: Vec<SessionManifestIssue>,
    errors: Vec<String>,
}

pub(crate) async fn publish_manifest_issues(
    backend: &dyn FspecBackend,
    sender: &UnboundedSender<Action>,
    replay: bool,
) {
    let action = match backend.session_manifest_issues().await {
        Ok(issues) => Action::SessionManifestIssuesLoaded { issues, replay },
        Err(error) => {
            Action::SessionOperationFailed(format!("Session storage discovery failed: {error}"))
        }
    };
    let _ = sender.send(action);
}

impl App {
    pub(crate) fn spawn_session_manifest_issues(&mut self, replay: bool) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let sender = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            publish_manifest_issues(backend.as_ref(), &sender, replay).await;
        }));
    }

    pub(crate) fn try_dispatch_session_storage(&mut self, action: &Action) -> bool {
        match action {
            Action::SessionManifestIssuesLoaded { issues, replay } => {
                if !replay && self.session_storage.issues == *issues {
                    return true;
                }
                self.session_storage.issues.clone_from(issues);
            }
            Action::SessionOperationFailed(error) => {
                if !self.session_storage.errors.contains(error) {
                    self.session_storage.errors.push(error.clone());
                }
            }
            Action::SessionStorageDismissed => {
                self.session_storage.errors.clear();
                return true;
            }
            _ => return false,
        }
        let _ = self.compositor.remove(SESSION_STORAGE_DIALOG_ID);
        if !self.session_storage.issues.is_empty() || !self.session_storage.errors.is_empty() {
            self.compositor.push(Box::new(SessionStorageDialog::new(
                &self.session_storage.issues,
                &self.session_storage.errors,
                self.action_tx.clone(),
            )));
        }
        true
    }
}
