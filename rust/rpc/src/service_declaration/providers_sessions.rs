macro_rules! compose_service_providers_sessions {
    ($($collected:tt)*) => {
        compose_service_supervision_credentials! {
            $($collected)*
    /// RPC-022: list every provider and its available models. Delegates
    /// through the optional `SessionManagerHandle::list_providers` —
    /// when no handle is attached the default returns
    /// `Vec::new()`. The rust/napi `SessionManager` override reads
    /// the cached `ModelRegistry` and maps each provider/model into
    /// the transport-portable `ProviderInfo` / `ModelEntry` shape.
    async fn list_providers() -> Vec<ProviderInfo>;

    /// RPC-022: set the model bound to a session. Delegates through
    /// the optional `SessionManagerHandle::set_model`. Returns
    /// `Err(String)` only when the underlying handle returns an error
    /// (e.g. unknown model). Without an attached handle returns
    /// `Ok(())` (silent no-op, idempotent like `send_input`).
    async fn set_session_model(
        session_id: SessionId,
        provider_id: String,
        model_id: String,
    ) -> Result<(), String>;

    /// PROV-118: set the in-process DEFAULT model used by `create_session`
    /// when no per-session model is bound. Delegates through the optional
    /// `SessionManagerHandle::set_default_model`. Without an attached handle
    /// returns `Ok(())` (silent no-op, idempotent like `set_session_model`).
    async fn set_default_model(model: String) -> Result<(), String>;

    /// RPC-347: add a NEW custom model to a local-server profile. Delegates
    /// through `SessionManagerHandle::add_custom_model` (which calls
    /// `profile_sections::save_custom_model(.., None)`). Without an attached
    /// handle returns `Ok(())` (silent no-op, idempotent like
    /// `set_session_model`).
    async fn add_custom_model(
        provider_id: String,
        profile_name: String,
        definition: CustomModelDefinition,
    ) -> Result<(), String>;

    /// RPC-347: UPDATE an existing custom model in place. `original_model_id`
    /// names the entry to replace. Delegates through
    /// `SessionManagerHandle::update_custom_model`; without a handle returns
    /// `Ok(())`.
    async fn update_custom_model(
        provider_id: String,
        profile_name: String,
        original_model_id: String,
        definition: CustomModelDefinition,
    ) -> Result<(), String>;

    /// RPC-347: DELETE a custom model from a local-server profile by id.
    /// Delegates through `SessionManagerHandle::delete_custom_model`; without
    /// a handle returns `Ok(())`.
    async fn delete_custom_model(
        provider_id: String,
        profile_name: String,
        model_id: String,
    ) -> Result<(), String>;

    /// PROV-108: create or update a local-server profile. Delegates through
    /// `SessionManagerHandle::save_profile` (read-modify-write preserving
    /// customModels + sibling keys). Without an attached handle returns
    /// `Ok(())` (silent no-op, idempotent like `set_session_model`).
    async fn list_provider_settings(
    ) -> Result<Vec<codelet_rpc_types::ProviderSettingsConfig>, String>;

    async fn save_provider_settings(
        provider_id: String,
        definition: ProfileDefinition,
    ) -> Result<(), String>;

    async fn save_profile(
        provider_id: String,
        profile_name: String,
        definition: ProfileDefinition,
    ) -> Result<(), String>;

    /// PROV-108: delete a local-server profile. Delegates through
    /// `SessionManagerHandle::delete_profile`; without a handle returns
    /// `Ok(())`.
    async fn delete_profile(provider_id: String, profile_name: String) -> Result<(), String>;

    /// PROV-136: rename a local-server profile (or in-place update when
    /// `old_name == new_name`). Delegates through
    /// `SessionManagerHandle::rename_profile` (single read-modify-write moving
    /// the old key to `new_name`, preserving customModels, rejecting a
    /// collision). Without an attached handle returns `Ok(())`.
    async fn rename_profile(
        provider_id: String,
        old_name: String,
        new_name: String,
        definition: ProfileDefinition,
    ) -> Result<(), String>;

    /// RPC-022: set the per-session thinking/reasoning level.
    /// Delegates through `SessionManagerHandle::set_thinking_level`.
    /// Returns `Err(String)` on handle error; without an attached
    /// handle returns `Ok(())`.
    async fn set_thinking_level(session_id: SessionId, level: ThinkingLevel) -> Result<(), String>;

    /// CONT-002: set the session's auto-continue state (`/continue`).
    /// Delegates through `SessionManagerHandle::set_continue_state`.
    /// Without an attached handle returns `Ok(())`.
    async fn set_continue_state(
        session_id: SessionId,
        enabled: bool,
        budget: u32,
    ) -> Result<(), String>;

    /// CONT-002: read the session's auto-continue state as
    /// `(enabled, budget)`. Without an attached handle returns
    /// `(false, 10)`.
    async fn get_continue_state(session_id: SessionId) -> (bool, u32);

    /// CONT-003: set or clear the session's goal chrome state (`/goal`)
    /// as `(text, verify)`. Delegates through
    /// `SessionManagerHandle::set_goal_state`. Without an attached
    /// handle returns `Ok(())`.
    async fn set_goal_state(
        session_id: SessionId,
        goal: Option<(String, Option<String>)>,
    ) -> Result<(), String>;

    /// CONT-003: read the session's goal chrome state as
    /// `(text, verify)`. Without an attached handle returns `None`.
    async fn get_goal_state(session_id: SessionId) -> Option<(String, Option<String>)>;

    /// RPC-022: read the session's current role overlay text.
    /// Delegates through `SessionManagerHandle::get_role`. Without an
    /// attached handle returns `None`.
    async fn get_session_role(session_id: SessionId) -> Option<String>;

    /// RPC-022: set or clear the session's role overlay. Passing
    /// `None` clears. Delegates through
    /// `SessionManagerHandle::set_role`. Without an attached handle
    /// returns `Ok(())` (silent no-op).
    async fn set_session_role(session_id: SessionId, role: Option<String>) -> Result<(), String>;

    // ========================================================================
    // RPC-037: Widened tarpc surface for AgentView parity. Each method below
    // mirrors an addition on `SessionManagerHandle`; `FspecServiceImpl`
    // delegates via `self.inner.session_manager()` and returns safe defaults
    // when no handle is attached.
    // ========================================================================

    /// RPC-037: send user input with provider-specific thinking config.
    async fn send_input_with_thinking(
        session_id: SessionId,
        text: String,
        thinking: Option<ThinkingConfig>,
    );

    /// RPC-037: per-session input/output token totals.
    async fn get_session_tokens(session_id: SessionId) -> SessionTokens;

    /// RPC-037: per-session model binding (provider + model + limits).
    async fn get_session_model(session_id: SessionId) -> SessionModel;

    /// RPC-037: in-flight compaction progress, if any.
    async fn get_compaction_progress(session_id: SessionId) -> Option<CompactionProgress>;

    /// RPC-037: replay-buffer of recent stream chunks for a session.
    async fn get_buffered_output(session_id: SessionId, limit: u32) -> Vec<StreamChunk>;

    /// RPC-037: clear session history.
    async fn clear_history(session_id: SessionId) -> Result<(), String>;

    /// RPC-037: compact session history and return statistics.
    async fn compact_session(session_id: SessionId) -> Result<CompactionResult, String>;

    /// RPC-037: restore session messages from raw JSONL envelopes.
    async fn restore_session_messages(
        session_id: SessionId,
        envelopes: Vec<String>,
    ) -> Result<(), String>;

    /// RPC-037: restore cumulative-billed counters and cache totals.
    async fn restore_session_token_state(
        session_id: SessionId,
        state: TokenRestoreState,
    ) -> Result<(), String>;

    /// RPC-049: durable-restore aggregate that orchestrates load_session
    /// + get_session_message_envelopes + restore_session_messages +
    /// restore_session_token_state in a single round-trip. Used by the
    /// TUI's `/resume` flow.
    async fn resume_session(session_id: SessionId) -> Result<(), String>;

    async fn get_work_unit_focus(session_id: SessionId) -> Result<WorkUnitFocus, String>;

    async fn set_work_unit_focus(
        session_id: SessionId,
        work_unit_id: Option<String>,
        source: String,
    ) -> Result<WorkUnitFocus, String>;

    /// RPC-037: read the work-unit context bound to a session.
    async fn get_work_unit_context(session_id: SessionId) -> Option<WorkUnitContext>;

    /// RPC-037: bind (or detach) a work unit on a session.
    async fn set_work_unit_context(
        session_id: SessionId,
        context: Option<WorkUnitContext>,
    ) -> Result<(), String>;

    /// RPC-037: read the per-session pending input draft.
    async fn get_pending_input(session_id: SessionId) -> Option<String>;

    /// RPC-037: write the per-session pending input draft.
    async fn set_pending_input(session_id: SessionId, text: Option<String>);

    /// RPC-037: set the active session for the application.
    async fn set_active_session(session_id: SessionId);

    /// RPC-037: clear the active session.
    async fn clear_active_session();

    /// RPC-037: read the active session, if any.
    async fn get_active_session() -> Option<SessionId>;

    /// RPC-037: effective cwd for a session (worktree-aware). Returned
    /// as a String so the wire shape stays portable.
    async fn get_effective_cwd(session_id: SessionId) -> String;
        }
    };
}
