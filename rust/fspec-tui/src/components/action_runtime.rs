macro_rules! compose_action_runtime {
    ($($collected:tt)*) => {
        compose_action_board! {
            $($collected)*
            Quit,
            Transcript(crate::app::transcript_sync::TranscriptAction),
            /// Render request — typically emitted after a state change.
            Redraw,
            /// Future-proofing for tests + later cards: a string-tagged action
            /// the App fans out across the compositor without interpreting.
            Custom(String),
            /// RPC-009: trigger a fresh `backend.list_work_units()` snapshot
            /// fetch (used by the work_units subscriber task on
            /// `RecvError::Lagged`).
            LoadWorkUnits,
            /// RPC-009: a fresh work-units snapshot has arrived (either from
            /// bootstrap's `list_work_units()` or from the broadcast subscriber
            /// task converting `work_units_rx` messages).
            WorkUnitsLoaded(Vec<codelet_rpc_types::WorkUnitInfo>),
            /// RPC-009: bootstrap's `create_session(None)` returned a session id;
            /// the AgentReplView records it as its active session.
            SessionCreated(codelet_rpc_types::SessionId),
            ContinueStateLoaded(
                codelet_rpc_types::SessionId,
                u64,
                Result<Option<(bool, u32)>, String>,
            ),
            /// RPC-434: a session was destroyed on the daemon (from this client
            /// or from ANY other connected machine). Folded onto the action bus
            /// by the bootstrap's session-removed subscriber (both transports —
            /// the WS transport splits the RPC-433 `SessionEvent` wire stream
            /// onto per-variant receivers); `App::dispatch` routes it to the
            /// idempotent `handle_session_removed` (app/dispatch_session_lifecycle.rs).
            SessionRemoved(codelet_rpc_types::SessionId),
            /// RPC-437: a session's history was cleared on the daemon (from
            /// this client or from ANY other connected machine, including the
            /// NAPI/Telegram clear entry point). Folded onto the action bus by
            /// the bootstrap's session-cleared subscriber (both transports —
            /// the WS transport splits the RPC-433 `SessionEvent` wire stream
            /// onto per-variant receivers); `App::dispatch` routes it to the
            /// idempotent `handle_session_cleared`
            /// (app/dispatch_session_lifecycle.rs). Appended after
            /// `SessionRemoved` (RPC-434) so the existing variant order is
            /// untouched.
            SessionCleared(codelet_rpc_types::SessionId),
            /// PROV-101: `create_session` returned an EMPTY `SessionId` because no
            /// default model was set (decline). The TUI must NOT append an empty-id
            /// session — every caller maps the empty id to THIS action via
            /// `app::session_creation::post_create_session_action`, and App::dispatch
            /// surfaces it as a Priority::Critical ErrorDialog so the decline is
            /// explicit instead of being silently swallowed.
            SessionCreationDeclined,
            SessionOperationFailed(String),
            SessionManifestIssuesLoaded {
                issues: Vec<codelet_rpc_types::SessionManifestIssue>,
                replay: bool,
            },
            SessionStorageDismissed,
            /// RPC-009: a streaming chunk arrived for the (session_id, chunk)
            /// pair. The chunks_rx subscriber filters by the AgentReplView's
            /// active session id BEFORE emitting this variant.
            ChunkReceived(codelet_rpc_types::SessionId, codelet_rpc_types::StreamChunk),
            /// RPC-009: the user pressed Enter on the AgentReplView's input
            /// box. The App's run loop dispatches `backend.send_input` and then
            /// forwards into compositor.update so layers can react.
            InputSubmitted(String),
            /// RPC-009: the user pressed Ctrl+C on the AgentReplView's input
            /// box. The App's run loop dispatches `backend.interrupt` —
            /// Ctrl+C is NOT a quit (Ctrl+D / `q` are).
            Interrupt,
            /// RPC-009: the user pressed Tab; the App's run loop mutates
            /// RootView's `focused_pane` field to alternate between WorkUnits
            /// and Repl.
            FocusNext,
            /// RPC-011 CR-1: the WebSocketFspecBackend has detected its
            /// underlying WS stream has dropped. Emitted onto the action bus
            /// by the transport-layer supervisor task (rule [0] / [18]).
            /// App.dispatch() pushes a DisconnectDialog @ Priority::Critical
            /// onto the Compositor in response.
            Disconnected,
            /// RPC-011 auto-reconnect: the supervisor task is about to attempt
            /// connect_async for the Nth time (1-indexed). The DisconnectDialog
            /// re-renders its body to "auto-reconnecting (attempt N)…".
            Reconnecting(u32),
            /// RPC-011 auto-reconnect: the supervisor successfully reconnected
            /// AND re-issued bootstrap (list_work_units + create_session(None)).
            /// RPC-415: App.dispatch() also respawns the five broadcast subscriber
            /// tasks (work_units / chunks / logs / status_changes / session_created)
            /// bound to the new client's receivers, then pops the DisconnectDialog
            /// from the Compositor.
            Reconnected,
            /// RPC-011 CR-1: the user pressed 'r' while DisconnectDialog was
            /// topmost. The supervisor cancels its current backoff sleep, tries
            /// connect immediately, and resets the backoff schedule.
            ManualReconnect,
            /// RPC-416: fired by the auto-dismiss timer armed on `Reconnected`.
            /// Carries the ORIGINATING session id + the stable scrollback `seq`
            /// of the inline reconnect notice so `App::dispatch` removes exactly
            /// that chunk (silent no-op if the session closed or the notice was
            /// already superseded by a re-drop).
            ClearReconnectNotice {
                /// The session the reconnect notice was pushed into.
                session_id: codelet_rpc_types::SessionId,
                /// The stable scrollback seq of the notice chunk to remove.
                seq: u64,
            },
            /// RPC-417: 10s auto-hide fired — seq-guarded COMPACTED badge clear.
            ClearCompactionReduction {
                session_id: codelet_rpc_types::SessionId,
                seq: u64,
            },
        }
    };
}
