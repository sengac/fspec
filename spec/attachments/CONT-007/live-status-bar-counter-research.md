# Research: Live Status-Bar Updates for Auto-Continue / Goal Counters (VERIFIED 2026-07-10)

## Problem

The `⏩ auto-continue (n/N)` / `🎯 goal (n/N)` indicator in the TUI footer (the bar directly
above the message input) never updates during a turn. Counter changes are instead emitted
into the CHAT transcript, and some transitions (refunds) are emitted nowhere at all.

## 1. Where the bar is painted and what it reads

### Layout — the bar IS directly above the input
`codelet/fspec-tui/src/views/agent.rs:250-267`: vertical split
`Header(1), RoleBanner, Scrollback, Footer(1), Input(h)`; `footer: split[3]` sits
immediately above `input: split[4]`. Painted at views/agent.rs:286 →
`chrome_paint::paint_footer`.

### Indicator assembly — `chrome_paint.rs:92-119`
```rust
// codelet/fspec-tui/src/views/agent/chrome_paint.rs:102-111
// CONT-002: … The TUI paints the cached (enabled, budget) pair; the
// nudge count is per-turn engine state, shown as 0 between turns.
let continue_indicator = sid.and_then(|s| {
    let (enabled, budget) = store.continue_state_for(s);
    let goal_active = store.goal_state_for(s).is_some();
    crate::app::goal_parser::goal_status_indicator(goal_active, enabled, 0, budget)
});
```
**The `n` in `(n/N)` is a hard-coded `0` — always, not just between turns.**

### Rendering — `footer.rs`
`codelet/fspec-tui/src/views/agent/footer.rs:45-47` (field), :69-78 (paint, cyan,
left-aligned). Suppressed when the supervisor chip (:60) or compaction chip (:67) claims
the slot.

### Formatting helpers
- `app/continue_parser.rs:154-158` `continue_status_indicator` → only referenced from tests
  (fspec-tui/tests/cont002_continue_command_test.rs:176,187).
- `app/goal_parser.rs:149-165` `goal_status_indicator` → the live paint path, called with
  `nudges_used = 0`.

### Data source and refresh cadence
Paint reads only the TUI-local cache `store/agent_view/chrome_state.rs`:
`continue_state_for` (:55-60, default `(false, 10)`), `set_continue_state` (:64-67),
`goal_state_for` (:73-75), `set_goal_state` (:79-92).

Cache written from exactly TWO places, both user slash-command dispatches:
- `app/dispatch_slash_continue.rs:50-54`
- `app/dispatch_slash_goal.rs:50-51`

The footer is painted every frame, but the DATA only changes on `/continue` or `/goal`.
The RPC getters exist end-to-end (`FspecBackend::get_continue_state`/`get_goal_state`:
transport/mod.rs:397,418; transport/embedded.rs:409,429; transport/websocket.rs:668,693;
served by sessions/handle_impl.rs:1352,1377 → background_session.rs:1191,1206) — **but the
TUI never calls them** (grep-verified: no callers in fspec-tui/src/app or bootstrap).
Dead plumbing for UI purposes.

## 2. Counter/state changes currently landing in CHAT

All engine emissions go through `emit_status` → `StreamEvent::Status` (cli output.rs:261-263)
→ CLI stdout (output.rs:437-442) OR `StreamChunk::user_notification`
(agent-loop/src/background_output.rs:216-219; napi mirror napi/src/agent_loop.rs:1627-1630)
→ TUI transcript as `ChunkKind::Notification` (store/agent_view/session_context.rs:125-133).

| What | file:line | Should be |
|---|---|---|
| **Nudge consumption `⏩ auto-continue: nudging (n/N)`** | stream_loop.rs:1546-1559 (`emit_status` at :1556-1559) | **BAR — the only live surface of `n`, and it spams chat once per nudge.** Also prints `continue_budget` instead of the effective Goal budget (cf. :1493 vs :1558) |
| **Nudge refund** | stream_loop.rs:1467-1478 (`apply_segment_outcome` decrements silently) | **Invisible everywhere today** — must surface in bar |
| Budget exhaustion `⚠ … never called done() after N retries` | stream_loop.rs:1527-1531 (msg from auto_continue.rs:170-172) | Chat OK (terminal event) + bar should reflect exhaustion (e.g. `(N/N)`) |
| `✓ done: <summary>` | stream_loop.rs:1523 | Chat OK (turn-terminating) + bar reset |
| `🎯 goal satisfied: <summary>` | stream_loop.rs:1516-1518 (goal.rs:81-84) | Chat OK + bar clears goal (see CONT-008) |
| Escalation blocked messages | stream_loop.rs:1533-1544 (msgs goal.rs:55-58, auto_continue.rs:129-134, :146-148) | Chat OK + bar state |
| Synthetic nudge user message | stream_loop.rs:1575-1587 (`AUTO_CONTINUE_NUDGE_PROMPT`, auto_continue.rs:26-27, pushed as `Message::User`) | **Stays in chat — genuine conversation content** |
| done() rejection tool errors | done.rs:334-353 (`ToolError::Validation`) | Stays in chat — tool results |
| `/continue` acknowledgement prints | TUI dispatch_slash_continue.rs:34-43 (msgs continue_parser.rs:77-147); CLI repl_loop.rs:158-183 | Chat OK (command responses) |
| `/goal` acknowledgement prints | TUI dispatch_slash_goal.rs:34-43 (msgs goal_parser.rs:85-141 — note :88,:98 hard-code "nudges used: 0"); CLI repl_loop.rs:139-152 | Chat OK, but state text must come from real state, not hard-coded zeros |

## 3. Why the bar CANNOT be live today (data gap)

- Live `n` lives on the inner CLI `Session` owned by the stream loop
  (`session.continue_nudges_used` — stream_loop.rs:1468, :1547, reset :1525).
- Chrome-visible `BackgroundSession` state (background_session.rs:397-406) holds only
  `(enabled, budget)` atomics + goal `(text, verify)` — **no nudge counter exported**.
  Sync is one-way chrome→inner at dispatch (agent-loop/src/agent_loop.rs:495-530, now via
  the CONT-009 shared helper in sessions/src/background_session.rs).
- Even per-frame polling of the existing RPC getters could not show `n`.

### Existing push mechanisms (precedent)
`StreamChunk` (rpc-types/src/lib.rs:1194-1305) has state-only variants consumed outside the
transcript ("State-only chunks — consumed elsewhere" arm, session_context.rs:144-155):
- `CompactionProgress` → footer chip read per frame (chrome_paint.rs:98)
- `SupervisorPendingInjection` → live footer chip (session_context.rs:152, supervisor_state.rs)
- `FooterStateUpdate`, `TokenUpdate`, `ContextFillUpdate`, …

**No chunk exists for continue/goal counters.** Push (not polling) is the codebase precedent.

## 4. Proposed architecture

1. **New state-only chunk** e.g. `StreamChunk::ContinueStateUpdate { enabled, budget,
   nudges_used, goal_active, effective_budget }` (rpc-types), mapped from a new
   `StreamEvent` variant in `background_output.rs` + napi mirror.
2. **Emit at every transition** in stream_loop.rs: nudge consume (:1547), refund
   (:1467-1471), turn reset (`reset_for_new_user_turn`, agent-loop agent_loop.rs:503),
   exhaustion/finish (:1525, :1531), goal accept/clear (:1517-1519).
3. **Consume in the TUI** in session_context.rs's state-only arm → write into
   chrome_state.rs (extend cache with `nudges_used`/`effective_budget`), thread into
   `chrome_paint.rs:110` replacing the literal `0`.
4. **Remove** the per-nudge chat print (stream_loop.rs:1556-1559). CLI repl keeps a stdout
   line (it has no bar — `CliOutput` can keep rendering Status or the new event as text).
5. **Fix** effective-budget display: nudging line/bar must show
   `max(explicit, 15)` in Goal mode (stream_loop.rs:1493 computes it; :1558 ignores it).

### CLI split point (confirmed fine)
`StreamOutput` trait (cli output.rs:186): `CliOutput` (stdout) vs `BackgroundOutput`
(StreamChunk → TUI). CLI repl has no chrome; `println!` surfaces are correct there
(repl_loop.rs:151, :182). `cli/src/interactive/goal.rs:206` `goal_status_indicator` is
test-only. A fix touches only the StreamEvent→StreamChunk mapping and TUI side; CLI
behavior can stay byte-for-byte.

## Dependencies / relations

- Builds on CONT-002/CONT-003 (done).
- **Related: CONT-008** (goal back-sync — shares the push channel; goal clear must also
  update the bar).
- **Related: CONT-009** (napi arming gap — fixed; counters now move on the NAPI surface).

## Test Coverage Sketch

- Nudge consumed → bar shows (1/N) same frame-ish (chunk dispatched), no chat notification.
- Refund → bar decrements; previously invisible transition now observable.
- Turn reset on real user message → bar shows (0/N).
- Goal mode → bar shows effective budget max(explicit,15), not continue_budget.
- Exhaustion → bar shows (N/N) + warning still in chat.
- CLI repl output unchanged (byte-for-byte) apart from the removed/retained nudging line
  decision.
