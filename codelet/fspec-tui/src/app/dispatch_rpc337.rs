//! App::dispatch routing for RPC-337 — the full-screen ModelSelector
//! mode-view actions (`OpenModelSelectorView`, `CloseModelSelectorView`,
//! `RefreshModelSelector`) plus folding `ListProvidersLoaded` into the
//! view.
//!
//! Replaces the retired RPC-022 `OpenModelDialog` Compositor-modal path.
//! Factored out of `app/dispatch.rs` to keep the orchestrator under the
//! 300-LoC ceiling.

use codelet_rpc_types::ProviderInfo;

use crate::components::Action;

use super::state::App;

impl App {
    /// RPC-337: open the full-screen ModelSelector mode-view. Seeds the
    /// view with the current session + last-selected model id (for the
    /// `(current)` marker), then spawns `backend.list_providers()` whose
    /// result returns via `Action::ListProvidersLoaded`.
    pub(crate) fn handle_open_model_selector_view(&mut self) {
        let session = self.agent_view_store.current_session().cloned();
        let current_model = session
            .as_ref()
            .and_then(|sid| self.agent_view_store.selected_model_id_for(sid))
            .map(str::to_string);
        let view = &mut self.navigator.model_selector;
        view.set_session(session);
        view.set_current_model(current_model);
        self.spawn_list_providers_for_selector();
    }

    /// RPC-337: re-spawn `backend.list_providers()` for the open
    /// ModelSelector mode-view (the `r` refresh keybind). The view has
    /// already flipped its `is_refreshing` flag.
    pub(crate) fn handle_refresh_model_selector(&mut self) {
        self.spawn_list_providers_for_selector();
    }

    /// RPC-337: fold a backend-fetched provider list into the open
    /// ModelSelector mode-view.
    pub(crate) fn handle_model_selector_providers_loaded(&mut self, providers: Vec<ProviderInfo>) {
        self.navigator.model_selector.set_providers(providers);
    }

    fn spawn_list_providers_for_selector(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            if let Ok(providers) = backend.list_providers().await {
                let _ = action_tx.send(Action::ListProvidersLoaded(providers));
            }
        });
        self.pending_tasks.push(handle);
    }

    /// Route the RPC-337 model-selector actions. Called from the
    /// catch-all arm of `App::dispatch`. Returns `true` if handled.
    pub(crate) fn try_dispatch_rpc337(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenModelSelectorView => self.handle_open_model_selector_view(),
            Action::RefreshModelSelector => self.handle_refresh_model_selector(),
            Action::ListProvidersLoaded(providers) => {
                self.handle_model_selector_providers_loaded(providers.clone());
            }
            // CloseModelSelectorView is a pure ViewMode flip handled by
            // Navigator::apply_action — no App-side state to mutate.
            Action::CloseModelSelectorView => {}
            _ => return false,
        }
        true
    }
}
