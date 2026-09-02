# BUG-171: Exec-stdin TUI overlay never appears — pull probe has no push trigger

**Status:** findings (research for the fix)
**Symptom:** When a Bash/unified_exec command goes interactive (e.g.
`printf 'Please enter some input: '; read -r user_input`), the TUI shows the
command prompt text in scrollback, but the magenta ⌨ exec-stdin composer
overlay (TOOL-022 P2) never appears, so the user cannot type into the
running command's stdin from the composer.

## What works (verified)

The data path is complete:

1. `UnifiedExecTool::handle_run` spawns the per-exec-session quiet detector
   with the agent session id — `rust/tools/src/unified_exec/tool.rs:263`.
2. The detector fires after ≥3s quiet (`EXEC_STDIN_QUIET_THRESHOLD_SECS`)
   while the child is alive — `rust/tools/src/unified_exec/exec_stdin.rs:130`.
3. Fire → `emit_exec_stdin_request(agent_session_id, request)` → the
   agent-loop callback registered at
   `rust/agent-loop/src/agent_loop.rs:685-693` (and
   `rust/napi/src/agent_loop.rs:720`) →
   `BackgroundSession::set_exec_stdin_request(Some(request))` stores it in
   the per-session `RwLock` slot (`rust/sessions/src/background_session.rs:1215`).

So at the moment of the reported symptom, the `ExecStdinRequest` **is**
sitting on the `BackgroundSession`. `get_exec_stdin_request` (the wire
getter) would return it on demand. The failure is entirely in how the TUI
*learns* a request exists.

## The bug: pull-based probe, no push trigger

The TUI populates its composer slot (`AgentViewStore.exec_stdin_by_session`)
from exactly two probe sites:

| # | Trigger | Site | Fires while session is Running & focused? |
|---|---------|------|-------------------------------------------|
| 1 | Session focus switch | `probe_exec_stdin_for` — `rust/fspec-tui/src/app/dispatch_session_cycle.rs:155` | NO — only on Shift+Left/Right |
| 2 | `SessionStateChange { state: Paused }` chunk | `handle_stream_chunk_state_updates` (`dispatch_stream_chunks.rs:58-62`) → `handle_pause_chunk` (`dispatch_pause_hitl.rs:52-139`) probes `get_exec_stdin_request` in parallel with `get_hitl_request` | NO — exec-stdin performs **no status flip** (spec scenario: "no Paused chunk was emitted for the agent session", `exec-stdin-prompt.feature:172`) |

Both triggers are inert in the reported scenario: the user is watching the
focused session, which stays `Running` (the screenshot shows
`auto-continue (0/300)` / `Thinking…`), so no `Paused` chunk is emitted and
no focus switch happens. The stored request is never probed → the slot stays
empty → the overlay never renders.

The commit's tests (exec_stdin_prompt_p2 in tools/sessions/tui) pass because
they dispatch `Action::ExecStdinPromptFetched` directly or script the mock
backend — none exercise the live end-to-end path
"detector fires while the session stays Running → overlay appears".

## Why HITL works (the pattern to mirror)

The HITL handler flips status synchronously (`agent_loop.rs:666` →
`set_status(Paused)`), and `BackgroundSession::set_status` **pushes** a
`StreamChunk::session_state_change` into the chunks broadcast
(`background_session.rs:904-927`). The TUI's chunk subscriber forwards it as
`ChunkReceived` → `handle_stream_chunk_state_updates` → probe →
`HitlPromptFetched` → slot populated → composer prompt renders.

HITL is **push**. Exec-stdin was built as **pull** with no push event, and
the "no status flip" decision (correct — the agent keeps streaming) silently
removed the only trigger that would have fired while the user watches.

## The fix

Add a push `StreamChunk` pair, emitted from
`BackgroundSession::set_exec_stdin_request`, exactly mirroring the
`set_status` → `session_state_change` flow:

1. **rpc-types**: two new state-only `StreamChunk` variants
   (`ExecStdinRequest { request }` and `ExecStdinRequestCleared`), added to
   the state-only match arms in the TUI scrollback recorder
   (`session_context.rs:144-156`) and the NAPI/agent-loop JSON mappers.
2. **sessions**: `set_exec_stdin_request(Some)` emits the request chunk;
   every `set_exec_stdin_request(None)` (exec session exited — alive-check in
   `get_exec_stdin_request`/`set_exec_stdin_request`, `write_exec_stdin`
   success, or reaper race) emits the cleared chunk. Dedupe: emit only on
   actual slot transitions (Some→Some with different ts_ms is an update,
   still emit; None→None does not).
3. **TUI**: `handle_stream_chunk_state_updates` branches on the two variants
   — request → dispatch `ExecStdinPromptFetched` (reusing the existing
   reducer + precedence chain HITL > exec-stdin > pause > composer);
   cleared → `ExecStdinDismissed`. The two existing probe sites (focus
   switch, Paused probe) remain as belt-and-braces.
4. **Transport**: the chunks broadcast already carries `StreamChunk`
   end-to-end (embedded + websocket + NAPI), so no new transport methods are
   needed — the variants ride the existing `chunks_rx` subscriber.

## Out of scope

- The LLM-side P1 steering (quiet_seconds + fixed line) — unaffected.
- The detector's 3s threshold / 30s cooldown — unchanged.
- The write path (`write_exec_stdin`) — unchanged.
- Rendering / keys / precedence — already built and tested (TOOL-022 P2).
