macro_rules! compose_action_prompts {
    ($($collected:tt)*) => {
        compose_action_provider_settings! {
            $($collected)*
            /// RPC-053: emitted by the chunk dispatcher when
            /// `StreamChunk::SessionStateChange { state: Paused }` arrives.
            /// App::dispatch routes this through `handle_pause_chunk`, which
            /// spawns parallel `backend.get_pause_state(id)` and
            /// `backend.get_hitl_request(id)` reads. The first non-None result
            /// dispatches `Action::PauseStateFetched` (PauseState → RPC-406
            /// inline prompt slot) or `Action::HitlPromptFetched` (HitlRequest
            /// → RPC-411 inline prompt slot). When both return Some the HITL
            /// prompt wins (the HITL handler in the agent loop is the only
            /// path that populates `hitl_request`).
            PauseChunkReceived(codelet_rpc_types::SessionId),
            /// RPC-053: emitted by the chunk dispatcher when
            /// `StreamChunk::SessionStateChange { state: Running | Idle }`
            /// arrives. App::dispatch routes this through
            /// `handle_pause_cleared`, which clears the RPC-406 per-session
            /// pause slot AND the RPC-411 HITL slot so the UI does not
            /// strand a stale prompt after the agent loop has resumed
            /// server-side.
            PauseCleared(codelet_rpc_types::SessionId),
            /// RPC-406: emitted by `handle_pause_chunk` on `Ok(Some(state))`.
            /// App::dispatch stores the fetched PauseState into the
            /// AgentViewStore per-session pause slot — the AgentView paints
            /// the inline tool-approval prompt from that slot when the paused
            /// session is focused (replaces the deleted RPC-053 modal push).
            PauseStateFetched {
                session_id: codelet_rpc_types::SessionId,
                state: codelet_rpc_types::PauseState,
            },
            /// RPC-406: emitted by the AgentView pause-prompt key handler on
            /// ←/→ while a Triple pause prompt is showing. App::dispatch
            /// cycles the per-session selection with wraparound
            /// (0 = Allow Once, 1 = Allow Session, 2 = Deny).
            PausePromptNav {
                session_id: codelet_rpc_types::SessionId,
                delta: i32,
            },
            /// RPC-406: emitted by the AgentView pause-prompt key handler on
            /// Enter while a Triple pause prompt is showing. App::dispatch
            /// reads the authoritative selection from the store, maps it onto
            /// `ApprovalChoice::{Approve, ApproveSession, Deny}`, and routes
            /// through `handle_pause_triple` (which clears the slot).
            PausePromptEnter {
                session_id: codelet_rpc_types::SessionId,
            },
            /// RPC-411: emitted by `handle_pause_chunk` on
            /// `Ok(Some(hitl_request))`. App::dispatch stores the fetched
            /// request into the AgentViewStore per-session HITL slot — the
            /// AgentView paints the inline HITL prompt from that slot when the
            /// session is focused (replaces the deleted RPC-053 modal push).
            HitlPromptFetched {
                session_id: codelet_rpc_types::SessionId,
                request: codelet_rpc_types::HitlRequest,
            },
            /// RPC-411: ↑/↓ on an options question — cycle the selection with
            /// wraparound over `options.len() + 1` (virtual "Other...").
            HitlPromptNav {
                session_id: codelet_rpc_types::SessionId,
                delta: i32,
            },
            /// RPC-411: Enter on an options question. The reducer reads the
            /// authoritative slot state: Other... selected → enter Other mode,
            /// else capture `{id, selected:[label]}` and advance-or-submit.
            HitlPromptEnter {
                session_id: codelet_rpc_types::SessionId,
            },
            /// RPC-411: Enter in freeform/Other mode with non-empty text —
            /// the key handler reads + clears the SHARED composer input and
            /// carries the value here so the reducer stays store-authoritative.
            /// Captures `{id, selected:[], other:text}` and advance-or-submits.
            HitlAnswerCaptured {
                session_id: codelet_rpc_types::SessionId,
                text: String,
            },
            /// RPC-411: Esc in Other mode — local only: back to the options
            /// list, clears the empty-submit hint. NOTHING is sent.
            HitlOtherExit {
                session_id: codelet_rpc_types::SessionId,
            },
            /// RPC-411: empty/whitespace Enter in freeform/Other mode — sets
            /// the yellow empty-submit hint. NOTHING is sent.
            HitlEmptySubmit {
                session_id: codelet_rpc_types::SessionId,
            },
            /// RPC-411: typing in freeform/Other mode while the empty-submit
            /// hint is showing — clears the hint (useHitlInput.ts:201-208).
            HitlHintCleared {
                session_id: codelet_rpc_types::SessionId,
            },
            /// RPC-411: Esc outside Other mode — cancel the WHOLE request:
            /// sends `HitlResponse { cancelled: true, answers: [] }` via the
            /// backend, THEN clears the slot (no path clears without sending).
            HitlCancelled {
                session_id: codelet_rpc_types::SessionId,
            },
            /// RPC-053: emitted by the RPC-406 inline pause prompt (Confirm
            /// kind) on Y/N/Esc. App::dispatch routes this through
            /// `handle_pause_confirmed`, which clears the per-session pause
            /// slot and fires `backend.pause_confirm(session_id, accept)`
            /// fire-and-forget (best-effort UX so a slow backend does not
            /// leave the prompt dangling on screen).
            PauseConfirmed {
                session_id: codelet_rpc_types::SessionId,
                accept: bool,
            },
            /// RPC-053: emitted by the RPC-406 inline pause prompt (Triple
            /// kind) — via the `PausePromptEnter` reducer or directly with
            /// `ApprovalChoice::Deny` on Esc. App::dispatch routes this
            /// through `handle_pause_triple`, which clears the per-session
            /// pause slot and fires `backend.pause_triple(session_id, choice)`
            /// fire-and-forget.
            PauseTriple {
                session_id: codelet_rpc_types::SessionId,
                choice: codelet_rpc_types::ApprovalChoice,
            },
            /// RPC-053: routes through `handle_pause_resumed`, which fires
            /// `backend.pause_resume(session_id)` fire-and-forget. The agent
            /// loop receives the resume signal and unblocks from
            /// `wait_for_pause_response`. RPC-406: NOT reachable from the
            /// inline pause prompt (Esc DENIES) — kept for other callers
            /// (Continue-kind internal pauses via the napi path).
            PauseResumed {
                session_id: codelet_rpc_types::SessionId,
            },
            /// RPC-053/RPC-411: routes through `handle_hitl_submitted`, which
            /// fires `backend.send_hitl_response(session_id, response)`
            /// fire-and-forget and clears the per-session HITL slot.
            HitlSubmitted {
                session_id: codelet_rpc_types::SessionId,
                response: codelet_rpc_types::HitlResponse,
            },

            // ========================================================================
            // TOOL-022 P2: exec-stdin inline prompt actions. The prompt is a
            // non-blocking overlay in the SAME composer slot as HITL (precedence
            // HITL > exec-stdin > pause > composer); it has NO response channel
            // and does NOT flip the session status.
            // ========================================================================
            /// Authoritative exec-stdin request update for the session's overlay.
            ExecStdinPromptFetched {
                agent_session: codelet_rpc_types::SessionId,
                request: codelet_rpc_types::ExecStdinRequest,
            },
            ExecStdinProbeFetched {
                agent_session: codelet_rpc_types::SessionId,
                generation: u64,
                request: Option<codelet_rpc_types::ExecStdinRequest>,
            },
            /// TOOL-022 P2: Enter on the exec-stdin prompt. The key handler
            /// already read + cleared the SHARED composer input; this carries
            /// the captured text. App::dispatch routes through
            /// `handle_exec_stdin_submit`, which fires
            /// `backend.write_exec_stdin(agent_session, exec_session, text)`
            /// fire-and-forget (slot cleared on success; kept + logged on
            /// failure — NEVER a scrollback notice).
            ExecStdinSubmit {
                agent_session: codelet_rpc_types::SessionId,
                exec_session: String,
                text: String,
            },
            /// TOOL-022 P2: Esc on the exec-stdin prompt — dismiss the
            /// overlay only. The session keeps running; NOTHING is sent,
            /// cancelled, or killed. App::dispatch routes through
            /// `handle_exec_stdin_dismissed` which clears the slot.
            ExecStdinDismissed {
                agent_session: codelet_rpc_types::SessionId,
            },

        }
    };
}
