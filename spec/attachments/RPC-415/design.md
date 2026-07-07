# RPC-415 — Live streaming dies permanently after first auto-reconnect

## Summary

After the **first** successful WebSocket auto-reconnect, the Rust ratatui TUI
silently stops receiving **all** live streaming data. The board stops updating,
agent chunks stop rendering, logs/status/session-created events stop arriving.
The connection appears healthy (the disconnect UI goes away), but no live data
flows again **until the process is restarted**.

This is a **critical correctness bug**: the reconnect *looks* successful while
the app is functionally dead for streaming.

## Severity / Impact

- **Severity:** Critical (silent, permanent data loss for the session).
- **Trigger:** Any transient WebSocket drop that the transport supervisor
  auto-recovers from (network blip, daemon restart, laptop sleep/wake).
- **Blast radius:** All 5 broadcast streams — work_units, chunks, logs,
  status_changes, session_created.
- **User-visible symptom:** "Everything froze after my connection came back;
  had to restart the TUI."

## Root Cause (code-verified)

### 1. Subscriber loops break permanently on `Closed`

`codelet/fspec-tui/src/app/bootstrap.rs:137-244` spawns **5** long-lived
subscriber tasks, one per broadcast stream. Each loop has the same terminal arm:

```rust
Err(broadcast::error::RecvError::Closed) => break,
```

When the WebSocket drops, the **old** RPC client is dropped by the transport
supervisor. Dropping the client drops its broadcast `Sender`s. Every subscriber's
`Receiver` then returns `RecvError::Closed`, so **all 5 tasks hit `break` and
exit permanently**.

### 2. `spawn_subscriber_tasks()` is only called once

`spawn_subscriber_tasks()` is invoked a **single** time at
`codelet/fspec-tui/src/app/bootstrap.rs:52` (grep-confirmed single call site).
It is never re-invoked after a reconnect. Once the 5 tasks exit, nothing
respawns them.

### 3. `Action::Reconnected` handler does not respawn subscribers

`codelet/fspec-tui/src/app/dispatch.rs:45-64` handles `Action::Reconnected` by:

- removing the `DisconnectDialog` from the compositor (`dispatch.rs:46`),
- doing a **one-shot** `list_work_units()` refetch,
- calling `create_session(None)`.

It **never** respawns the subscriber tasks against the **new** client's
broadcast receivers. That is why the board updates *once* (the refetch) but never
streams again.

### 4. The transport supervisor's re-subscribe does not help the App bus

The transport supervisor in `codelet/fspec-tui/src/transport/websocket.rs`
re-subscribes to the **new** client's `chunks_rx()` after reconnect — but that is
only its **own** drop-detector (so it can notice the *next* disconnect). It does
**not** forward those broadcasts onto the App action bus. The App-side subscriber
tasks remain dead.

## Masking Factors (why this wasn't caught)

- **Stale/false doc comment** at `codelet/fspec-tui/src/components/mod.rs:160-163`
  claims the `Reconnected` action "resubscribed three broadcasts." This is
  (a) **not implemented**, and (b) **stale** — there are now **5** streams, not 3.
- **Unasserted Gherkin step** at
  `codelet/fspec-tui/tests/auto_reconnect_slice2_rpc011.rs:179`:
  `// @step And it respawns the three subscriber tasks` — there is **no backing
  assertion**, so the test passes while the behaviour is absent.

## Fix Strategy

On `Action::Reconnected`, **respawn the subscriber tasks** bound to the **new**
RPC client's broadcast receivers, after tearing down / confirming the old tasks
are gone.

Design considerations:

1. **Single source of truth for spawning.** Reuse `spawn_subscriber_tasks()`
   (or an extracted helper) so bootstrap and reconnect share one code path — no
   duplicated loop bodies (DRY).
2. **No leaked/duplicate tasks.** Ensure the old 5 tasks are fully terminated
   (they self-exit on `Closed`) before/around respawning, or track `JoinHandle`s
   and abort defensively so a flapping connection cannot accumulate N×5 tasks.
3. **Bind to the new client.** The respawn must subscribe to the *current*
   client's `*_rx()` receivers, not stale ones.
4. **Idempotency under flapping.** Multiple `Reconnected` actions in quick
   succession must not stack duplicate subscriber sets.
5. **Keep the one-shot refetch.** Preserve the existing `list_work_units()`
   refetch and `create_session` behaviour where still correct.

## Cleanup Included in This Card

- Fix the stale doc comment at `components/mod.rs:160-163` to state what the
  handler **actually** does (respawn **5** subscriber tasks), and correct the
  count.
- Fix the unasserted Gherkin step at
  `tests/auto_reconnect_slice2_rpc011.rs:179` so the "respawns the subscriber
  tasks" step has a **real assertion** (correct the "three" → actual count and
  assert the tasks are live / receiving after reconnect).

## Acceptance Criteria (to be turned into scenarios)

1. After a simulated WebSocket drop + successful auto-reconnect, **all 5**
   broadcast streams deliver a subsequently-emitted event to the App
   (work_units, chunks, logs, status_changes, session_created).
2. Respawning binds to the **new** client (an event emitted only by the new
   client's senders is received).
3. Repeated reconnects (flapping) do **not** accumulate duplicate subscriber
   tasks (no N×5 growth; each stream delivers each event exactly once).
4. The doc comment at `components/mod.rs` accurately describes the respawn and
   the correct stream count.
5. The `auto_reconnect_slice2_rpc011.rs` "respawns the subscriber tasks" step is
   backed by a real assertion that fails if respawn is removed.

## Key File / Line Reference

| Concern | File | Lines |
|---|---|---|
| 5 subscriber loops, `Closed => break` | `codelet/fspec-tui/src/app/bootstrap.rs` | 137–244 |
| Single `spawn_subscriber_tasks()` call | `codelet/fspec-tui/src/app/bootstrap.rs` | 52 |
| `Reconnected` handler (no respawn) | `codelet/fspec-tui/src/app/dispatch.rs` | 45–64 |
| Transport supervisor re-subscribe (own detector only) | `codelet/fspec-tui/src/transport/websocket.rs` | ~1390–1395 |
| Stale/false doc comment ("three broadcasts") | `codelet/fspec-tui/src/components/mod.rs` | 160–163 |
| Unasserted Gherkin step | `codelet/fspec-tui/tests/auto_reconnect_slice2_rpc011.rs` | 179 |

## Relationship to RPC-416

**RPC-416 (inline reconnect status) depends on this card.** RPC-416 shows a
`✓ Reconnected` success line; if streaming is still dead, that message would
actively lie to the user. This bug must be fixed **first** so the success
indicator is truthful.

## Addendum: RPC-011 ripple (existing @done artifacts)

The stale claim lives in the shipped, `@done` feature
`spec/features/auto-reconnect-supervisor.feature` (tag `@RPC-011`):

- Line 17 (architecture doc): "...create_session(None) + resubscribe broadcasts..."
- Line 48 (Scenario "Auto-reconnect happy path"):
  `And it respawns the three subscriber tasks against the new chunks/logs/work_units broadcasts`

The matching unasserted step is `auto_reconnect_slice2_rpc011.rs:178-179`.

RPC-415 must make this claim TRUE (respawn actually happens, count corrected to
the real number — currently 5 streams) and backed by a real assertion. Update
the existing feature step wording (three → correct count) OR add a new
RPC-415-tagged scenario that asserts respawn behaviour, and update the test with
a real assertion. Prefer a new RPC-415 feature file for the correctness
behaviour, plus a minimal correction to the stale RPC-011 step so documentation
is not left lying.

Note: the real transport backoff schedule is 250ms → 500 → 1000 → 2000 → 5000
(cap), reset on first successful frame (see auto-reconnect-supervisor.feature
lines 27-40) — NOT the 1s→30s schedule of the unrelated TS `bridge/relay-endpoint.ts`.
