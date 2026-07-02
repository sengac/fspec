# RPC-406 — Inline Tool-Approval Pause Prompt in Input Area (TS Parity, Esc Denies)

**Type:** Bug (UX parity + security semantics)
**Crates:** `codelet-fspec-tui` (primary), tests only elsewhere
**Reference implementations:**
- TS: `src/tui/components/InputTransition.tsx` (pause branch, lines 467–533)
- TS: `src/tui/components/AgentView.tsx` (pause key handler, lines 4521–4607; wiring 5483–5504; state 1310–1331)
- TS: `src/tui/components/MultiLineInput.tsx` (the component the pause UI replaces — parity target for the input area swap)
- Rust current: `codelet/fspec-tui/src/views/agent/input_transition.rs` (missing the pause branch entirely)
- tui-textarea clone: `/tmp/tui-textarea` (state-engine reference used by RPC-405; the pause prompt must NOT disturb the TextArea state while swapped in)

---

## 1. Problem statement

When a tool call trips a blocklist `prompt` rule (e.g. reading `.env` → `env-file-prompt` in `~/.fspec/blocklist.json`), the backend pause chain works correctly:

```
read.rs:289 check_file_path → middleware.rs:231 pause_for_user(Triple, "Read")
→ agent_loop.rs:501-518 pause handler → set_pause_state + set_status(Paused)
→ background_session.rs:846 StreamChunk::SessionStateChange{Paused}
→ fspec-tui dispatch_stream_chunks.rs:59 → dispatch_pause_hitl.rs:52 handle_pause_chunk
→ backend.get_pause_state → Action::OpenPauseDialog → components/pause_dialog.rs (MODAL)
```

Two defects:

### Defect A — wrong UX surface (parity violation)
RPC-053 implemented the pause prompt as a **centered Critical-priority modal** (`pause_dialog.rs`, yellow "Tool Pause — Approval Required"). The TS reference **swaps the prompt into the input area**: `InputTransition.tsx:467-533` early-returns the pause UI *instead of* rendering `MultiLineInput`. The RPC-002 port spec (`spec/attachments/RPC-002/09-dialog-and-input-priority-port-spec.md` §C.5) explicitly required the confirm prompt to be *"rendered inline in the composer… a small Component that embeds in the layout (not a popup)"*. Even the middleware's own test step (`middleware.rs:641`) says *"the TUI should show an inline triple pause"*.

### Defect B — Esc silently ALLOWS access (security bug)
Rust modal: Esc → `Action::PauseResumed` → `backend.pause_resume` → `handle_impl.rs:719 send_pause_response(Resumed)` → `middleware.rs:254` catch-all `_ => Ok(())` → **the sensitive file is read**.
TS reference: Esc on a triple pause → `sessionPauseTriple(id, 'deny')` (AgentView.tsx:4593-4600); Esc on a confirm pause → `sessionPauseConfirm(id, false)` (AgentView.tsx:4560). **Esc must deny, never resume.**

---

## 2. Exact TS behavior to replicate (the acceptance-criteria source)

### 2.1 Rendering (InputTransition.tsx:467-533) — replaces the input row content
When `isPaused && pauseInfo` (and no HITL request is active — HITL wins):

**`triple` kind (lines 490–521):**
```
⏸ {toolName}: {message} ({details})           ← ⏸+toolName cyan, message default, details dim, parenthesised
[Allow Once] [Allow Session] [Deny] (←/→ Navigate | Enter Select | Esc Deny)
```
- Option colors: `Allow Once` green, `Allow Session` blue, `Deny` red.
- Selected option rendered **inverse** (`inverse={triplePauseSelection === idx}`).
- Hint text dim.
- Details = the gated command/file path (wire `PauseState.tool_call_id`, populated from internal `details` by `conversions.rs:47-62`; the wire `prompt` field already carries `"{tool_name}: {message}"`).

**`confirm` kind (lines 468–489):**
```
⏸ {toolName}: {message}                        ← yellow
  {details}                                     ← dim, own line, only when present
[Y] Approve [N] Deny (Esc to cancel)            ← green / red / dim
```

**`continue` kind (lines 522–532):** `⏸ {toolName}: {message} (Press Enter to continue)` — NOTE: the Rust wire `PauseKind` intentionally collapses `Continue` into `Confirm` (`rpc-types/src/lib.rs:1045-1053`, `conversions.rs:49`). **Do not add a Continue wire variant** — out of scope; Confirm rendering applies.

### 2.2 Key handling (AgentView.tsx:4521-4607, HIGH priority, only while paused)
| Kind | Key | Action |
|---|---|---|
| triple | ← / → | cycle selection with wraparound (0=Allow Once, 1=Allow Session, 2=Deny) |
| triple | Enter | `pause_triple(session, [Approve, ApproveSession, Deny][selection])`, reset selection to 0 |
| triple | **Esc** | `pause_triple(session, Deny)` |
| confirm | Y/y | `pause_confirm(session, true)` |
| confirm | N/n or **Esc** | `pause_confirm(session, false)` |

- Selection resets to 0 whenever the pause ends or the kind changes (AgentView.tsx:1326-1331).
- While paused, **no other input reaches the MultiLineInput** (the prompt replaces it); printable keys other than the ones above are swallowed by the pause handler's scope, everything routed to the paused session id (not the focused session id).
- `ApprovalChoice` mapping already exists: `Approve→AllowOnce`, `ApproveSession→AllowSession`, `Deny→Denied` (`sessions/src/conversions.rs:72-78`). Reuse — do not duplicate.

### 2.3 Input draft preservation (MultiLineInput parity)
In TS, `MultiLineInput` unmounts during pause but its `value` lives in AgentView state, so the user's draft survives the pause round-trip. In Rust, `MultiLineInput` (`views/agent/multiline_input.rs`) is a persistent struct owned by the view — the pause prompt must only swap the **rendering + key routing**, never touch the TextArea state (`/tmp/tui-textarea` is the state engine; RPC-404/405 wrap geometry lives in `multiline_wrap.rs`). Acceptance criterion: text typed before the pause is intact (content AND cursor position) after the pause resolves. The hardware-cursor containment from RPC-404 must not paint a cursor inside the pause prompt (`InputTransitionState::is_cursor_painted()`-style gating).

---

## 3. Rust architecture (recommended shape — worker may refine, deviations must be documented)

1. **Store:** add per-session `pause_state: Option<codelet_rpc_types::PauseState>` + `triple_pause_selection: usize` to the AgentView store (`store/agent_view/…`, follow the existing per-session slot pattern e.g. `set_session_status` / `isolation_state`).
2. **Dispatch:** in `app/dispatch_pause_hitl.rs`, `handle_pause_chunk`'s pause arm stops pushing the modal: `Action::OpenPauseDialog` is **replaced** by storing the fetched `PauseState` into the store slot (new action, e.g. `Action::PauseStateFetched{session_id, state}` — naming free). HITL arm unchanged (HITL stays a modal; HITL still wins on tie). `handle_pause_cleared` clears the store slot (and still pops the HITL dialog).
3. **Rendering:** `views/agent/input_transition.rs` gets the pause branch. Mirror the existing pattern: the AgentView orchestrator consults the focused session's pause slot BEFORE `paint_input_or_spinner`; when `Some`, paint the inline prompt (2 rows for triple, 2–3 rows for confirm-with-details) into the input area instead of the MultiLineInput/spinner. Input-area height must accommodate the prompt (reuse the RPC-405 auto-grow seam — the input row already supports 1→6 rows).
4. **Key routing:** in the AgentView event path (`views/agent/dispatch.rs` or a new `pause_keys.rs` <300 LoC), when the focused session has a pause slot, consume keys per the table in §2.2 and emit the **existing** actions `Action::PauseTriple{session_id, choice}` / `Action::PauseConfirmed{session_id, accept}` (their `dispatch_pause_hitl.rs` handlers already do the fire-and-forget backend writes — keep them, but they must ALSO clear the store slot). `Action::PauseResumed` must NOT be reachable from the pause prompt (grep-lockable).
5. **Modal removal:** delete `components/pause_dialog.rs` + its mounting (`handle_open_pause_dialog`, `Action::OpenPauseDialog`) OR leave the component file but sever all mounting paths — prefer full deletion; update `components/mod.rs`, `pause_hitl_rpc053.rs` tests, and the RPC-053 feature file `spec/features/pause-and-hitl-dialogs.feature` (pause scenarios superseded — rewrite them against the inline prompt; HITL scenarios untouched).
6. **Multi-session:** pause slot is per-session. The prompt renders only when the paused session is focused (TS parity — TS only ever shows the focused session's pause). A non-focused paused session keeps its slot; switching focus to it shows the prompt. Actions carry the paused session's id.

## 4. Non-goals
- HITL inline UI (BUG-118 TS radio UI) — HitlDialog modal stays.
- Wire `PauseKind::Continue` variant — stays collapsed to Confirm.
- Changing `middleware.rs` response mapping (`Resumed → Ok(())` is required for headless auto-proceed). The fix is UI-side: never send resume from the approval prompt.
- Scrollback notices for backend RPC errors (RPC-053 decided silent tracing logs; unchanged).
- `pause_resume` RPC removal — still used elsewhere (Continue-kind internal pauses via napi path).

## 5. Test plan (minimum)
- Rendering: triple prompt row content/colors/inverse selection; confirm prompt; details shown; no prompt when another session is focused; prompt replaces MultiLineInput (draft chars not painted); cursor not painted while paused.
- Keys: ←/→ wraparound; Enter sends correct ApprovalChoice per selection; **Esc sends Deny (triple) / accept=false (confirm)**; Y/N on confirm; selection resets on clear; keys route to paused session id, not focused id.
- Dispatch: PauseChunkReceived → slot set (no modal pushed); PauseCleared → slot cleared; HITL-wins-on-tie unchanged; stale-Paused (both None) → no slot.
- Draft preservation: type text → pause → deny → text + cursor intact.
- Source-shape: `Action::PauseResumed` not emitted from any pause-prompt key path; `pause_dialog.rs` gone (or unmounted).
- Update, don't orphan: `pause_hitl_rpc053.rs`, `spec/features/pause-and-hitl-dialogs.feature` pause scenarios.

## 6. Manual verification recipe
`~/.fspec/blocklist.json` already has `env-file-prompt` (action: prompt). Run the release TUI, ask the agent to read `.env` → inline prompt must appear in the input area; Esc must produce "User denied access" in the tool result; full release rebuild required to observe (`cd codelet && cargo build --release` — binary is `target/release/fspec`, ~16 min).
