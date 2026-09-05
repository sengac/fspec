# TOOL-022 Research: Surfacing Exec-Session Stdin Prompts in the TUI (HITL Integration)

**Date:** 2026-09-02
**Scope:** Research only — no code changes. This document records the full trace of the
existing HITL (question tool) machinery, the existing exec/stdin machinery, prior
plans/stubs, and the recommended design for rendering an interactive process's
stdin prompt **in the same place the `request_user_input` (HITL) question prompt
renders: the composer input area**.

Companion research: the vtcode-vs-fspec interactive-bash gap analysis (session
`2bc9d511`, 2026-09-01; gaps G1–G12), which identified **G11: "TUI cannot feed
input to a waiting session."** This document is that gap, worked against the
existing HITL architecture.

---

## 1. TL;DR

- fspec already has three mature "tool asks the user, UI renders inline in the
  composer" mechanisms, all sharing the same pattern:
  **block the tool on a per-session channel → store request state on the
  `BackgroundSession` → flip session status → TUI probes via backend getters on a
  `Paused` chunk → renders an inline prompt in the input area → user responds via
  a backend send → channel unblocks the tool.**
  1. **Tool pause** (`tool_pause.rs`, `PauseKind::{Continue, Confirm, Triple}`) — used by the blocklist prompt action (BLOCK-007).
  2. **HITL** (`request_user_input` tool + `HitlHandler`) — the "question tool" (BUG-116/117/118, RPC-410/411).
  3. *(planned)* **Exec-stdin prompt** — new; this card.
- The exec machinery (TOOL-016 `unified_exec` + BUG-114/115 Codex facades) can
  **already write to a live session's stdin** (`write` / `write_stdin` actions) —
  but **nothing ever tells the TUI or the LLM that a process is waiting for
  input**, and the user has no way to type into a waiting session from the TUI.
- Recommended design (§7): a new `ExecStdinRequest` on `BackgroundSession` that
  reuses the HITL render slot (same place, same shared freeform input), triggered
  by the unified_exec engine when a live session stops producing output and its
  last output line looks like a prompt. Unlike HITL, it does **not** pause the
  agent loop — it is a convenience overlay that forwards typed text straight to
  the session's stdin; Esc dismisses without cancelling anything.

---

## 2. Problem statement

When an LLM-run command needs interactive input (password prompt, `git commit`
message editor, install wizard, REPL, `ssh` host-key confirmation, `less`), the
user is stuck:

| Surface | What happens today |
|---|---|
| LLM (Codex) | Can `poll`/`write_stdin` in a loop, but no signal that input is *needed* — it polls on a guess (vtcode ships pre-filled `next_wait_args`/`next_continue_args` hints; fspec has none). |
| LLM (Claude/OpenAI/Copilot/custom) | Only the one-shot `Bash` tool exists: blocks on `child.wait()` forever; stdin is *inherited* from the fspec process (raw-mode TUI terminal in TUI mode, /dev/null in RPC mode). The only escape is ESC (SIGKILL). |
| TUI user | No prompt, no status, no way to type into the child. Only "ESC to interrupt". |

Goal: when a managed exec session is waiting for stdin, show a prompt **exactly
where the HITL question prompt shows** (inline in the composer input area), and
let the user's typing flow into the session's stdin.

---

## 3. Existing HITL architecture (the "question tool")

The full pipeline, traced end-to-end (all line numbers as of 2026-09-02):

### 3.1 Tool layer — `codelet-tools`

- `rust/tools/src/request_user_input.rs` (TOOL-017):
  - Wire-independent types: `HitlRequest { questions: Vec<HitlQuestion> }`,
    `HitlQuestion { id, header, question, options }`, `HitlResponse`
    (`Answered { answers }` / `Cancelled`).
  - **Per-session handler registry**: `HITL_HANDLERS: RwLock<HashMap<Uuid, HitlHandler>>`
    (`set_hitl_handler` / `has_hitl_handler` / `clear_all_hitl_handlers`).
  - `RequestUserInputTool` (rig `Tool`): validates (1–3 questions, snake_case id,
    ≤12-char header, 2–3 options), then `execute_hitl(session_id, request)` →
    dispatches to the registered handler **synchronously** (the handler blocks
    until the TUI responds).
  - Mode-gating: no handler registered → error
    `"request_user_input is unavailable in the current session mode"` (headless).
- Registered for every provider: `providers/src/{claude.rs:554, openai.rs:467,
  copilot/rig_agent.rs:104, gemini.rs:213, zai.rs:283, custom/custom_provider.rs:272}`;
  Codex gets the facade wrapper `CodexRequestUserInputFacade` (BUG-116,
  `facade/codex.rs:750+`) which maps Codex's `request_user_input` schema onto it.

### 3.2 Handler wiring (the "pause pattern") — agent loop

Both agent loops register the same closure (BUG-117 fix):

- `rust/agent-loop/src/agent_loop.rs:660-678` (CLI/RPC embedded loop)
- `rust/napi/src/agent_loop.rs:690-708` (NAPI loop)

```
hitl_handler(session_id, request):
    session.set_hitl_request(Some(request))        # store on BackgroundSession
    session.set_status(SessionStatus::Paused)      # fires SessionStateChange{Paused} chunk
    response = session.wait_for_hitl_response()    # BLOCKS (std mpsc recv wrapped in
                                                  #   block_in_place — RPC-409)
    session.set_hitl_request(None)
    session.set_status(SessionStatus::Running)     # fires SessionStateChange{Running}
    Ok(response)
```

Registered at agent-run start; cleared on loop end (`set_hitl_handler(id, None)`,
agent_loop.rs:1408/1471).

### 3.3 Session state — `codelet-sessions`

- `rust/sessions/src/background_session.rs`:
  - `hitl_request: RwLock<Option<HitlRequest>>` (line ~388)
  - `hitl_response_tx: std::sync::mpsc::Sender<HitlResponse>` +
    `hitl_response_rx: Mutex<Receiver>` (lines ~381-384)
  - `wait_for_hitl_response()` (1146-1158): blocking `rx.recv()` via
    `blocking_recv_compat` (RPC-409 `block_in_place` guard); cancelled-channel
    fallback returns `Cancelled`.
  - `send_hitl_response()` (1164-1168), `set_hitl_request()` (1174+),
    `get_hitl_request()` (1186-1187) — polled by the NAPI getter.
- Wire mapping: `rust/sessions/src/hitl_mapping.rs` (RPC-410) — pure pass-through
  between rpc-types `HitlRequest`/`HitlResponse` and the internal types; NO
  inference.

### 3.4 Wire / transport

- `rust/rpc-types/src/lib.rs:1202-1258`: `HitlOption`, `HitlQuestion`,
  `HitlRequest`, `HitlAnswer`, `HitlResponse` (NAPI + serde objects).
- `rust/rpc/src/lib.rs` + `rust/sessions/src/handle_impl.rs:938-977`:
  `get_hitl_request` (wire getter) and `send_hitl_response` (wire response) on the
  `SessionManagerHandle` — used by **both** the embedded (in-process) and
  WebSocket transports.
- NAPI (TS host): `rust/napi/src/session_bindings.rs:1600`
  (`session_get_hitl_request`) and `:1725` (`session_send_hitl_response`).

### 3.5 TUI — chunk-driven, store-slot, rendered in the composer

- **Trigger**: `StreamChunk::SessionStateChange { state: Paused }` →
  `handle_stream_chunk_state_updates` → `Action::PauseChunkReceived` →
  `App::handle_pause_chunk` (`rust/fspec-tui/src/app/dispatch_pause_hitl.rs:52-110`):
  spawns `tokio::join!(backend.get_pause_state, backend.get_hitl_request)`;
  **HITL wins on a tie** (rule from `pause-and-hitl-dialogs.feature`).
- **Store**: `rust/fspec-tui/src/store/agent_view/hitl_state.rs` —
  `AgentViewStore.hitl_prompt_by_session: HashMap<SessionId, HitlPromptState>`;
  `HitlPromptState` is a faithful port of the TS `useHitlInput` machine
  (question_index, selected_option, answers, other_active, show_empty_hint).
- **Render location (THE answer to "same place")**:
  `rust/fspec-tui/src/views/agent/input_area.rs::paint_input_area` —
  precedence order: **HITL slot → pause slot → spinner/composer**. The HITL
  prompt paints into the same padded input-area rect the composer uses; in
  freeform/Other mode it renders the **SHARED composer `MultiLineInput`**
  (`hitl_prompt.rs::render_hitl_prompt`, placeholder "Type your answer...").
  Height comes from `hitl_prompt::prompt_height` fed to the RPC-405 auto-grow
  layout (`input_area_height`).
- **Keys**: `rust/fspec-tui/src/views/agent/hitl_keys.rs` — consulted in
  `views/agent/dispatch.rs` BEFORE the pause-prompt keys; options mode consumes
  every key; freeform mode routes typing/paste into the shared input, Enter
  captures (`HitlAnswerCaptured`), Esc cancels (`HitlCancelled`).
- **Submit/cancel**: `Action::HitlSubmitted`/`HitlCancelled` →
  `handle_hitl_submitted` (dispatch_pause_hitl.rs:224-243) →
  `backend.send_hitl_response` (fire-and-forget) → slot cleared.
- **Clearing**: `Running`/`Idle` chunk → `handle_pause_cleared` clears BOTH the
  pause and HITL slots (dispatch_pause_hitl.rs:115-119).

**Key invariants to preserve:**
1. No code path dismisses the HITL UI without sending a response (stranding
   guard, RPC-411).
2. Errors from backend getters/senders are silently logged (tracing), never
   scrollback notices.
3. The composer draft + cursor survive the round-trip (the prompt paints over,
   never mutates, the shared input's state — except the deliberate freeform
   capture).
4. Per-session isolation (multiple sessions can pause independently).

---

## 4. Sibling mechanism: tool pause (BLOCK-007 precedent)

`rust/tools/src/tool_pause.rs` (PAUSE-001 + BLOCK-007):

- `PauseKind::{Continue, Confirm, Triple}`, `PauseRequest { kind, tool_name,
  message, details }`, `PauseResponse::{Resumed, Approved, Denied, Interrupted,
  AllowOnce, AllowSession}`.
- Per-session `PauseHandler` registry (`SessionRegistry<PauseHandler>`);
  `pause_for_user(session_id, request)` blocks until the TUI answers; no handler
  → `Resumed` immediately.
- **Real mid-execution blocker**: `blocklist/middleware.rs::check_bash_command`
  (160-197) calls `pause_for_user(Triple)` while a tool is executing. This proves
  the established pattern: *a tool may block in the middle of execution and the
  TUI renders an inline prompt in the composer* — the exact class of behavior an
  exec-stdin prompt needs (or a sibling of it, §7).

---

## 5. Current exec/stdin machinery

### 5.1 `unified_exec` (TOOL-016, done)

`rust/tools/src/unified_exec/` — actions `run | write | poll | list | close`:

- `spawning.rs`: pipe mode pipes **all three** stdio (stdin via
  `mpsc::channel::<Vec<u8>>(64)` → dedicated writer task, lines 78, 107-116).
  `spawn_pty_process` is a **documented fallback stub** to pipe mode
  (lines 145-152); `portable-pty` IS already a dependency of the tools crate
  (used by `bridge_pty.rs`) — real PTY was TOOL-016 FIX-1, deferred.
- `tool.rs::poll_session` (346-405): yield-and-resume; yield clamped
  250ms–30s (default 10s; poll min 5s). Reaper race (FIX-15) returns
  `exit_code: -1` when the reaper removed the session first.
- `types.rs::UnifiedExecResult { exit_code, session_id, output,
  wall_time_seconds, sessions, error }` — **no waiting-for-input signal, no
  next-step hints** (vtcode's `next_wait_args`/`next_continue_args` are absent).
- Defects from the gap analysis (session `2bc9d511`): G1 stdin inheritance in
  BashTool, G2 missing `PAGER/GIT_PAGER/NO_COLOR`, G3 `timeout_ms` dropped in
  `facade/wrapper.rs::internal_exec_params_to_json` (1827-1828), G5 reaper race,
  G6 no `wait` action, G7 no response hints, G8 PTY stub, G9 no echo stripping,
  G10 no output spooling, G11 **no TUI input forwarding (this card)**, G12 no
  PTY denial list.

### 5.2 Provider registration

- **Codex only**: `providers/src/codex/mod.rs:382-386` registers
  `exec_command` / `write_stdin` / `shell` facades (`ExecToolFacadeWrapper` →
  `UnifiedExecTool`); `facade/codex.rs:696-731` maps empty `chars` → poll,
  non-empty → write.
- **Claude/OpenAI/Copilot/custom/internal**: one-shot `BashTool` only
  (`providers/src/{claude.rs:536, openai.rs:452, copilot/rig_agent.rs:89,
  custom/internal_dispatch.rs:112}`) — `bash_process.rs::spawn_command`
  (143-172) pipes stdout/stderr, **not** stdin (child inherits fspec's stdin),
  no timeout, blocks until exit or ESC (`bash_abort.rs` flag → SIGKILL).

---

## 6. Prior plans & stubs (answering "did we plan this?")

What exists, in order of concreteness:

1. **The stdin pipe itself is done** — unified_exec pipe sessions carry a live
   `stdin_tx` mpsc sender per session (G11's *backend half* already works for
   the LLM via `write`/`write_stdin`).
2. **TOOL-016 description** explicitly scoped "Interactive stdin writing to live
   sessions (write action)" as LLM-driven; the *user-facing* interactive layer
   was out of scope. TOOL-016's own review (`spec/attachments/TOOL-016/fixes.md`)
   deferred FIX-1 (real PTY) and FIX-11 (non-tty write rejection) — both are
   prerequisites for the full experience (§8, Phase 3).
3. **G11 was identified** in the vtcode gap research (session `2bc9d511`,
   2026-09-01): "no way to type into a running session / waiting session" —
   listed but never turned into a card until now.
4. **vtcode reference**: its TUI only shows a *status shimmer* for
   "waiting for input" / "input required" (`vtcode-ui/.../input.rs:1188-1190`,
   `session/state.rs:341`) — i.e. vtcode does **not** inline-prompt for process
   stdin either; it relies on LLM-driven polling with pre-filled next-step args.
   So the "inline prompt in the composer" design below is a **fspec
   improvement** over vtcode, built on fspec's stronger HITL UX.
5. **No dedicated work unit** exists for a TUI stdin prompt (verified: full scan
   of `spec/work-units.json` titles/descriptions for stdin/interactive-shell/
   input-forwarding terms). The closest cards (TOOL-016/017, BUG-114/115/116/
   117/118, PAUSE-001, BLOCK-007) are all `done`.
6. **Near-miss names, NOT this feature**: `BackgroundSession::pending_input`
   (RPC-052) is the *composer draft* durability channel (user's own next message),
   not process stdin. Don't confuse them.

---

## 7. Recommended design

### 7.1 Core decision: overlay, not pause

HITL pauses the agent (status → `Paused`) because the *answer belongs to the
user and the turn cannot proceed without it*. An exec-stdin prompt is
different: **the LLM can answer it itself** (that's what `write_stdin` is for).
Pausing the agent to hand the prompt to the user would be a behavior regression
for any provider, and a deadlock risk if the LLM and user race for the same
prompt.

**Design:** the exec-stdin prompt is a **non-blocking overlay** in the same
composer slot:

- It does **not** change `SessionStatus` (stays `Running`).
- It does **not** block any tool or the agent loop.
- User's Enter → typed text is written to the session's stdin (new backend
  method), exactly as if the LLM had called `write`. The LLM's next poll sees
  the effect.
- Esc → dismisses the prompt only (session keeps running; no cancel semantics —
  this differs from HITL, and is safe because nothing is blocked).
- Dismissed/answered prompts re-appear if the session goes quiet again and the
  detector fires again (with a per-session cooldown to avoid flicker).

### 7.2 Wire types (rpc-types)

```rust
/// A live exec session that appears to be waiting for stdin input.
pub struct ExecStdinRequest {
    pub exec_session_id: String,   // unified_exec session id (NOT the agent session id)
    pub command: String,           // command display (already stored per ProcessEntry)
    pub hint: Option<String>,      // candidate prompt line detected in output tail
    pub ts_ms: u64,                // when the detector fired (cooldown keying)
}
```

New `StreamChunk` variant (or `ToolProgress` reuse — see §7.4) so the TUI can
react even without a status change; plus two handle methods mirroring HITL:

- `get_exec_stdin_request(session_id) -> Option<ExecStdinRequest>`
- `write_exec_stdin(session_id, exec_session_id, text: String) -> Result<(), String>`
  (appends `\n` when the text doesn't end with one — matches `write` semantics)

### 7.3 Session state (codelet-sessions)

- `BackgroundSession.exec_stdin_request: RwLock<Option<ExecStdinRequest>>`
  (mirror of `hitl_request`; no status flip, no response channel — the write
  path goes through `ProcessStore` directly, not a blocking channel).
- No handler registry needed in tools: the *detector* runs where the session's
  output is already being observed (see §7.4), and calls a session callback.

### 7.4 Detection & trigger — where the signal originates

Options, in order of preference:

- **A. Reader-task detector in `unified_exec` (recommended).** The pipe/PTY
  reader tasks already own the output stream. A small "activity" tracker per
  session (last-output-time + last-line heuristic) runs in the reaper loop
  (already polling every 2s) or a dedicated task: fire when
  `now - last_output > quiet_threshold` (e.g. 3s) AND child alive AND
  `looks_like_prompt(last_output_tail)` (heuristic list below). Fire → set the
  session's stored request (via a per-agent-session callback registered by the
  agent loop, analogous to `set_tool_progress_callback`). Cooldown: re-fire at
  most every ~30s per exec session; never while a prompt is currently shown.
- **B. LLM-driven only (cheap floor, ship first).** No detector: the exec tool
  result itself gains `waiting_for_input: bool` (same quiet-time heuristic,
  evaluated at poll/write time) + `hint`. LLM-facing hint string mirrors
  vtcode: *"The process appears to be waiting for input — send your answer via
  the write action, or the user may type it directly in the TUI."* This makes
  the LLM stop guessing and gives the TUI a signal *without* new chunks: the
  TUI can derive "waiting" from a `ToolProgress`/tool-result event that carries
  the exec session id.
- **C. Full PTY heuristics** (cursor-position escape sequences, `in` read
  patterns) — out of scope until Phase 3 PTY lands.

**Heuristic list for `looks_like_prompt`** (conservative, last non-empty line,
case-insensitive): ends with `:`, `>`, `?`, `) `, `] `; contains `password`,
`passphrase`, `confirm`, `continue`, `y/n`, `yes/no`, `press enter`,
`type `, `enter `, `choice`, `select`; or equals a known REPL prefix
(`> `, `$ `, `# `, `python`, `PS>`). False positives are low-risk (the prompt is
dismissible; the LLM still drives stdin). False negatives are fine (the LLM
path works unchanged).

### 7.5 TUI (codelet-fspec-tui)

- New store slot in `AgentViewStore`: `exec_stdin_by_session:
  HashMap<SessionId, ExecStdinRequest>` (mirrors `hitl_state.rs`).
- New `FspecBackend` methods: `get_exec_stdin_request`,
  `write_exec_stdin` (embedded + websocket + MockBackend).
- Trigger: on a new `ExecStdin` chunk (or derived tool-result event, option B)
  dispatch `Action::ExecStdinPromptFetched` → store the slot. Also clear the
  slot when the exec session exits/closes (derive from a follow-up chunk or
  poll on focus).
- **Render: same place as HITL** — `input_area.rs::paint_input_area` inserts the
  exec-stdin prompt in the precedence chain: `HITL > exec-stdin > pause >
  composer`. Visual contract (deliberately simpler than HITL — it is a
  freeform-only prompt):
  ```
  ⌨ git commit (abc1234) is waiting for input        ← magenta glyph, bold command, dim "is waiting for input"
  <shared MultiLineInput, placeholder "Type to send to the command…">
  (Enter Send | Esc Dismiss)                          ← dim footer
  ```
  Reuses the shared `MultiLineInput` exactly as HITL freeform does (same
  `render_with_prompt` path, same draft-preservation rule, hardware cursor
  visible). Prompt line shows the session's `command` display + the detected
  `hint` when present (dim).
- **Keys** (new `exec_stdin_keys.rs` mirroring `hitl_keys.rs`): Enter →
  `Action::ExecStdinSubmit { agent_session, exec_session, text }` (captures +
  clears the shared input); Esc → `Action::ExecStdinDismissed`; typing/paste →
  shared input. Must be consulted AFTER the HITL keys (HITL still wins) and
  BEFORE the pause keys.
- **Submit**: fire-and-forget `backend.write_exec_stdin(...)`; clear the slot on
  success; on error, keep the slot and log (tracing) — never strand a scrollback
  notice (invariant #2).
- **Clearing on focus loss / session exit / Running-chunk of the exec session**:
  the slot is ephemeral; cleared on `ExecStdin`-none events and when the exec
  session id no longer exists (the TUI can call `get_exec_stdin_request` on
  focus switch; a `None` clears).

### 7.6 Session status: no change

Explicitly: **do not** introduce a new `SessionStatus` variant or reuse
`Paused` — the agent keeps streaming, the tool keeps polling, and ESC
interruption semantics stay intact. The prompt is purely additive UI.

### 7.7 Relationship to the LLM path

Both paths write the same `stdin_tx`. Concurrency is safe (mpsc serializes
writes). When the user types an answer, the LLM's next `poll` returns that
answer's echo/output and proceeds normally — no protocol change, no conflict.
Add the G7-style hint to exec results *regardless* of whether the TUI shows a
prompt, so headless/RPC sessions get the LLM-side benefit (option B ships as
Phase 1 and is independent of the TUI work).

---

## 8. Implementation phases (for follow-up cards)

| Phase | Scope | Depends on | Est. |
|---|---|---|---|
| **P1 — LLM-side signal** (option B) | `UnifiedExecResult.waiting_for_input + hint`; quiet-time heuristic helper (pure fn, proptest-able); G7-style next-step hint text; wire through `ExecToolFacadeWrapper` + Codex facades. No TUI, no new chunks. | none | 3 |
| **P2 — TUI inline prompt (pipe sessions)** | `ExecStdinRequest` wire type; `BackgroundSession.exec_stdin_request`; handle methods `get_exec_stdin_request`/`write_exec_stdin` (embedded + ws + NAPI); reaper-loop detector + per-agent-session callback; TUI slot/store/keys/render (composer slot, shared input); MockBackend + integration tests (inline-prompt scenarios mirroring `inline_hitl_prompt_rpc411.rs`). | P1 | 8 |
| **P3 — PTY + correctness hardening** | Real PTY via `portable-pty` (G8/FIX-1), command-echo stripping (G9), `PAGER/GIT_PAGER/NO_COLOR` env (G2), stdin `piped()` in `BashTool` (G1), PTY denial list (G12), non-tty write rejection (FIX-11), reaper tombstone (G5), `wait` action (G6), `timeout_secs` wiring (G3). | P2 | 8+ |
| **P4 — Main-provider exposure (G4)** | Register unified exec (or exec+write_stdin facades) for claude/openai/copilot/custom; decide BashTool coexistence. | P1-P3 | 5 |

Phase P2 can ship on pipe-mode sessions only (today's reality, since PTY is a
stub) — the prompt fires from the same detector and writes to the same `stdin_tx`.

---

## 9. Risks & open questions

1. **False-positive prompt spam** — a legitimately silent long-running command
   (`sleep`, `tail -f` with quiet periods) would show the prompt. Mitigations:
   quiet threshold ≥3s, per-session 30s cooldown, prompt is trivially dismissible
   (Esc, no side effects), and the heuristic requires a *prompt-shaped* last
   line (not mere silence). **Decision needed:** should "silent + alive" alone
   ever show the prompt (useful for `less`/pagers) or only prompt-shaped lines?
   Recommendation: prompt-shaped only, for P2; "silent" variant behind a config
   flag later.
2. **Two writers** (user + LLM) to one stdin — benign (mpsc ordering), but the
   LLM could be confused if the user answers a question the LLM was about to
   answer. The G7 hints + the prompt's visibility of the answer (echo shows up
   in the next poll) make this self-healing. No locking needed.
3. **Secrets** — password prompts: typed text stays in the TUI process + a
   `write_exec_stdin` call (in-memory, not persisted in scrollback; the tool
   result the LLM sees WILL contain the prompt line but not the secret — the
   echo of a `getpass`-style prompt is nothing). Decision: mask the hint line
   when it contains "password"/"passphrase".
4. **Mux mode** — multiple agent panes: the slot is per-agent-session (like
   HITL), only the focused pane renders it; ghost panes show nothing (consistent
   with `paint_ghost_input_row` today).
5. **WebSocket transport parity** — every new handle method needs ws + embedded
   + MockBackend arms (see `rpc037_cross_transport_parity.rs` as the checklist).
6. **Blocker on P1**: `write_exec_stdin` on an *exited* session must return a
   clean error (not -1-race noise) — G5 tombstone improves this.
7. **Status line vs composer**: vtcode shows a status shimmer; we render in the
   composer per the user's requirement. Confirm the magenta-glyph visual
   contract (⌨ vs ⏸) in the feature file example mapping.

---

## 10. References

- Prior gap research (G1–G12): session `2bc9d511-c1f2-4a1f-8b92-4f71d92d729a`
  (2026-09-01), incl. the vtcode comparison (`/tmp/vtcode`).
- `spec/features/unified-exec-tool.feature` (TOOL-016)
- `spec/attachments/TOOL-016/{fixes.md,unified-exec-reference.md}` — deferred
  FIX-1 (PTY), FIX-9 (timeout), FIX-11 (non-tty write rejection), FIX-15 (reaper
  race).
- `spec/features/{request-user-input-hitl-tool, hitl-handler-wiring,
  hitl-wire-protocol-parity, inline-hitl-prompt, pause-and-hitl-dialogs,
  paused-chunk-delivery-during-blocking-waits,
  integrate-blocklist-prompt-action-with-tool-pause-system,
  tool-pause-handler-mechanism}.feature`
- `spec/attachments/BUG-117/fixes.md` — the pause-pattern rationale ("HITL
  request_user_input is a pause").
- `spec/attachments/TOOL-017/ast-research-handler-pattern.md`
- vtcode reference: `crates/codegen/vtcode-core/src/tools/registry/executors/
  {exec_sessions.rs, exec_support.rs}` (yield-and-resume, next-step hints),
  `vtcode-ui/src/tui/core_tui/session/input.rs:1188` ("waiting for input"
  shimmer needle only).
