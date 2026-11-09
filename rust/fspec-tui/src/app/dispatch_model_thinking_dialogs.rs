//! App::dispatch routing for RPC-022 Action variants (model / thinking /
//! role) plus the RPC-027 `Action::SetThinkingLevelDefault` handler that
//! persists the per-user default via `backend.set_thinking_level_default`.
//! Factored out of `app/dispatch.rs` to keep the orchestrator under 300 LoC.

use codelet_rpc_types::{SessionId, ThinkingLevel};

use crate::components::Action;

use super::dispatch_thinking_levels::set_level;
use super::state::App;

impl App {
    /// RPC-022: persist the model selection through the backend and
    /// re-fetch ModelInfo so the SessionHeader chrome repaints.
    /// PROV-117/PROV-118: `session_id` is optional. With a present session the
    /// live-session write (`set_session_model`) fires and the `(current)`
    /// marker is updated. With NO session the choice is persisted as the
    /// in-process DEFAULT model via `set_default_model` (TS parity: the
    /// session guard gates ONLY the live-session write; the default write is
    /// unconditional), breaking the no-default-model deadlock. MODEL-006: once
    /// the default is committed (the spawned task's `Ok` branch) we re-attempt
    /// `create_session` through `run_session_creation`: a created session
    /// seeds the active session and a creation error opens the session
    /// failure dialog (BUG-410) — never a silent no-op.
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
        self.spawn_model_sync(session_id, Some((provider_id, model_id)));
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
        let handle = tokio::spawn(set_level(backend, action_tx, session_id, level));
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
            // MODEL-011: a refused default (the daemon checks it too) is shown.
            let saved = backend.set_thinking_level_default(session_id.clone(), level);
            if let Err(error) = saved.await {
                let _ = action_tx.send(Action::ThinkingLevelRefused(session_id, error.to_string()));
                return;
            }
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
            Action::ModelSelected(s, p, m) => {
                self.handle_model_selected(s.clone(), p.clone(), m.clone());
            }
            Action::ModelSelectionLoaded(session, revision, result) => {
                self.handle_model_selection_loaded(session, *revision, result);
            }
            Action::ThinkingLevelSelected(s, l) => {
                self.handle_thinking_level_selected(s.clone(), l.clone());
            }
            Action::SetThinkingLevelDefault(s, l) => {
                self.handle_set_thinking_level_default(s.clone(), l.clone());
            }
            Action::ThinkingOptionsLoaded(s, options) => {
                self.handle_thinking_options_loaded(s.clone(), options.clone());
            }
            Action::ThinkingLevelRefused(s, reason) => {
                self.handle_thinking_level_refused(s.clone(), reason.clone());
            }
            Action::SetSessionRole(s, r) => self.handle_set_session_role(s.clone(), r.clone()),
            Action::SessionRoleLoaded(s, r) => {
                self.handle_session_role_loaded(s.clone(), r.clone());
            }
            Action::WorkUnitFocusChanged(session, focus) => {
                self.handle_work_unit_focus_changed(session.clone(), focus.clone());
            }
            Action::SessionObserved(info) => self
                .handle_session_observed(info.clone(), super::session_arrival::SessionFocus::Keep),
            Action::WorkUnitSessionCreated(session, card) => {
                self.handle_work_unit_session_created(session.clone(), card.clone());
            }
            // RPC-051 Esc cascade — helper in dispatch_esc_cascade.rs.
            Action::AgentEscPressed => self.handle_agent_esc_pressed(),
            // RPC-098 ESC exit-confirmation dispatcher.
            Action::AgentExitChoice { choice } => self.handle_agent_exit_choice(*choice),
            // TUI-124 close facts + close outcome — dispatch_agent_close.rs.
            Action::CloseFactsLoaded {
                session,
                kind,
                result,
            } => self.handle_close_facts_loaded(session.clone(), *kind, result.clone()),
            Action::CloseFinished {
                facts,
                closed,
                failure,
            } => self.handle_close_finished(facts, closed, failure.clone()),
            // RPC-052 pending-input debounce + hydration — helpers in dispatch_pending_input.rs.
            Action::PendingInputChanged(text) => {
                self.handle_pending_input_changed(text.clone());
            }
            Action::SeedPendingInput { session_id, text } => {
                self.handle_seed_pending_input(session_id.clone(), text.clone());
            }
            Action::PendingInputHydrated {
                session_id,
                text,
                generation,
            } => {
                self.handle_pending_input_hydrated(session_id.clone(), text.clone(), *generation);
            }
            _ => return false,
        }
        true
    }
}
