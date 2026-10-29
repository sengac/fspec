macro_rules! compose_action_session_context {
    ($($collected:tt)*) => {
        compose_action_prompts! {
            $($collected)*
            /// RPC-022: emitted by the slash command popup (on Enter over the
            /// Thinking row) AND by `parse_slash_command("/thinking")`.
            /// App::dispatch pushes a fresh ThinkingLevelDialog onto the
            /// Compositor at Priority::Foreground seeded with the cached
            /// thinking level for the focused session.
            OpenThinkingDialog,
            /// RPC-022: a spawned `backend.list_providers()` task resolved.
            /// App::dispatch folds the result into the open ModelSelector
            /// mode-view. No-op when the view is not active.
            ListProvidersLoaded(Vec<codelet_rpc_types::ProviderInfo>),
            /// RPC-022: emitted by the ModelSelector mode-view on Enter over a
            /// model row. App::dispatch spawns `backend.set_session_model(...)` then
            /// re-runs `backend.get_model_info(...)` to refresh the
            /// SessionHeader chrome via `Action::ModelInfoLoaded`.
            /// PROV-117: the session id is `Option` — TS parity, the Enter handler
            /// has no session guard and always emits a selection; the backend write
            /// is skipped (and the selector still closes) when there is no session.
            ModelSelected(Option<codelet_rpc_types::SessionId>, String, String),
            /// RPC-022: emitted by ThinkingLevelDialog on Enter over a level
            /// row. App::dispatch spawns `backend.set_thinking_level(...)`
            /// then re-runs `backend.get_thinking_level(...)` to refresh the
            /// `[T:Level]` badge via `Action::ThinkingLevelLoaded`.
            ThinkingLevelSelected(
                codelet_rpc_types::SessionId,
                codelet_rpc_types::ThinkingLevel,
            ),
            /// RPC-022: emitted by `parse_slash_command("/role …")` and by
            /// the bare `/role clear` path. App::dispatch updates
            /// AgentViewStore.role_by_session AND spawns
            /// `backend.set_session_role(...)`.
            SetSessionRole(codelet_rpc_types::SessionId, Option<String>),
            /// RPC-022: a spawned `backend.get_session_role(...)` task
            /// resolved. App::dispatch folds the result into
            /// AgentViewStore.role_by_session so the RoleBanner repaints. No
            /// backend write is fired for this — it is purely a read path.
            SessionRoleLoaded(codelet_rpc_types::SessionId, Option<String>),
            /// RPC-027: emitted by ThinkingLevelDialog on the `D` / `d` key.
            /// App::dispatch spawns `backend.set_thinking_level_default(...)`.
            /// The dialog stays open; no badge refresh fires because the
            /// default does not change the current session's effective level.
            SetThinkingLevelDefault(
                codelet_rpc_types::SessionId,
                codelet_rpc_types::ThinkingLevel,
            ),
            /// RPC-045: emitted by the status_changes_rx subscriber when the
            /// SessionManager broadcasts a (SessionId, SessionStatus) transition.
            /// App::dispatch writes the payload into
            /// `AgentViewStore.session_status_by_session` so the SessionFooter
            /// status pill repaints push-driven on the next render — replacing
            /// the polling `get_session_status` path that previously refreshed
            /// the chrome.
            SessionStatusChanged(
                codelet_rpc_types::SessionId,
                codelet_rpc_types::SessionStatus,
            ),
            /// RPC-046: emitted by spawned tokio tasks (currently the /clear
            /// `backend.clear_history` round-trip in `dispatch_slash_commands`) to
            /// route a `[notice]` / `[error]` scrollback line back to the
            /// ORIGINATING session — regardless of which session is currently
            /// focused when the response arrives. App::dispatch handles this
            /// by looking up the SessionContext for the supplied id and
            /// pushing the text via `SessionContext::push_line`. Silently
            /// drops when the session has been closed in the meantime.
            EmitSessionNotice(codelet_rpc_types::SessionId, String),
            SessionResumeComplete(codelet_rpc_types::SessionId),
            /// RPC-430: emitted by the debug-hydration task spawned during
            /// session attach (`handle_attach_to_session`) or by the debug
            /// propagation task spawned during session creation. Carries the
            /// session id and the resolved debug-capture flag. App::dispatch
            /// stores the value in `AgentViewStore.set_debug_enabled`.
            DebugEnabledLoaded(codelet_rpc_types::SessionId, bool),
            /// RPC-050: emitted by BoardView's Enter handler (and the explicit
            /// "attach" path) to bind the supplied work-unit id to the
            /// currently-focused AgentView session. App::dispatch routes this
            /// through `handle_attach_work_unit_to_session`, which spawns
            /// `backend.set_work_unit_context(session_id, Some(ctx))` and on
            /// Ok dispatches `Action::WorkUnitAttached`. With no current
            /// session the helper is a silent no-op.
            AttachWorkUnitToSession(String),
            /// RPC-050: emitted by the spawned `set_work_unit_context(Some)`
            /// task on the Ok branch. App::dispatch routes this through
            /// `handle_work_unit_attached`, which folds the supplied
            /// `WorkUnitContext` into `AgentViewStore.work_unit_context_by_session`
            /// so the SessionHeader chip and downstream per-session readers see
            /// the binding on the next frame.
            WorkUnitFocusChanged(
                codelet_rpc_types::SessionId,
                codelet_rpc_types::WorkUnitFocus,
            ),
            SessionObserved(codelet_rpc_types::SessionInfo),
            WorkUnitSessionCreated(codelet_rpc_types::SessionId, String),
            WorkUnitAttached(
                codelet_rpc_types::SessionId,
                codelet_rpc_types::WorkUnitContext,
            ),
            /// RPC-051: emitted by the AgentView's default Esc arm (after the
            /// popup → compositor-dialog → mode-view cascade levels have
            /// already had a chance to consume the keypress). App::dispatch
            /// routes this through `handle_agent_esc_pressed`, which:
            ///   - if the current session has SessionStatus::Running or
            ///     SessionStatus::Compacting → spawn `backend.interrupt(id)`
            ///     and stay on the AgentView (no navigation), matching the TS
            ///     Ink AgentView's Esc behaviour;
            ///   - otherwise → dispatch Action::BackToBoard.
            ///
            /// When no current session is open this is a fast path back to
            /// the Board view (silent no-op for the interrupt branch).
            AgentEscPressed,
            /// RPC-098: emitted by [`exit_confirmation_dialog::ExitConfirmationDialog`]
            /// when the user picks one of the two options (Close Session / Cancel).
            /// Routed through `App::dispatch` to `handle_agent_exit_choice`:
            ///   - `ExitChoice::Cancel` → no-op (dialog already removed via Callback);
            ///   - `ExitChoice::CloseSession` → spawn `backend.destroy_session(id)` as
            ///     a pending task, then dispatch `Action::BackToBoard`.
            AgentExitChoice {
                choice: exit_confirmation_dialog::ExitChoice,
            },
            /// RPC-052: emitted by the AgentView's `views/agent/dispatch.rs`
            /// after `MultiLineInput::handle_event` returns Continued AND the
            /// buffer text actually changed (before/after value comparison).
            /// App::dispatch routes this through `handle_pending_input_changed`,
            /// which:
            ///   - mirrors the new text into the current SessionContext's
            ///     `input_draft` synchronously (no backend round-trip on
            ///     Shift+Left/Right cycling — RPC-024 fast path preserved);
            ///   - aborts any in-flight `pending_input_save_handle`;
            ///   - spawns a fresh task that sleeps 300ms then calls
            ///     `backend.set_pending_input(session, Some(text))`. Errors
            ///     are swallowed via `tracing` — no scrollback notice.
            ///
            /// With no current session this is a silent no-op.
            PendingInputChanged(String),
            /// RPC-052: emitted by the spawned `spawn_hydrate_pending_input`
            /// task on session activation when `backend.get_pending_input(id)`
            /// returns `Ok(Some(text))`. App::dispatch routes this through
            /// `handle_seed_pending_input`, which:
            ///   - always folds the text into the matching
            ///     `SessionContext.input_draft` (so the next cycle restores
            ///     the same draft);
            ///   - ONLY seeds the live `MultiLineInput` when the activated
            ///     session is still the focused session at the moment the
            ///     hydration completes (race-safe — late results targeting a
            ///     stale session are dropped from the live input).
            SeedPendingInput {
                session_id: codelet_rpc_types::SessionId,
                text: String,
            },
        }
    };
}
