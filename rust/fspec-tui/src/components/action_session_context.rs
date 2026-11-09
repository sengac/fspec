macro_rules! compose_action_session_context {
    ($($collected:tt)*) => {
        compose_action_prompts! {
            $($collected)*
            /// RPC-022: a spawned `backend.list_providers()` task resolved.
            /// BUG-576: it carries the number of its request; the
            /// ModelSelector applies only the newest one requested.
            ListProvidersLoaded(u64, Vec<codelet_rpc_types::ProviderInfo>),
            /// BUG-576: the selector's list numbered `0` failed with `1`.
            ListProvidersFailed(u64, String),
            /// RPC-022: emitted by the ModelSelector mode-view on Enter over a
            /// model row. App::dispatch spawns `backend.set_session_model(...)` then
            /// re-runs `backend.get_model_info(...)` to refresh the
            /// SessionHeader chrome via `Action::ModelInfoLoaded`.
            /// PROV-117: the session id is `Option` — TS parity, the Enter handler
            /// has no session guard and always emits a selection; the backend write
            /// is skipped (and the selector still closes) when there is no session.
            ModelSelected(Option<codelet_rpc_types::SessionId>, String, String),
            /// BUG-547: a model selection or session creation was refused
            /// because a local profile model is missing a limit; the App
            /// opens the missing-limit prompt.
            OpenMissingProfileLimitPrompt {
                limit: codelet_sessions::profile_limits::MissingProfileLimit,
                retry: crate::components::missing_profile_limit_dialog::MissingLimitRetry,
            },
            /// BUG-547: the prompt's value; the App saves it to the profile,
            /// then runs `retry` again.
            SaveMissingProfileLimit {
                limit: codelet_sessions::profile_limits::MissingProfileLimit,
                value: u32,
                retry: crate::components::missing_profile_limit_dialog::MissingLimitRetry,
            },
            /// BUG-547: the prompt was cancelled; the App shows the refusal.
            MissingProfileLimitCancelled {
                limit: codelet_sessions::profile_limits::MissingProfileLimit,
                retry: crate::components::missing_profile_limit_dialog::MissingLimitRetry,
            },
            /// BUG-547: the missing limit was saved; run the selection or
            /// creation again.
            RetryAfterMissingProfileLimit(
                crate::components::missing_profile_limit_dialog::MissingLimitRetry,
            ),
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
            /// RPC-430: emitted by the debug-hydration task spawned during
            /// session attach (`handle_attach_to_session`) or by the debug
            /// propagation task spawned during session creation. Carries the
            /// session id and the resolved debug-capture flag. App::dispatch
            /// stores the value in `AgentViewStore.set_debug_enabled`.
            DebugEnabledLoaded(codelet_rpc_types::SessionId, bool),
            /// BUG-230: a session's work-unit focus, read after an attach or a
            /// focus change. App::dispatch folds it into the AgentViewStore and
            /// rebuilds the board's focus occupancy.
            WorkUnitFocusChanged(
                codelet_rpc_types::SessionId,
                codelet_rpc_types::WorkUnitFocus,
            ),
            SessionObserved(codelet_rpc_types::SessionInfo),
            WorkUnitSessionCreated(codelet_rpc_types::SessionId, String),
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
            /// when the user picks one of the two options (Close / Cancel).
            /// Routed through `App::dispatch` to `handle_agent_exit_choice`:
            ///   - `ExitChoice::Cancel` → the gathered close facts are dropped;
            ///   - `ExitChoice::CloseSession` → TUI-124: close the bot's
            ///     sub-agents and then the session, one backend close at a
            ///     time, ending in `Action::CloseFinished`.
            AgentExitChoice {
                choice: exit_confirmation_dialog::ExitChoice,
            },
            /// TUI-124: the close relations of `session` were fetched (its
            /// sub-agents, the sessions that started it): open the close
            /// dialog, or report why nothing was closed.
            CloseFactsLoaded {
                session: codelet_rpc_types::SessionId,
                kind: exit_confirmation_dialog::SessionKind,
                result: std::result::Result<exit_confirmation_dialog::CloseRelations, String>,
            },
            /// TUI-124: a confirmed close ended. `closed` holds every session
            /// whose backend close succeeded, in order; `failure` is the
            /// report when the close stopped early (the rest stay open).
            CloseFinished {
                facts: exit_confirmation_dialog::CloseFacts,
                closed: Vec<codelet_rpc_types::SessionId>,
                failure: Option<String>,
            },
            /// RPC-052: emitted by the AgentView's `views/agent/dispatch.rs`
            /// after `MultiLineInput::handle_event` returns Continued AND the
            /// buffer text actually changed (before/after value comparison).
            /// App::dispatch routes this through `handle_pending_input_changed`,
            /// which (BUG-549) records the composer's CURRENT text as the
            /// draft of the session the composer is bound to: the
            /// SessionContext `input_draft` mirror plus a per-session ordered
            /// daemon save (300ms debounce). The reported text is not trusted
            /// — focus may have moved since the key. Errors are swallowed via
            /// `tracing` — no scrollback notice.
            ///
            /// With no current session this is a silent no-op.
            PendingInputChanged(String),
            /// RPC-052 / BUG-549: code seeds a session's draft (the RPC-057
            /// merge-conflict context). App::dispatch routes this through
            /// `handle_seed_pending_input`: a draft change like typing — the
            /// matching `SessionContext.input_draft`, the live composer when
            /// it holds that session's draft, and a daemon save. Dropped when
            /// the session's tab is not open.
            SeedPendingInput {
                session_id: codelet_rpc_types::SessionId,
                text: String,
            },
            /// BUG-549: the attach hydration read `text` (empty: no draft)
            /// from the daemon at the session's draft `generation`. Applied
            /// by `handle_pending_input_hydrated` only when no draft change
            /// was scheduled for the session since the read.
            PendingInputHydrated {
                session_id: codelet_rpc_types::SessionId,
                text: String,
                generation: u64,
            },
        }
    };
}
