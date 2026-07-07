# RPC-416 — Inline reconnect status in scrollback (replace-in-place + auto-dismiss)

## Summary

Replace the **modal** reconnect UI (`DisconnectDialog`) with **inline scrollback
status lines** in the focused session, matching the TypeScript reference UX:

- On disconnect → show an inline `⟳ Reconnecting…` line in the focused session's
  scrollback.
- On each retry attempt → update that **same** line in-place
  (`⟳ Reconnecting… (attempt N)`), do **not** push a new line per attempt.
- On successful reconnect → replace that same line in-place with `✓ Reconnected`.
- After a short delay (~1.5–2s) → the `✓ Reconnected` line **auto-dismisses**
  (is removed from scrollback).

**The `DisconnectDialog` modal is removed entirely** (per product decision:
"Fully inline, remove modal"). The inline line lives in the **focused session
only** (per product decision).

## Product Decisions (locked)

1. **Fully inline, remove modal.** All states (disconnected, reconnecting,
   reconnected) are inline scrollback lines. `DisconnectDialog` is deleted.
2. **Focused session only.** The line is pushed/updated/removed in whichever
   session is currently focused when the disconnect begins.
3. **Depends on RPC-415.** The subscriber-respawn bug is fixed first so
   `✓ Reconnected` is truthful.

## Why This Is Net-New Work (not a straight TS port)

- In **TS**, `⟳ Reconnecting...` is a `UserNotification` chunk tied to a
  **session's** LLM/SSE stream retry, so it naturally lands in that session's
  scrollback. TS replaces it in-place with `✓ Reconnected` / `✗ Reconnection
  failed` via `Array.findLastIndex` (`src/tui/components/AgentView.tsx`, 3
  duplicated code paths ~479–498, 2385–2415, 3430–3461). **TS never removes the
  line** — it persists in scrollback forever.
- In **Rust**, the reconnect event is **connection-level** (WebSocket transport
  drop in `transport/websocket.rs`), **not** tied to any session. Rust scrollback
  is **strictly per-session** (`SessionContext.scrollback: ScrollbackList` — no
  global buffer).
- Therefore two things are genuinely new in Rust:
  1. **Routing** a connection-level event into a chosen (focused) session's
     scrollback.
  2. **Auto-dismiss** of an inline line — this exists in **neither** codebase
     (TS replaces but never removes). The "then it goes away" behaviour is new.

## Relevant Existing Infrastructure

### Scrollback data structures
- `SessionContext` — `codelet/fspec-tui/src/store/agent_view/session_context.rs:30-55`
  (`scrollback: ScrollbackList`, `scrollback_next_seq: u64`).
- `ScrollbackList` — `codelet/fspec-tui/src/views/agent/scrollback.rs:36-57`
  (private `chunks: Vec<RenderedChunk>`).
- `RenderedChunk` — `codelet/fspec-tui/src/views/agent/rendered_chunk.rs:96-103`
  (**stable `seq: u64`**, `lines`, `source`).
- `ChunkKind::Notification` — `rendered_chunk.rs:48-49` (rendered verbatim,
  white, plain wrap, no bullet; `chunk_wrap.rs:65`).

### Existing push / notice paths
- `SessionContext::push_line` — `session_context.rs:159-168` (builds a
  `Notification` chunk, allocates `seq`, appends).
- `AgentView::push_line` (focused-session wrapper) — `views/agent.rs:192-196`.
- `Action::EmitSessionNotice(SessionId, String)` — `components/mod.rs:441-443`;
  dispatched `app/dispatch.rs:273-275` → `handle_emit_session_notice`
  (`app/dispatch_slash_clear.rs:30-34`). Targets a session **by explicit
  SessionId**; silent no-op if the session is gone.

### Existing mutation / removal precedent (NOT append-only)
- `ScrollbackList::chunks_mut()` — `scrollback.rs:102`.
- In-place edit pattern: `chunks_mut().get_mut(idx)` → edit `source.text` →
  `rewrap_at(idx)` (e.g. `chunk_processor.rs:23,64,101,156,213,244`).
- Removal pattern: `chunks_mut().remove(idx)` (e.g. `chunk_processor.rs:243,291`)
  — note existing callers manually re-anchor `in_flight_assistant` /
  `in_flight_thinking` index slots and selection/scroll state afterward.
- **Net-new helpers recommended:** `replace_by_seq(seq, new_source)` and
  `remove_by_seq(seq)` on `ScrollbackList` (seq→index resolved at call time via
  linear scan like `full_text_for_seq` at `scrollback_select.rs:114`), because a
  raw `usize` index cannot be safely cached across intervening pushes/removals.

### Existing auto-dismiss timer pattern (reuse)
- `NotificationDialog` — `codelet/fspec-tui/src/components/notification_dialog.rs:174-191`
  (`tokio::spawn` + `tokio::time::sleep` → `Action::DismissDialog(id)`; aborts on
  Drop).
- `StatusDialog` — `codelet/fspec-tui/src/components/status_dialog.rs:149-159`
  (same pattern, 3s auto-close).
- Central dismiss routing: `Action::DismissDialog` →
  `app/dispatch_dialog_dismiss.rs:41-48` → `compositor.remove(id)`.
- **For inline** we need a **new action** (e.g. `ClearReconnectNotice { session_id,
  seq }`) that resolves to `remove_by_seq` on the target session, since
  `DismissDialog` removes compositor layers, not scrollback lines.

### Current modal wiring to be removed
- `DisconnectDialog` — `codelet/fspec-tui/src/components/disconnect_dialog.rs`
  (`DISCONNECT_DIALOG_ID`, `attempt: Option<u32>`, body at 56–62).
- Push on `Disconnected` — `app/dispatch.rs:39-41`.
- Remove on `Reconnected` — `app/dispatch.rs:46` (keep the re-bootstrap at 47–64).
- Transport emits `Disconnected` / `Reconnecting(n)` / `Reconnected` —
  `transport/websocket.rs:1342,1358,1395`.
- `q to quit` / `r to reconnect` (`ManualReconnect`) affordances currently live in
  the modal — see "Lost affordances" below.

### Render loop / idle repaint
- 60fps loop with action-channel arm — `app/events.rs:227-271`; idle frames skip
  drawing via `tick_should_draw` (`app/mod.rs:78-80`). Both the in-place replace
  and the timed removal arrive as **Actions**, which wake `select!` → dispatch →
  redraw. **Do not** attempt a live "closing in Ns…" countdown (idle frames won't
  repaint it).

## Design (target behaviour)

1. **On `Action::Disconnected`:** push `⟳ Reconnecting…` into the **focused**
   session's scrollback; record `(SessionId, seq)` on App/transport-facing state
   so later actions can find the exact line.
2. **On `Action::Reconnecting(n)`:** update the tracked line in-place to
   `⟳ Reconnecting… (attempt N)` via `replace_by_seq`. No new line.
3. **On `Action::Reconnected`:** replace the tracked line in-place with
   `✓ Reconnected` (green), then arm a `tokio::sleep(~1.5–2s)` →
   `Action::ClearReconnectNotice { session_id, seq }`. Keep the existing
   re-bootstrap logic (list_work_units + create_session).
4. **On `ClearReconnectNotice`:** `remove_by_seq(seq)` on the target session;
   silent no-op if session/seq gone. Re-anchor in-flight/selection state.
5. **Remove `DisconnectDialog`** and its push/remove wiring.

## Edge Cases (must be handled explicitly)

1. **Re-drop within the success window.** If `Disconnected` fires during the
   ~2s `✓ Reconnected` display: abort the pending clear timer and revert the line
   to `⟳ Reconnecting…` (reuse the same seq if still present; otherwise push a
   fresh line).
2. **Target session closed before timer fires.** `ClearReconnectNotice` /
   `remove_by_seq` must silently no-op (mirror `EmitSessionNotice`'s silent
   drop).
3. **Focus changes between disconnect and reconnect.** The line was pushed to
   session A (focused at disconnect time). Replace/remove must target **session A
   by SessionId**, not "whatever is focused now."
4. **No open session at disconnect time.** If there is no focused session to push
   into, degrade gracefully (skip inline line; no panic). Define behaviour.
5. **Flapping.** Rapid disconnect/reconnect cycles must not leak lines or timers
   (each cycle reuses/cleans its tracked seq; abort superseded timers).
6. **Idle repaint.** Rely on Action-driven wake; no countdown animation.

## Out of Scope

- `✗ Reconnection failed` terminal state. The Rust transport currently retries
  with capped backoff and effectively never terminates, so there is no natural
  trigger. If a failed state is later added, a `✗` line can reuse the same
  replace/remove machinery. (Tracked separately; not in this card.)
- The subscriber-respawn correctness fix (that is **RPC-415**, a dependency).

## Lost Affordances (decide during specifying)

Removing the modal drops its `q to quit` / `r to reconnect` (`ManualReconnect`)
keybinding surface. Since the decision is "fully inline, remove modal," confirm
where (if anywhere) manual-reconnect/quit affordances now live — e.g. rely on the
transport's automatic capped-backoff reconnect (no manual trigger needed), or
surface `ManualReconnect` via an existing global keybinding. Document the chosen
approach in the feature file's architecture note.

## Acceptance Criteria (to be turned into scenarios)

1. On disconnect, an inline `⟳ Reconnecting…` line appears in the focused
   session's scrollback (no modal is shown).
2. On retry attempt N, the same line updates in-place to include the attempt
   count; the scrollback does **not** gain additional reconnect lines.
3. On successful reconnect, the same line is replaced in-place with
   `✓ Reconnected`.
4. The `✓ Reconnected` line is automatically removed from scrollback after the
   configured short delay.
5. If a new disconnect occurs during the success window, the pending removal is
   cancelled and the line reverts to a reconnecting state.
6. Replace/remove always target the originating session (by SessionId), even if
   focus changed.
7. If the originating session is closed before the removal timer fires, no panic
   occurs and the operation is a silent no-op.
8. `DisconnectDialog` no longer appears for any disconnect/reconnect flow.

## Key File / Line Reference

| Concern | File | Lines |
|---|---|---|
| `SessionContext` (scrollback + seq) | `store/agent_view/session_context.rs` | 30–55 |
| `ScrollbackList` (`chunks` Vec) | `views/agent/scrollback.rs` | 36–57, 102 |
| `RenderedChunk` (stable `seq`) | `views/agent/rendered_chunk.rs` | 96–103 |
| `push_line` | `store/agent_view/session_context.rs` | 159–168 |
| In-place edit precedent | `views/agent/chunk_processor.rs` | 23,64,101,156,213,244 |
| Removal precedent | `views/agent/chunk_processor.rs` | 243,291 |
| `EmitSessionNotice` handler | `app/dispatch_slash_clear.rs` | 30–34 |
| Auto-dismiss timer pattern | `components/notification_dialog.rs` | 174–191 |
| Auto-dismiss timer pattern | `components/status_dialog.rs` | 149–159 |
| `DismissDialog` routing | `app/dispatch_dialog_dismiss.rs` | 41–48 |
| DisconnectDialog (to remove) | `components/disconnect_dialog.rs` | whole file |
| Modal push/remove wiring (to remove) | `app/dispatch.rs` | 39–41, 46 |
| Transport emits actions | `transport/websocket.rs` | 1342, 1358, 1395 |
| TS reference (replace-in-place) | `src/tui/components/AgentView.tsx` | 479–498, 2385–2415, 3430–3461 |
| Render loop / idle repaint | `app/events.rs`, `app/mod.rs` | 227–271, 78–80 |

## Addendum: RPC-011 ripple (removing the modal breaks existing @done scenarios)

`spec/features/auto-reconnect-supervisor.feature` (`@RPC-011`, `@done`) and its
test `codelet/fspec-tui/tests/auto_reconnect_slice2_rpc011.rs` assert the
DisconnectDialog directly. Removing the modal (this card's decision) invalidates:

- Scenario "Auto-reconnect happy path" — line 50: "And the App pops the
  DisconnectDialog from the Compositor".
- Scenario "Auto-reconnect Reconnecting Action updates the dialog text inline"
  (lines 61-65) and its test (`auto_reconnect_slice2_rpc011.rs:263-299`), which
  assert `disconnect-dialog` is topmost and mutated in place.
- Scenario "Client receives ServerGoingAway..." — line 72 references the dialog
  text.

RPC-416 MUST update these existing RPC-011 scenarios + tests so they describe the
new inline behaviour (assert the focused session's scrollback contains the
`⟳ Reconnecting…` / `✓ Reconnected` line and that it auto-dismisses), instead of
asserting the removed modal. Keep RPC-011's transport/backoff scenarios intact —
only the presentation (modal → inline) changes. Coordinate so the RPC-011 test
file still compiles and passes after DisconnectDialog is deleted.

Backoff schedule reminder: transport supervisor uses 250ms → 500 → 1000 → 2000 →
5000 cap (auto-reconnect-supervisor.feature lines 27-40).
