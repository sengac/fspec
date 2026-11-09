macro_rules! compose_handle_trait_1 {
    ($($collected:tt)*) => {
        compose_handle_trait_2! {
            $($collected)*
    fn reconcile_work_units(&self, _project: &std::path::Path) -> Result<(), String> {
        Ok(())
    }
    /// Return public metadata for every session currently tracked, filtered by
    /// `project_path`. RPC-427: added `project_path` parameter so `/resume` only
    /// shows sessions belonging to the current project.
    fn list_sessions(&self, project_path: &str) -> Result<Vec<SessionInfo>, String>;
    /// Create a new session with an optional role. Returns the
    /// freshly-minted [`SessionId`], or the reason creation failed
    /// (BUG-399: there is no infallible variant).
    fn create_session_checked(&self, role: Option<String>) -> Result<SessionId, String>;
    /// RPC-444: create a session whose project is `workspace` (canonicalized).
    fn create_session_in_workspace(
        &self,
        workspace: &std::path::Path,
        role: Option<String>,
    ) -> Result<SessionId, String> {
        let _ = (workspace, role);
        Err("creating sessions in a named workspace is not supported by this handle".into())
    }
    /// Send user input to a live session. Returns once the input is
    /// accepted — the actual streaming response arrives on the chunks
    /// broadcast subscribed via [`SessionManagerHandle::chunks_rx`].
    /// BUG-328: a session the handle does not hold live is refused with
    /// "Session not found: <id> ..."; nothing is resumed implicitly.
    fn send_input(&self, session_id: &SessionId, text: String) -> Result<(), String>;
    /// Interrupt a running session. Returns immediately. A subsequent
    /// `StreamChunk::Interrupted` will arrive on the chunks broadcast.
    fn interrupt(&self, session_id: &SessionId) -> Result<(), String>;
    /// Return the current lifecycle state of a session.
    fn get_session_status(&self, session_id: &SessionId) -> Result<SessionStatus, String>;
    /// Subscribe to the per-process StreamChunk broadcast. Every send_input
    /// pushes its streaming output here as `(SessionId, StreamChunk)` tuples.
    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)>;
    /// Subscribe to the per-process LogRecord broadcast. The host's
    /// tracing::Layer pushes structured events here.
    fn logs_rx(&self) -> broadcast::Receiver<LogRecord>;
    /// Return a cloneable handle to the chunks broadcast sender so the
    /// host's tracing layer / NAPI ThreadsafeFunction co-listener can
    /// publish or co-subscribe directly.
    fn chunks_tx(&self) -> broadcast::Sender<(SessionId, StreamChunk)>;
    /// Return a cloneable handle to the logs broadcast sender so the
    /// host's tracing::Layer can push records onto the same broadcast
    /// that other listeners observe via `logs_rx`.
    fn logs_tx(&self) -> broadcast::Sender<LogRecord>;
    /// RPC-018: return the display + capability metadata for the model
    /// currently bound to `session_id`. Default implementation returns
    /// `ModelInfo::default()` (empty display name, all-false caps,
    /// context_window = 0) so handles that don't yet know how to
    /// resolve provider/model state — including `StubSessionManagerHandle`
    /// — compile without per-test wiring. The concrete rust/napi
    /// `SessionManager` overrides this in RPC-022 once the ModelSelector
    /// modal dialog needs live data.
    fn get_model_info(&self, session_id: &SessionId) -> Result<ModelInfo, String> {
        let _ = session_id;
        Err("get_model_info is not implemented by this session manager".to_string())
    }
    /// RPC-018: return the per-session thinking/reasoning level.
    /// Mirrors `get_model_info` in shape — default returns
    /// `ThinkingLevel::Off`; the rust/napi `SessionManager` overrides
    /// this in RPC-022 (ThinkingLevel modal dialog).
    fn get_thinking_level(&self, session_id: &SessionId) -> Result<ThinkingLevel, String> {
        let _ = session_id;
        Err("get_thinking_level is not implemented by this session manager".to_string())
    }
    /// MODEL-011: the thinking levels the session's current model offers,
    /// or that it has no reasoning control.
    fn get_thinking_options(
        &self,
        session_id: &SessionId,
    ) -> Result<codelet_rpc_types::ThinkingOptions, String> {
        let _ = session_id;
        Err("get_thinking_options is not implemented by this session manager".to_string())
    }
    /// RPC-022: return the available provider/model registry for the
    /// /model modal dialog. Default returns `Vec::new()` so handles
    /// that have not yet wired the model registry — including the
    /// `StubSessionManagerHandle` used by integration tests — compile
    /// without per-test wiring. The concrete rust/napi
    /// `SessionManager` overrides this to read the cached
    /// `ModelRegistry` and map each provider/model into the
    /// transport-portable `ProviderInfo` / `ModelEntry` shape.
    /// RPC-451: a failed provider listing is an error, never an empty list.
    fn list_providers(&self) -> Result<Vec<ProviderInfo>, String> {
        Err("list_providers is not implemented by this session manager".to_string())
    }
    /// RPC-022: set the model bound to a session. Default returns
    /// `Ok(())` (silent no-op) so handles that have not yet wired
    /// model selection — including the stub used by tests — compile
    /// without per-test wiring. The rust/napi `SessionManager`
    /// overrides this to delegate to the existing
    /// `session_set_model`-style flow (model_string parsing +
    /// `ProviderManager::select_model`).
    fn set_model(
        &self,
        session_id: &SessionId,
        provider_id: &str,
        model_id: &str,
    ) -> Result<(), String> {
        let _ = (session_id, provider_id, model_id);
        Ok(())
    }
    /// PROV-118: set the in-process DEFAULT model used by `create_session`
    /// when no per-session model is bound. Mirrors `set_model` in shape but
    /// targets the manager-level default rather than a live session. Default
    /// returns silently so handles that have not wired the default-model path
    /// — including the stub used by tests — compile without per-test wiring.
    /// The rust/sessions `SessionManager` overrides this to delegate to
    /// `SessionManager::set_default_model` (which ignores empty strings,
    /// preserving the PROV-101 no-fallback policy).
    fn set_default_model(&self, model: &str) -> Result<(), String> {
        // BUG-411: a handle without a default-model store refuses the write.
        Err(format!(
            "setting the default model ({model}) is not supported by this session manager handle"
        ))
    }
    /// RPC-440: the default model, if any. Default `None` (handles without a
    /// default-model store, including the stub, report none).
    fn get_default_model(&self) -> Option<String> {
        None
    }
    /// RIG-016: submit input under a delivery mode. Default refuses, so
    /// handles without live sessions (including the stub) compile unchanged.
    fn submit_input(
        &self,
        session_id: &SessionId,
        text: String,
        mode: codelet_rpc_types::InputMode,
    ) -> Result<(), String> {
        let _ = (text, mode);
        Err(format!("session {} cannot take input on this handle", session_id.value))
    }
    /// RPC-347: add a NEW custom model to a local-server profile. Default
    /// refuses the write (BUG-325: a handle that has not wired the
    /// custom-model write path must never report a write it did not make).
    /// The rust/sessions `SessionManager` overrides this to delegate to
    /// `profile_sections::save_custom_model(.., None)` (append).
    fn add_custom_model(
        &self,
        provider_id: &str,
        profile_name: &str,
        definition: &CustomModelDefinition,
    ) -> Result<(), String> {
        let _ = (provider_id, profile_name, definition);
        Err("this session manager does not support custom-model writes".to_string())
    }
    /// RPC-347: UPDATE an existing custom model in place. `original_model_id`
    /// names the entry to replace. Default refuses the write; the
    /// rust/sessions override delegates to
    /// `profile_sections::save_custom_model(.., Some(original_model_id))`.
    fn update_custom_model(
        &self,
        provider_id: &str,
        profile_name: &str,
        original_model_id: &str,
        definition: &CustomModelDefinition,
    ) -> Result<(), String> {
        let _ = (provider_id, profile_name, original_model_id, definition);
        Err("this session manager does not support custom-model writes".to_string())
    }
    /// RPC-347: DELETE a custom model from a local-server profile by id.
    /// Default refuses the write; the rust/sessions override delegates to
    /// `profile_sections::delete_custom_model`.
    fn delete_custom_model(
        &self,
        provider_id: &str,
        profile_name: &str,
        model_id: &str,
    ) -> Result<(), String> {
        let _ = (provider_id, profile_name, model_id);
        Err("this session manager does not support custom-model writes".to_string())
    }
    /// PROV-108: create or update a local-server profile, persisting its
    /// connection settings to `~/.fspec/fspec-config.json`. Default returns
    /// `Ok(())` (silent no-op) so handles that have not wired the profile
    /// write path — including the stub — compile without per-test wiring. The
    /// rust/sessions `SessionManager` overrides this to delegate to
    /// `profile_persistence::save_profile` (preserving `customModels` and all
    /// sibling keys) for built-in and custom providers.
    fn list_provider_settings(
        &self,
    ) -> Result<Vec<codelet_rpc_types::ProviderSettingsConfig>, String> {
        Err("Provider settings are unavailable on this session manager".into())
    }
    fn save_provider_settings(
        &self,
        provider_id: &str,
        definition: &ProfileDefinition,
    ) -> Result<(), String> {
        let _ = (provider_id, definition);
        Err("Provider settings are unavailable on this session manager".into())
    }
    fn save_profile(
        &self,
        provider_id: &str,
        profile_name: &str,
        definition: &ProfileDefinition,
    ) -> Result<(), String> {
        let _ = (provider_id, profile_name, definition);
        Ok(())
    }
    /// PROV-108: delete a local-server profile from
    /// `~/.fspec/fspec-config.json`. Default returns `Ok(())`; the
    /// rust/sessions override delegates to
    /// `profile_persistence::delete_profile` (idempotent no-op when absent).
    fn delete_profile(&self, provider_id: &str, profile_name: &str) -> Result<(), String> {
        let _ = (provider_id, profile_name);
        Ok(())
    }
    /// PROV-136: rename a local-server profile (or in-place update when
    /// `old_name == new_name`). Default returns `Ok(())` (silent no-op) so
    /// handles that have not wired the rename path — including the stub —
    /// compile without per-test wiring. The rust/sessions `SessionManager`
    /// overrides this to delegate to `profile_persistence::rename_profile`
    /// (moving the old key to `new_name`, preserving `customModels`, rejecting a
    /// collision with an existing different profile).
    fn rename_profile(
        &self,
        provider_id: &str,
        old_name: &str,
        new_name: &str,
        definition: &ProfileDefinition,
    ) -> Result<(), String> {
        let _ = (provider_id, old_name, new_name, definition);
        Ok(())
    }
    /// RPC-022: set the base thinking/reasoning level for a session.
    /// Default returns `Ok(())` (silent no-op). The rust/napi
    /// override forwards to the existing
    /// `session_set_base_thinking_level` flow.
    fn set_thinking_level(
        &self,
        session_id: &SessionId,
        level: ThinkingLevel,
    ) -> Result<(), String> {
        let _ = (session_id, level);
        Ok(())
    }
    /// RPC-027: set the PER-USER DEFAULT thinking/reasoning level.
    /// Unlike `set_thinking_level` (which is session-scoped), this
    /// persists the level so new sessions inherit it. Default returns
    /// `Ok(())` (silent no-op). The rust/napi override forwards
    /// to the future `session_set_default_thinking_level` flow.
    fn set_thinking_level_default(
        &self,
        session_id: &SessionId,
        level: ThinkingLevel,
    ) -> Result<(), String> {
        let _ = (session_id, level);
        Ok(())
    }
    /// CONT-002: set the session's auto-continue state (`/continue`).
    /// `budget` is the zero-progress nudge budget (>= 1). Default returns
    /// `Ok(())` (silent no-op) so handles that have not wired auto-continue
    /// — including the stub — compile without per-test wiring.
    fn set_continue_state(
        &self,
        session_id: &SessionId,
        enabled: bool,
        budget: u32,
    ) -> Result<(), String> {
        let _ = (session_id, enabled, budget);
        Ok(())
    }
    /// CONT-002: read the session's auto-continue state as
    /// `(enabled, budget)`. BUG-445: the default refuses.
    fn get_continue_state(&self, session_id: &SessionId) -> Result<(bool, u32), String> {
        let _ = session_id;
        Err("get_continue_state is not implemented by this session manager".to_string())
    }
        }
    };
}
