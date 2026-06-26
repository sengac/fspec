//! App::dispatch routing for RPC-022 Action variants (model / thinking /
//! role) plus the RPC-027 `Action::SetThinkingLevelDefault` handler that
//! persists the per-user default via `backend.set_thinking_level_default`.
//! Factored out of `app/dispatch.rs` to keep the orchestrator under 300 LoC.

use codelet_rpc_types::{SessionId, ThinkingLevel};
use codelet_sessions::default_thinking_level_persistence::load_default_thinking_level_opt;

use crate::components::{
    thinking_level_dialog::{ThinkingLevelDialog, THINKING_LEVEL_DIALOG_ID},
    Action,
};

use super::state::App;

impl App {
    /// RPC-022: push a fresh ThinkingLevelDialog seeded with the focused session's level.
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
        // TUI-094: thread the persisted default (TS-parity nullable
        // `defaultLevel`) so the matching row renders ` (default)`.
        let dialog = ThinkingLevelDialog::new(session_id, current)
            .with_default_level(load_default_thinking_level_opt())
            .with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
    }

    /// RPC-022: persist the model selection through the backend and
    /// re-fetch ModelInfo so the SessionHeader chrome repaints.
    /// PROV-117/PROV-118: `session_id` is optional. With a present session the
    /// live-session write (`set_session_model`) fires and the `(current)`
    /// marker is updated. With NO session the choice is persisted as the
    /// in-process DEFAULT model via `set_default_model` (TS parity: the
    /// session guard gates ONLY the live-session write; the default write is
    /// unconditional), breaking the no-default-model deadlock. MODEL-006: once
    /// the default is committed (the spawned task's `Ok` branch) we re-attempt
    /// `create_session` and route the result through
    /// `route_bootstrap_create_session` so a real id seeds the active session
    /// and an empty id surfaces the explicit decline dialog — never a silent
    /// no-op.
    pub(crate) fn handle_model_selected(
        &mut self,
        session_id: Option<SessionId>,
        provider_id: String,
        model_id: String,
    ) {
        tracing::info!(
            target: "model_select",
            session_id = ?session_id,
            provider_id = %provider_id,
            model_id = %model_id,
            "[MODEL-SELECT] handle_model_selected ENTER"
        );
        // PROV-118: with NO active session we cannot bind a per-session
        // model, but we MUST still persist the user's choice as the in-process
        // DEFAULT model — otherwise the next create_session is declined
        // (PROV-101: no anthropic fallback) and the selection is silently
        // dropped (the chicken-and-egg deadlock). TS parity: the session guard
        // gates ONLY the live-session write; the default/store write happens
        // unconditionally. Empty strings are ignored downstream (PROV-101).
        let Some(session_id) = session_id else {
            self.handle_model_selected_no_session(provider_id, model_id);
            return;
        };
        // RPC-337: remember the chosen model id so the ModelSelector's
        // green `(current)` marker lights up on reopen.
        self.agent_view_store
            .set_selected_model_id(session_id.clone(), model_id.clone());
        if tokio::runtime::Handle::try_current().is_err() {
            tracing::warn!(
                target: "model_select",
                "[MODEL-SELECT] handle_model_selected: no tokio runtime -> skipping backend write"
            );
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let sid_for_refresh = session_id.clone();
        tracing::info!(
            target: "model_select",
            session_id = ?session_id,
            provider_id = %provider_id,
            model_id = %model_id,
            "[MODEL-SELECT] handle_model_selected: spawning backend.set_session_model write"
        );
        let handle = tokio::spawn(async move {
            match backend
                .set_session_model(session_id, provider_id, model_id)
                .await
            {
                Ok(()) => tracing::info!(
                    target: "model_select",
                    "[MODEL-SELECT] backend.set_session_model OK"
                ),
                Err(e) => tracing::error!(
                    target: "model_select",
                    error = %e,
                    "[MODEL-SELECT] backend.set_session_model FAILED"
                ),
            }
            // Best-effort chrome refresh — the SessionHeader badges
            // come from RPC-018's get_model_info path.
            if let Ok(info) = backend.get_model_info(sid_for_refresh.clone()).await {
                let _ = action_tx.send(Action::ModelInfoLoaded(sid_for_refresh, info));
            }
        });
        self.pending_tasks.push(handle);
    }

    /// MODEL-006: the `session_id == None` branch of `handle_model_selected`.
    /// Persists the choice as the in-process DEFAULT model
    /// (`set_default_model`) — required because the next `create_session` is
    /// otherwise declined (PROV-101: no silent anthropic fallback). Then, once
    /// the default is committed (the spawned task's `Ok` branch), re-attempts
    /// `create_session` and routes via `route_bootstrap_create_session`: a real
    /// id seeds the active session (`SessionCreated`); an empty id surfaces the
    /// decline dialog (`SessionCreationDeclined`) and is NEVER seeded. No
    /// tokio runtime (`try_current().is_err()`) → the write/retry is skipped.
    fn handle_model_selected_no_session(&mut self, provider_id: String, model_id: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            tracing::warn!(target: "model_select", "[MODEL-SELECT] no runtime -> skip default+retry");
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let active_session_tx = self.active_session_tx.clone();
        let model_string = format!("{provider_id}/{model_id}");
        let handle = tokio::spawn(async move {
            if let Err(e) = backend.set_default_model(model_string.clone()).await {
                tracing::error!(target: "model_select", error = %e, "[MODEL-SELECT] set_default_model FAILED");
                return;
            }
            tracing::info!(target: "model_select", model = %model_string, "[MODEL-SELECT] set_default_model OK");
            // MODEL-006: the default was meant to UNBLOCK session creation;
            // retry it now that the default is committed and route the result.
            match backend.create_session(None).await {
                Ok(session) => crate::app::session_creation::route_bootstrap_create_session(
                    session,
                    &active_session_tx,
                    &action_tx,
                ),
                Err(e) => {
                    tracing::error!(target: "model_select", error = %e, "[MODEL-SELECT] retried create_session FAILED")
                }
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
            let _ = backend.set_thinking_level(session_id, level).await;
            if let Ok(fresh) = backend.get_thinking_level(sid_for_refresh.clone()).await {
                let _ = action_tx.send(Action::ThinkingLevelLoaded(sid_for_refresh, fresh));
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-027 rule [8]: persist the PER-USER DEFAULT thinking level through the
    /// backend. Unlike `handle_thinking_level_selected` this does NOT close the
    /// dialog. TUI-093: after persisting, re-fetch + emit `ThinkingLevelLoaded`
    /// so the active session's `[T:level]` badge repaints (TS `setDefault`).
    pub(crate) fn handle_set_thinking_level_default(
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
            let _ = backend.set_thinking_level_default(session_id, level).await;
            // TUI-093: set_thinking_level_default applied the default in-memory,
            // so a follow-up get reflects it (mirrors handle_thinking_level_selected).
            if let Ok(fresh) = backend.get_thinking_level(sid_for_refresh.clone()).await {
                let _ = action_tx.send(Action::ThinkingLevelLoaded(sid_for_refresh, fresh));
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-022: update AgentViewStore.role_by_session AND fire a
    /// backend.set_session_role write task. Mirrors the
    /// `handle_input_submitted_persistence` fire-and-forget pattern.
    pub(crate) fn handle_set_session_role(&mut self, session_id: SessionId, role: Option<String>) {
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

    /// Route the RPC-022 (model / thinking / role) and RPC-050 (work-unit
    /// attach / detach) Action variants through their helpers. Returns `true`
    /// if the action was handled.
    pub(crate) fn try_dispatch_model_thinking_dialogs(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenThinkingDialog => self.handle_open_thinking_dialog(),
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
            // RPC-050 work-unit binding — helpers in dispatch_work_unit_binding.rs.
            Action::AttachWorkUnitToSession(id) => {
                self.handle_attach_work_unit_to_session(id.clone());
            }
            Action::WorkUnitAttached(s, ctx) => {
                self.handle_work_unit_attached(s.clone(), ctx.clone());
            }
            Action::WorkUnitDetached(s) => self.handle_work_unit_detached(s.clone()),
            // RPC-051 Esc cascade — helper in dispatch_esc_cascade.rs.
            Action::AgentEscPressed => self.handle_agent_esc_pressed(),
            // RPC-098 ESC exit-confirmation dispatcher.
            Action::AgentExitChoice { choice } => self.handle_agent_exit_choice(*choice),
            // RPC-052 pending-input debounce + hydration — helpers in dispatch_pending_input.rs.
            Action::PendingInputChanged(text) => {
                self.handle_pending_input_changed(text.clone());
            }
            Action::SeedPendingInput { session_id, text } => {
                self.handle_seed_pending_input(session_id.clone(), text.clone());
            }
            _ => return false,
        }
        true
    }
}
