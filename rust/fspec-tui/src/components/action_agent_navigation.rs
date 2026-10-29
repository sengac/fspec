macro_rules! compose_action_agent_navigation {
    ($($collected:tt)*) => {
        compose_action_session_context! {
            $($collected)*
            /// RPC-018: per-session ModelInfo arrived (typically from a spawned
            /// `backend.get_model_info(session)` task fired in response to
            /// `Action::SessionCreated`). App::dispatch writes the payload
            /// into `AgentViewStore.model_info_by_session[session]`.
            ModelInfoLoaded(codelet_rpc_types::SessionId, codelet_rpc_types::ModelInfo),
            ModelSelectionLoaded(
                codelet_rpc_types::SessionId,
                u64,
                Result<(Option<String>, codelet_rpc_types::ModelInfo), String>,
            ),
            /// RPC-018: per-session ThinkingLevel arrived. Sibling of
            /// `ModelInfoLoaded` — fired by the same `Action::SessionCreated`
            /// dispatch arm.
            ThinkingLevelLoaded(
                codelet_rpc_types::SessionId,
                codelet_rpc_types::ThinkingLevel,
            ),
            /// RPC-018: workspace snapshot arrived (typically from
            /// `App::bootstrap` firing `backend.get_workspace_info()`).
            /// App::dispatch writes the payload into `AgentViewStore.workspace`.
            WorkspaceInfoLoaded(codelet_rpc_types::WorkspaceInfo),
            /// RPC-019: AgentView emits this when the user presses Shift+Up
            /// inside the MultiLineInput. RPC-021 will wire App::dispatch
            /// routing to walk the command history backward.
            HistoryPrev,
            /// RPC-019: AgentView emits this when the user presses Shift+Down.
            /// Sibling of `HistoryPrev` — walks forward through command history.
            HistoryNext,
            /// RPC-019: AgentView emits this when the user presses Shift+Left.
            /// RPC-021 will route through App::dispatch to cycle to the
            /// previous session in the AgentViewStore's session list.
            SessionPrev,
            /// RPC-019: AgentView emits this when the user presses Shift+Right.
            /// Sibling of `SessionPrev` — cycles forward through sessions.
            SessionNext,
            /// RPC-020: AgentView emits this when the user picks a command
            /// from the slash palette (Enter on a highlighted row). App::dispatch
            /// branches per `SlashCommandAction` variant — Help pushes the
            /// HelpDialog onto the Compositor, Clear resets the scrollback,
            /// Quit sets `should_quit`, all others surface a `[notice]`
            /// scrollback line until the future RPC card lands them.
            SlashCommandSelected(crate::views::agent::slash_commands::SlashCommandAction),
            /// RPC-020: AgentView emits this after the user types into the
            /// `@<filter>` token. App::dispatch spawns a tokio task calling
            /// `backend.search_files(prefix, 20)` and emits
            /// `Action::FileSearchResults(matches)` on success.
            SearchFiles(String),
            /// RPC-020: backend search returned this list of paths — App::dispatch
            /// forwards into AgentView's file_popup via `set_matches`.
            FileSearchResults(Vec<String>),
            /// RPC-024: AgentView emits this when the user presses PageUp.
            /// App::dispatch routes the scroll into the currently-focused
            /// SessionContext's ScrollbackList so cross-session scroll state
            /// stays correctly per-session.
            ScrollbackPageUp,
            /// RPC-024: sibling of `ScrollbackPageUp` — emitted on PageDown /
            /// End. App::dispatch advances the focused SessionContext's
            /// scrollback offset, snapping back to stick-mode at the tail.
            ScrollbackPageDown,
            /// RPC-094: emitted by `views/agent/dispatch.rs` when the user
            /// presses the Up arrow AND `MultiLineInput::handle_event` returns
            /// `Ignored` because the cursor is on the first visual line. The
            /// App routes this into the focused SessionContext, dropping
            /// stick-to-bottom and decrementing offset by exactly 1.
            ScrollbackLineUp,
            /// RPC-094: sibling of `ScrollbackLineUp` — emitted by the Down
            /// arrow at the last visual line of the input buffer.
            ScrollbackLineDown,
            /// RPC-094: emitted on Home when the input does not consume it
            /// (empty buffer or cursor already at position 0). Jumps the
            /// focused SessionContext's scrollback offset to 0 and drops
            /// stick-to-bottom.
            ScrollbackHome,
            /// RPC-094: emitted by `views/agent/mouse_dispatch.rs` when a
            /// `MouseEventKind::ScrollUp` hits inside the scrollback rect.
            /// The `u32` carries the velocity multiplier (1..=5) sampled from
            /// the AgentView's `WheelVelocity` primitive.
            ScrollbackMouseWheelUp(u32),
            /// RPC-094: sibling of `ScrollbackMouseWheelUp` — emitted on
            /// `MouseEventKind::ScrollDown` inside the scrollback rect.
            ScrollbackMouseWheelDown(u32),
            /// TUI-102: emitted by `views/agent/mouse_dispatch.rs` when a scrollbar
            /// click/drag on the scrollback gutter computes a new scroll offset.
            /// App::dispatch routes this into the focused SessionContext's
            /// ScrollbackList, setting `offset` and exiting `stick_to_bottom`.
            ScrollbackJumpToOffset(usize),
            /// RPC-025: emitted by the spawned `backend.persistence_get_history`
            /// task fired in response to the user's first `Action::HistoryPrev`
            /// on a session. Carries the SessionId and the newest-first list of
            /// past submitted inputs. App::dispatch caches the snapshot, snaps
            /// `recall_index` to `Some(0)`, and replaces the MultiLineInput
            /// value with `snapshot[0]`. Dropped (no-op) when the snapshot is
            /// empty so HistoryPrev with no history leaves state untouched.
            HistorySnapshotLoaded(codelet_rpc_types::SessionId, Vec<String>),
            /// RPC-026: open the /resume session picker mode view. App::dispatch
            /// installs AgentView.resume_view AND spawns `backend.list_sessions()`.
            OpenResumeView,
            /// RPC-026: open the /search history palette mode view.
            /// App::dispatch installs AgentView.search_view with an empty query
            /// (no backend call yet — that fires on the first SearchHistory).
            OpenSearchView,
            /// RPC-026: drop the resume view without changing focus. Emitted by
            /// the resume view's Esc handler (Dismiss outcome).
            CloseResumeView,
            /// RPC-026: drop the search view without inserting anything.
            /// Emitted by the search view's Esc handler (Dismiss outcome).
            CloseSearchView,
            /// RPC-026: a spawned `backend.list_sessions()` task resolved.
            /// App::dispatch folds the result into the open resume_view via
            /// `set_sessions`. No-op when resume_view is None.
            SessionListLoaded(Vec<codelet_rpc_types::SessionInfo>),
            /// RPC-026: emitted by the resume view on Enter. App::dispatch
            /// either moves `current_session_index` to the matching slot in
            /// AgentViewStore.open_sessions OR appends a fresh
            /// SessionContext::new(id) if the session is not yet open. Also
            /// publishes to active_session_tx and runs refresh_session_chrome.
            AttachToSession(codelet_rpc_types::SessionId),
            /// BUG-206: auto-restore re-attach (bootstrap + reconnect).
            /// Emitted ONLY for a session just verified LIVE in the daemon's
            /// in-memory session map (`list_live_sessions`) — the memory-only
            /// attach: re-open the tab when absent (fresh-start restore of the
            /// last-viewed session) or focus the existing tab (transient WS
            /// drop), hydrate chrome, and catch up from the in-memory
            /// `get_buffered_output`. NEVER runs the RPC-049 disk
            /// `resume_session` round-trip (that is the manual /resume path
            /// via `AttachToSession`) and NEVER resurrects a closed session.
            ReattachSession(codelet_rpc_types::SessionId),
            /// RPC-026: emitted by the search view as the user types.
            /// App::dispatch spawns `backend.persistence_search_history(q)` and
            /// dispatches Action::HistorySearchResults on success.
            SearchHistory(String),
            /// RPC-026: a spawned `backend.persistence_search_history` task
            /// resolved. App::dispatch folds the result into the open
            /// search_view via `set_matches`. No-op when search_view is None.
            ///
            /// RPC-064: widened to a struct variant carrying both the originating
            /// `query` and the `matches`. The dispatcher folds the matches ONLY
            /// when `query` still matches the live `search_view.query()` so
            /// stale responses for older queries are silently discarded.
            HistorySearchResults {
                query: String,
                matches: Vec<codelet_rpc_types::HistoryMatch>,
            },
            /// RPC-026: emitted by the search view on Enter with the
            /// highlighted match's text. App::dispatch replaces the
            /// MultiLineInput value with `text` AND drops the search_view.
            /// Does NOT auto-submit — the user may edit before pressing Enter.
            InsertIntoInput(String),
            /// RPC-026: emitted by the resume view when the user presses D on
            /// a row. App::dispatch surfaces a ConfirmDialog inside the
            /// resume_view — no backend call fires yet.
            RequestDeleteSession(codelet_rpc_types::SessionId),
            /// RPC-026: emitted by the ConfirmDialog when the user activates
            /// the primary "Delete" button. App::dispatch spawns
            /// `backend.persistence_delete_session(id)` and follows up with a
            /// fresh `backend.list_sessions()` → Action::SessionListLoaded
            /// so the resume view repaints without the deleted session.
            ConfirmDeleteSession(codelet_rpc_types::SessionId),
        }
    };
}
