//! App::dispatch routing for RPC-022 Action variants:
//! OpenModelDialog, OpenThinkingDialog, ListProvidersLoaded,
//! ModelSelected, ThinkingLevelSelected, SetSessionRole,
//! SessionRoleLoaded — plus the RPC-027
//! Action::SetThinkingLevelDefault handler that persists the
//! per-user default thinking level via `backend.set_thinking_level_default`.
//!
//! Factored out of `app/dispatch.rs` so the orchestrator file stays
//! under the 300-LoC ceiling. The `parse_slash_command` helper lives
//! in the sibling `slash_parser` module — re-exported from `app/mod.rs`
//! for backwards compatibility. Routing is invoked from `App::dispatch`'s
//! match arms via these explicit helper methods.

use codelet_rpc_types::{ProviderInfo, SessionId, ThinkingLevel};

use crate::components::{
    model_selector_dialog::{ModelSelectorDialog, MODEL_SELECTOR_DIALOG_ID},
    thinking_level_dialog::{ThinkingLevelDialog, THINKING_LEVEL_DIALOG_ID},
    Action,
};

use super::state::App;

impl App {
    /// RPC-022: push a fresh ModelSelectorDialog onto the Compositor
    /// AND spawn `backend.list_providers()` whose result is
    /// dispatched back via `Action::ListProvidersLoaded`. Idempotent
    /// — if the dialog is already pushed (id collision) we leave it
    /// alone and only re-spawn the list_providers task.
    pub(crate) fn handle_open_model_dialog(&mut self) {
        let session_id = match self.agent_view_store.current_session().cloned() {
            Some(sid) => sid,
            None => return,
        };
        if !self.compositor.contains(MODEL_SELECTOR_DIALOG_ID) {
            let dialog = ModelSelectorDialog::new(session_id, Vec::new())
                .with_action_tx(self.action_tx.clone());
            self.compositor.push(Box::new(dialog));
        }
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

    /// RPC-022: push a fresh ThinkingLevelDialog onto the Compositor,
    /// seeded with the cached thinking level for the focused session.
    pub(crate) fn handle_open_thinking_dialog(&mut self) {
        let session_id = match self.agent_view_store.current_session().cloned() {
            Some(sid) => sid,
            None => return,
        };
        if self.compositor.contains(THINKING_LEVEL_DIALOG_ID) {
            return;
        }
        let current = self
            .agent_view_store
            .thinking_level_for(&session_id)
            .copied()
            .unwrap_or(ThinkingLevel::Off);
        let dialog = ThinkingLevelDialog::new(session_id, current)
            .with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
    }

    /// RPC-022: fold a backend-fetched provider list into the open
    /// ModelSelectorDialog. The dialog reads the action from the
    /// Compositor's top-down action fan-out (see `App::dispatch`'s
    /// `compositor.update(action)` call at the end of every dispatch
    /// tick), so we only emit the action — the dialog itself updates
    /// its internal state via `ModelSelectorDialog::update`. We keep
    /// this helper as a hook for future explicit-find routing.
    pub(crate) fn handle_list_providers_loaded(&mut self, _providers: Vec<ProviderInfo>) {
        // Intentionally empty — the action fan-out delivers the
        // ProviderInfo list to the open ModelSelectorDialog directly.
    }

    /// RPC-022: persist the model selection through the backend and
    /// re-fetch ModelInfo so the SessionHeader chrome repaints.
    pub(crate) fn handle_model_selected(
        &mut self,
        session_id: SessionId,
        provider_id: String,
        model_id: String,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let sid_for_refresh = session_id.clone();
        let handle = tokio::spawn(async move {
            let _ = backend
                .set_session_model(session_id, provider_id, model_id)
                .await;
            // Best-effort chrome refresh — the SessionHeader badges
            // come from RPC-018's get_model_info path.
            if let Ok(info) = backend.get_model_info(sid_for_refresh.clone()).await {
                let _ = action_tx.send(Action::ModelInfoLoaded(sid_for_refresh, info));
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-022: persist the thinking level through the backend and
    /// re-fetch ThinkingLevel so the SessionHeader `[T:Level]` badge
    /// repaints.
    pub(crate) fn handle_thinking_level_selected(
        &mut self,
        session_id: SessionId,
        level: ThinkingLevel,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let sid_for_refresh = session_id.clone();
        let handle = tokio::spawn(async move {
            let _ = backend
                .set_thinking_level(session_id, level)
                .await;
            if let Ok(fresh) = backend.get_thinking_level(sid_for_refresh.clone()).await {
                let _ = action_tx.send(Action::ThinkingLevelLoaded(sid_for_refresh, fresh));
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-027 rule [8]: persist the PER-USER DEFAULT thinking level
    /// through the backend. Unlike `handle_thinking_level_selected`,
    /// this does NOT close the dialog (the dialog keeps mount per
    /// scenario "Pressing D in ThinkingLevelDialog … keeps the dialog
    /// open"). Spawns a fire-and-forget write via
    /// `backend.set_thinking_level_default`; the default no-op impl on
    /// transports without a session manager keeps callers safe.
    pub(crate) fn handle_set_thinking_level_default(
        &mut self,
        session_id: SessionId,
        level: ThinkingLevel,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let handle = tokio::spawn(async move {
            let _ = backend.set_thinking_level_default(session_id, level).await;
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-022: update AgentViewStore.role_by_session AND fire a
    /// backend.set_session_role write task. Mirrors the
    /// `handle_input_submitted_persistence` fire-and-forget pattern.
    pub(crate) fn handle_set_session_role(
        &mut self,
        session_id: SessionId,
        role: Option<String>,
    ) {
        self.agent_view_store
            .set_role(session_id.clone(), role.clone());
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let handle = tokio::spawn(async move {
            let _ = backend.set_session_role(session_id, role).await;
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-022: fold a backend-fetched role into
    /// AgentViewStore.role_by_session. No backend write fires — this
    /// is a pure read path used by bootstrap + SessionCreated.
    pub(crate) fn handle_session_role_loaded(
        &mut self,
        session_id: SessionId,
        role: Option<String>,
    ) {
        self.agent_view_store.set_role(session_id, role);
    }

    /// RPC-022: spawn a `backend.get_session_role(sid)` task whose
    /// result is dispatched back via `Action::SessionRoleLoaded`. Used
    /// by `App::refresh_session_chrome` on bootstrap + SessionCreated
    /// so the RoleBanner paints from the very first frame.
    pub(crate) fn spawn_get_session_role(&mut self, session_id: SessionId) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let sid = session_id;
        let handle = tokio::spawn(async move {
            if let Ok(role) = backend.get_session_role(sid.clone()).await {
                let _ = action_tx.send(Action::SessionRoleLoaded(sid, role));
            }
        });
        self.pending_tasks.push(handle);
    }

    /// Route the seven RPC-022 Action variants through their helpers.
    /// Called from the catch-all arm of `App::dispatch`'s match so the
    /// orchestrator file `app/dispatch.rs` stays under the 300-LoC
    /// ceiling. Returns `true` if the action was handled.
    pub(crate) fn try_dispatch_rpc022(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenModelDialog => self.handle_open_model_dialog(),
            Action::OpenThinkingDialog => self.handle_open_thinking_dialog(),
            Action::ListProvidersLoaded(p) => self.handle_list_providers_loaded(p.clone()),
            Action::ModelSelected(s, p, m) => {
                self.handle_model_selected(s.clone(), p.clone(), m.clone());
            }
            Action::ThinkingLevelSelected(s, l) => {
                self.handle_thinking_level_selected(s.clone(), *l);
            }
            Action::SetThinkingLevelDefault(s, l) => {
                self.handle_set_thinking_level_default(s.clone(), *l);
            }
            Action::SetSessionRole(s, r) => self.handle_set_session_role(s.clone(), r.clone()),
            Action::SessionRoleLoaded(s, r) => {
                self.handle_session_role_loaded(s.clone(), r.clone());
            }
            _ => return false,
        }
        true
    }
}

