//! App::dispatch routing for RPC-337 — the full-screen ModelSelector
//! mode-view actions (`OpenModelSelectorView`, `CloseModelSelectorView`,
//! `RefreshModelSelector`) plus folding `ListProvidersLoaded` into the
//! view.
//!
//! Replaces the retired RPC-022 `OpenModelDialog` Compositor-modal path.
//! Factored out of `app/dispatch.rs` to keep the orchestrator under the
//! 300-LoC ceiling.

use codelet_rpc_types::SessionId;

use crate::components::Action;

use super::state::App;

impl App {
    /// RPC-337: open the full-screen ModelSelector mode-view. BUG-576: the
    /// view starts from a clean list, seeded with the session and its current
    /// `(section, model)`, and a numbered `list_providers` is spawned.
    pub(crate) fn handle_open_model_selector_view(&mut self) {
        let session = self.agent_view_store.current_session().cloned();
        let current = session
            .as_ref()
            .and_then(|sid| self.agent_view_store.selected_model_for(sid))
            .cloned();
        tracing::debug!(target: "model_select", ?session, ?current, "BUG-576: /model opens");
        let view = &mut self.navigator.model_selector;
        view.reset_for_open();
        view.set_session(session);
        view.set_current_model(current.as_ref().map(|(p, m)| (p.as_str(), m.as_str())));
        self.spawn_list_providers_for_selector(false);
    }

    /// RPC-337: re-list for the open ModelSelector mode-view (the `r`
    /// refresh keybind; the view has already flipped `is_refreshing`).
    ///
    /// BUG-207: re-listing alone is a no-op — `list_providers` sources its
    /// models from the on-disk models.dev snapshot, so the refresh FORCES a
    /// catalog re-fetch (`refresh_models_cache("")`, every provider) first.
    pub(crate) fn handle_refresh_model_selector(&mut self) {
        self.spawn_list_providers_for_selector(true);
    }

    /// BUG-576: number the list when it is requested; the view applies only
    /// the newest number.
    fn spawn_list_providers_for_selector(&mut self, force_refresh: bool) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let number = self.navigator.model_selector.request_list();
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            // A failed force-refresh is not fatal: the list still shows
            // whatever snapshot is on disk.
            if force_refresh {
                if let Err(e) = backend.refresh_models_cache(String::new()).await {
                    tracing::warn!(
                        error = %e,
                        "model selector refresh: models.dev force-refresh failed; re-listing the existing snapshot"
                    );
                }
            }
            send_provider_list(&backend, &action_tx, number).await;
        }));
    }

    /// RPC-344: spawn one of the three custom-model write calls, then refresh
    /// the provider list so the new/edited/deleted entry shows immediately.
    /// BUG-325: a failed write is logged and reported back as
    /// `Action::CustomModelWriteFailed` (sent before the refresh).
    fn spawn_custom_model_write(&mut self, action: Action) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let session = self.navigator.model_selector.session().cloned();
        let number = self.navigator.model_selector.request_list();
        let handle = tokio::spawn(async move {
            let (operation, result) = match action {
                Action::AddCustomModel {
                    provider_id,
                    profile_name,
                    definition,
                } => (
                    "add",
                    backend
                        .add_custom_model(provider_id, profile_name, definition)
                        .await,
                ),
                Action::EditCustomModel {
                    provider_id,
                    profile_name,
                    original_model_id,
                    definition,
                } => (
                    "edit",
                    backend
                        .update_custom_model(
                            provider_id,
                            profile_name,
                            original_model_id,
                            definition,
                        )
                        .await,
                ),
                Action::DeleteCustomModel {
                    provider_id,
                    profile_name,
                    model_id,
                } => {
                    // BUG-613: the daemon clears a deleted default model; say so.
                    let before = backend.get_default_model().await;
                    let result = backend
                        .delete_custom_model(provider_id, profile_name, model_id)
                        .await;
                    if result.is_ok() {
                        report_default_cleared(&backend, &action_tx, session.as_ref(), before)
                            .await;
                    }
                    ("delete", result)
                }
                _ => return,
            };
            if let Err(error) = result {
                tracing::warn!(operation, %error, "custom model write failed");
                let _ = action_tx.send(Action::CustomModelWriteFailed {
                    session,
                    message: format!("{operation} custom model failed: {error}"),
                });
            }
            send_provider_list(&backend, &action_tx, number).await;
        });
        self.pending_tasks.push(handle);
    }

    /// BUG-325: show a failed custom-model write in the selector and in the
    /// session's scrollback.
    fn handle_custom_model_write_failed(&mut self, session: Option<&SessionId>, message: &str) {
        self.navigator
            .model_selector
            .set_write_error(Some(message.to_string()));
        if let Some(session) = session {
            self.handle_emit_session_notice(session, format!("[error] {message}"));
        }
    }

    /// Route the RPC-337 model-selector actions. Called from the
    /// catch-all arm of `App::dispatch`. Returns `true` if handled.
    pub(crate) fn try_dispatch_model_selector(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenModelSelectorView => self.handle_open_model_selector_view(),
            Action::RefreshModelSelector => self.handle_refresh_model_selector(),
            Action::ListProvidersLoaded(number, providers) => {
                self.navigator
                    .model_selector
                    .apply_list(*number, providers.clone());
            }
            Action::ListProvidersFailed(number, error) => {
                self.navigator
                    .model_selector
                    .fail_list(*number, error.clone());
            }
            // RPC-344: custom-model write actions spawn the backend call +
            // a provider-list refresh.
            Action::AddCustomModel { .. }
            | Action::EditCustomModel { .. }
            | Action::DeleteCustomModel { .. } => {
                self.spawn_custom_model_write(action.clone());
            }
            Action::CustomModelWriteFailed { session, message } => {
                self.handle_custom_model_write_failed(session.as_ref(), message);
            }
            // CloseModelSelectorView is a pure ViewMode flip handled by
            // Navigator::apply_action — no App-side state to mutate.
            Action::CloseModelSelectorView => {}
            _ => return false,
        }
        true
    }
}

/// BUG-576: list the providers for the selector request `number`. RPC-451:
/// a failed list opens the error dialog, and the selector says it failed.
async fn send_provider_list(
    backend: &std::sync::Arc<dyn crate::FspecBackend>,
    action_tx: &tokio::sync::mpsc::UnboundedSender<Action>,
    number: u64,
) {
    match backend.list_providers().await {
        Ok(providers) => {
            let _ = action_tx.send(Action::ListProvidersLoaded(number, providers));
        }
        Err(error) => {
            let error = format!("{error:#}");
            let _ = action_tx.send(Action::SessionOperationFailed(format!(
                "The provider list could not be read: {error}"
            )));
            let _ = action_tx.send(Action::ListProvidersFailed(number, error));
        }
    }
}

/// BUG-613: the session notice for a delete that cleared fspec's default
/// model (`before` was a default, `after` is none).
pub(crate) fn default_cleared_notice(before: Option<&str>, after: Option<&str>) -> Option<String> {
    match (before, after) {
        (Some(deleted), None) => Some(format!(
            "[info] {deleted} was fspec's default model and is deleted, so fspec has no default model now; choose one with /model"
        )),
        _ => None,
    }
}

/// BUG-613: after a successful delete, show the notice when the default
/// model (read as `before`) was cleared by it. A default that cannot be
/// read is shown as that error.
pub(crate) async fn report_default_cleared(
    backend: &std::sync::Arc<dyn crate::FspecBackend>,
    action_tx: &tokio::sync::mpsc::UnboundedSender<Action>,
    session: Option<&codelet_rpc_types::SessionId>,
    before: anyhow::Result<Option<String>>,
) {
    let Some(session) = session else {
        return;
    };
    let notice = match (before, backend.get_default_model().await) {
        (Ok(before), Ok(after)) => default_cleared_notice(before.as_deref(), after.as_deref()),
        (Err(error), _) | (_, Err(error)) => Some(format!(
            "[error] the default model could not be read: {error:#}"
        )),
    };
    if let Some(notice) = notice {
        let _ = action_tx.send(Action::EmitSessionNotice(session.clone(), notice));
    }
}

#[cfg(test)]
#[path = "dispatch_model_selector/bug613_tests.rs"]
mod bug613_tests;
