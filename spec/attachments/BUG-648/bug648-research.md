# BUG-648 research: attaching to a running session does not rebuild its live state

Date 2026-11-08. Paths are under rust/. Line numbers are at HEAD 97b32e5bd.

## 1. How the client learns a session's state today

Two channels feed the TUI, and they behave differently.

- **Transcript content** (Text, Thinking, UserInput, ToolCall, ToolResult, ToolProgress, Done, Error, Interrupted*, ResponseAttempt*, UserNotification, IncomingMessage) is never applied from the live chunk. The live chunk only wakes a `get_transcript_page` fetch (`fspec-tui/src/app/dispatch_transcript_live.rs:37-68`). The page cursor `{generation, sequence, byte_offset}` gives an exact, gapless, duplicate-free join, so the in-flight turn and a running tool's streamed output already come back on every attach (`dispatch_transcript.rs:37-84`, `:177-260`).
- **Live state** (SessionStateChange, TokenUpdate, ContextFillUpdate, ContextSnapshot, CommittedCompaction, CompactionProgressUpdate, CompactionComplete, ExecStdinRequest/Cleared, ContinueStateUpdate, SupervisorPendingInjection, ProviderWaiting, InteractionRequestChanged) is applied only from the live chunk (`Action::ChunkReceived` → `handle_chunk_received`, `dispatch_transcript_live.rs:15-34` and `dispatch_stream_chunks.rs`). When the same chunk comes back in a transcript page, it is dropped (`dispatch_transcript.rs:233-239`, filter `is_transcript_content`).

So every attach rebuilds the content and loses the state. The daemon already journals almost all of the state chunks, in order, under one lock, with the content (`sessions/src/background_session/output.rs:44-95`: `committed_context` lock held across counter, journal push and broadcast). The client pages through them on every attach and discards them.

## 2. The attach paths, and what each one fails to restore

| Path | Code | What it does | What it loses |
|---|---|---|---|
| /resume (live session) | `dispatch_attach.rs:25-54`, `dispatch_transcript_recovery.rs:36-68` | focus/append the tab; chrome, draft, supervisors, debug; `resume_session` then pages the transcript | status, pending question/approval/human action, exec-stdin, compaction progress, tokens, context fill, provider waiting, continue counters (all state chunks) |
| startup restore | `session_restore.rs:45-122` → `ReattachSession` (`dispatch_attach.rs:56-60`); bootstrap `replay_session_snapshot` (`dispatch_focus.rs:48-59`) | same wiring without `resume_session`; snapshot tabs open unfocused, the 1 s poll pages them | same; the snapshot's `SessionInfo.status` is ignored |
| daemon switch | `dispatch_daemons.rs:108-138` | replaces `AgentViewStore` with a default, clears transcript sync, respawns subscribers, replays the new daemon's session snapshot | everything except content and work-unit focus |
| workspace switch | `dispatch_workspaces.rs:88-116`, `store/agent_view/workspace_state.rs:40-52` | a display filter over the open tabs; attaches nothing | shows the snapshot-opened tabs of that workspace, which never had their state |
| mux pane | `dispatch_mux.rs:85-97, 166-185`, `views/agent/pane_render.rs:174` | a window over the open tabs; renders store maps | same as the tab it shows |
| WebSocket reconnect | `transport/websocket/reconnect.rs:19-139`, `dispatch_reconnect.rs:136-172` | respawns subscribers, reloads, `spawn_session_restore(focused)`; tabs and cursors survive | every state chunk sent while disconnected (the cursor catches the content up, the state chunks in those pages are dropped) |
| broadcast lag | `app/bootstrap.rs:116-136`; daemon WS fan-out `rpc-server/src/server/fanouts.rs:61-65` | `Lagged` sends `Transcript::Poll` | same as reconnect: the content is re-paged, the state is lost |

Work-unit focus is the one item restored today: `get_work_unit_focus` on attach (`dispatch_focus.rs:77-90`), the snapshot's `work_unit_focus`, and a revisioned channel that refreshes on every subscriber spawn (`bootstrap_focus.rs:20-36`; daemon `activity_focus.rs:78-107`, atomic under the focus mutex).

## 3. The live-state items

| Item | Daemon store | Journaled? | Snapshot RPC | Client store | Restored on attach today |
|---|---|---|---|---|---|
| Running/Compacting/Paused status and spinner ("Thinking..." is Running, `views/agent/animation.rs:35-50`) | `status: AtomicU8` | yes, `SessionStateChange` | `get_session_status` (not on the `FspecBackend` trait) | `session_status_by_session` | no |
| tool call in progress and its output | none (journal only) | yes | none | scrollback / in-flight | **yes** (pages) |
| pending question / approval / human action (AMGR-020) | `hitl_request`, `pause_state` interaction slots | the `Paused` status is; `InteractionRequestChanged` is **not** (`output.rs:47-55`) | `get_pause_state`, `get_hitl_request` (with `RequestRoute`) | `pause_state/pause_route/hitl_prompt_by_session`; AMGR-022's `request_chain` (in progress) | no: the fetch is triggered only by a live `Paused` chunk |
| exec-stdin prompt | `exec_stdin_request` | yes | `get_exec_stdin_request` (may clear a dead request) | `exec_stdin_by_session` | only on tab switch / a Paused probe |
| compaction progress | `compaction_progress` (emit under its write lock) | yes | `get_compaction_progress` (never called by the TUI) | `compaction_progress_by_session` | no |
| tokens and context fill | `cached_*_tokens` atomics; context fill has no field | yes (`TokenUpdate`, `ContextFillUpdate`, `ContextSnapshot`, `CommittedCompaction`) | `get_session_tokens` (input/output only, never called) | `token_state_by_session` | no |
| work-unit focus | `work_unit_focus` with revision | own channel | `get_work_unit_focus` | `work_unit_focus_by_session` | **yes** |
| provider waiting | presentation only | **no** (`output.rs:56-62`) | none | `SessionContext.provider_waiting` | no |
| continue/goal live counters | atomics + chunk | yes (`ContinueStateUpdate`) | `get_continue_state` returns `(enabled, budget)` only | `continue_live_by_session` | partly |
| supervisor pending injections | chunk only | yes | none | `supervisor_pending_count_by_session` | no |
| queued inputs after an interrupt | input inbox | the `Interrupted*` chunk is | none | `busy_inputs` | no (applied live only, `dispatch_transcript_live.rs:9`) |

## 4. Ordering and atomicity on the daemon

- Every chunk through `handle_output` is journaled and broadcast under `committed_context` (`output.rs:63-95`), so the journal order is the broadcast order. `get_transcript_page` reads the journal under `output_buffer.read()`.
- Two kinds skip the journal by explicit special cases: `InteractionRequestChanged` (AMGR-020 comment: "live state, not transcript") and `ProviderWaiting`. Three are sent straight on `chunks_tx`, never through `handle_output`: `DebugStateChange`, `IsolationStateChange`, `FooterStateUpdate` (these have their own attach reads or are workspace-wide). Block notifications (`session_tool_callbacks.rs:162-175`) send a `UserNotification` straight on `chunks_tx`: the TUI only treats that kind as a page wake, so these notices are never shown (pre-existing, out of scope).
- `set_status` swaps the atomic, then sends `status_changes_tx`, then journals `SessionStateChange`, with no lock across the three (`output.rs:8-42`). Two concurrent transitions can be journaled in the opposite order of the atomic.
- `set_exec_stdin_request` writes its slot, releases the lock, then journals (`exec_stdin.rs:22-40`). A concurrent set and clear can be journaled in the opposite order of the slot.
- `set_compaction_progress` already emits under its write lock; focus already broadcasts under its mutex.
- Interaction slots announce after the slot lock is released, but a reply needs the request id, which a holder learns only from the Pending event or the slot, so Pending is always journaled before its Answered. Each chain member gets its own copy through its own `handle_output` (`interactions.rs:158-172`).
- No chunk carries a journal sequence on the live broadcast. The broadcast is manager-wide (256 slots, shared by all sessions). WS lag drops chunks with only a counter.
- The persisted archive (installed only when the journal is unseeded, i.e. on a cold restore) holds content only (`replay_transcript.rs:6-57`). Resuming a live session never touches its journal (`handle_impl_1.rs:33-95`). A cold-restored session starts idle with no state chunks, which is its true state.

## 5. Design: the journal is the session's event log; the client's state is its fold

One mechanism, used by every attach path and by a client that never detached:

1. **The daemon journals every per-session state event.** `handle_output` journals `InteractionRequestChanged` and `ProviderWaiting` like every other chunk (the two special cases go). One shared classification, `StreamChunk::is_journaled()` in rpc-types, says which kinds go through the journal; `handle_output` uses it, so a kind is either always journaled or never. `set_status` holds a status-order lock across the swap, the push and the journal write, and `set_exec_stdin_request` journals under its slot lock, so the journal order is the order of the daemon's own state.
2. **The client applies journaled chunks only from transcript pages, in sequence order.** A live journaled chunk is a wake (as content already is). The page path applies each chunk's state (status, tokens, context fill, compaction progress and completion, exec-stdin, continue counters, supervisor injections, provider waiting, the interrupt queue, interaction requests) through the same state handler the live path uses today. Kinds the daemon never journals (debug, isolation, footer) are applied live as now.
3. **One-shot effects stay live-only.** An fspec command request runs once, when it arrives live, never from a page. The error dialog pops for a live Error chunk only (its text is in the scrollback either way). The pause/question read (`get_pause_state` + `get_hitl_request` + `get_exec_stdin_request`) is triggered by the folded `Paused` status, so attaching to a paused session reads the pending request with its route; a read answers with the daemon's current request, so a replayed earlier pause never mounts a stale prompt.

Why this and not an atomic snapshot RPC plus cursor:
- The cursor already exists and is exact; a snapshot would need sequence numbers on the live broadcast (a wire change to every transport and the chunk subscriber) to join without gaps or duplicates.
- A client that never detached and one that just attached run the same code on the same event sequence, so their state is equal by construction, on both transports (pages are an ordinary RPC).
- It also fixes the reconnect gap and the broadcast-lag loss: missed state events are in the pages the cursor fetches next.
- No new RPC and no new type. The wire-visible change is behavioural: transcript pages and `get_buffered_output` now also contain `InteractionRequestChanged` and `ProviderWaiting` chunks (both variants already exist on the wire).

Where the push channels stay: `status_changes_rx` still updates status (the board, and tabs whose pages lag by one round trip; both converge to the journal's last status, which the status-order lock makes the daemon's status). The work-unit focus keeps its revisioned channel.

## 6. Coordination

- AMGR-022 (in testing, uncommitted `app/dispatch_request_chain.rs`, `store/agent_view/request_chain.rs`, an `InteractionRequestChanged` arm in `dispatch_stream_chunks.rs`) folds `InteractionRequestChanged` from live chunks. With this design the event reaches that same arm from pages instead, once per chain member's journal; the fold must be idempotent per request id, and since copies from different members' journals can be applied in either order, an Answered/Cancelled request id must stay terminal (a later Pending copy of the same id must not revive it). BUG-648 does not edit AMGR-022's files.
- Wire: behavioural only (pages carry two more chunk kinds). The fspec-paseo plugin decodes `StreamChunk` strictly; both variants already exist, so it must only tolerate them in pages.

## 7. Open product question

- The compaction reduction badge (`[X%: COMPACTED Y%]`) auto-hides 10 s after the CompactionComplete event is applied (RPC-417). Pages carry no event time, so on attach a replayed CompactionComplete would show the badge again for 10 s. Either (A) a CompactionComplete applied while catching up to the attach-time snapshot boundary sets no badge (the notice line still lands in the scrollback), or (B) it shows the badge for 10 s from the attach. Recommendation: A.

## 8. Out-of-scope findings (need cards)

- Block notifications bypass the journal (`sessions/src/session_tool_callbacks.rs:162-175`) and are never shown in the TUI, live or on attach.
- `get_buffered_output` returns the oldest `limit` chunks, not the latest.
- `get_interaction_snapshot` reads the pause and question slots in two separate calls (not atomic).
- `IsolationStateChange` / `DebugStateChange` / `FooterStateUpdate` bypass the journal; debug is re-read on attach, isolation and footer are not checked here.
