macro_rules! compose_action_session_controls {
    ($($collected:tt)*) => {
        compose_action_project_files! {
            $($collected)*
            // ========================================================================
            // RPC-060: CreateSessionDialog + isolated-session creation actions.
            // The `/isolation` slash command and any future "new session" entry
            // point dispatches `OpenCreateSessionDialog`; the dialog itself emits
            // `CreateSessionSubmitted` (on Yes / Yes - Isolated) or
            // `CreateSessionCancelled` (on Cancel / Esc). App::dispatch routes
            // CreateSessionSubmitted into either `backend.create_session(None)`
            // (isolated=false) or `backend.create_isolated_session(None)`
            // (isolated=true), folds the response into Action::SessionCreated,
            // and on error emits `Action::EmitSessionNotice` against the focused
            // session.
            // ========================================================================
            /// RPC-060: emitted by the `/isolation` slash command palette pick
            /// (and any future explicit "new session" trigger). App::dispatch
            /// pushes a fresh [`crate::components::create_session_dialog::CreateSessionDialog`]
            /// onto the Compositor at Priority::Foreground, preselecting the
            /// option indicated by `preselect`.
            OpenCreateSessionDialog {
                preselect: Option<crate::components::create_session_dialog::CreateSessionOption>,
            },
            /// RPC-060: emitted by the CreateSessionDialog on Enter over "Yes"
            /// or "Yes - Isolated". App::dispatch spawns
            /// `backend.create_session(None)` (isolated=false) or
            /// `backend.create_isolated_session(None)` (isolated=true) and
            /// folds the result into `Action::SessionCreated`. On error the
            /// dispatch helper surfaces a `[error] create [isolated ]session: …`
            /// notice to the focused session via `Action::EmitSessionNotice`.
            CreateSessionSubmitted {
                isolated: bool,
            },
            /// RPC-060: emitted by the CreateSessionDialog on Enter over
            /// "Cancel" or on Esc. App::dispatch is a silent no-op — the
            /// callback returned by `handle_event` removes the dialog from the
            /// Compositor.
            CreateSessionCancelled,

            // ========================================================================
            // RPC-061: supervisor / subordinate links surface.
            // SupervisorsLoaded is the result of an async backend.get_supervisors
            // snapshot — the dispatch helper folds the list into AgentViewStore so
            // the SessionHeader paints the cyan `[Subordinate of: <id>]` badge.
            // SendToSubordinate is dispatched by a supervisor's UI to forward an
            // `IncomingMessageInput` payload onto a subordinate session via
            // `backend.receive_incoming_message`.
            // ========================================================================
            /// RPC-061: result of a `backend.get_supervisors(session_id)` snapshot.
            /// `App::dispatch` writes the list into `AgentViewStore` so the
            /// next render of the SessionHeader can paint the subordinate badge.
            SupervisorsLoaded(
                codelet_rpc_types::SessionId,
                Vec<codelet_rpc_types::SessionId>,
            ),
            /// RPC-061: dispatched by a supervisor's UI to forward an
            /// `IncomingMessageInput` payload onto `subordinate_id` via
            /// `backend.receive_incoming_message`. On `Err` the dispatch helper
            /// surfaces a `[error] send to subordinate: <e>` notice against the
            /// originating supervisor session via `Action::EmitSessionNotice`.
            SendToSubordinate {
                subordinate_id: codelet_rpc_types::SessionId,
                message: codelet_rpc_types::IncomingMessageInput,
            },
            /// RPC-079: request the Compositor pop the dialog identified by
            /// the given stable id. Emitted by `NotificationDialog` and
            /// `StatusDialog` (Complete state) when their auto-dismiss timer
            /// fires. `App::dispatch` routes this via `try_dispatch_dialog_dismiss`
            /// which calls `compositor.remove(&id)` and triggers a redraw.
            DismissDialog(String),

        }
    };
}
