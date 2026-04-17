# BRIDGE-020 — Pre-existing `test_sess018_*` Failures in `bridge_relay.rs`

## Summary

`cargo test --package codelet-tools --lib bridge_relay::` fails with 5 tests
broken on pristine HEAD. These failures existed **before** the CMPCT-022 work
landed — verified by `git stash && cargo test` producing the exact same
failures. They were surfaced only because CMPCT-023's validation pass ran the
whole workspace test suite, not just `codelet-cli`.

All failures live in a single file:

```
codelet/tools/src/bridge_relay.rs
```

All failing tests belong to **SESS-018** (per-session multiplexed routing for
the Telegram/Dashboard bridge). The test authors wrote the behavioural
contracts; the production code was never updated to honour them. Every failure
points to an **unwired handler arm** in `handle_multiplexed_inbound`
(`bridge_relay.rs:420`) and its terminal sub-dispatcher.

---

## Failing tests

| # | Test | File:line of panic | Root cause location |
|---|------|--------------------|---------------------|
| 1 | `test_sess018_session_input_routes_by_envelope_session_id` | `bridge_relay.rs:1309` | `bridge_relay.rs:434-450` (SessionInput arm) |
| 2 | `test_sess018_session_control_routes_by_envelope_session_id` | `bridge_relay.rs:1423` | `bridge_relay.rs:452-463` (SessionControl arm) |
| 3 | `test_sess018_terminal_destroy_removes_and_responds` | `bridge_relay.rs:1625` | `bridge_relay.rs:624-631` (stub fallback) |
| 4 | `test_sess018_terminal_create_spawns_reader_emitting_data_envelopes` | `bridge_relay.rs:1695` | `bridge_relay.rs:566-623` (no reader task spawned) |
| 5 | `test_sess018_terminal_input_writes_to_pty` | **HANGS** (>60 s) | `bridge_relay.rs:624-631` (stub fallback) |

### How to reproduce

From repo root:

```bash
cd codelet
# Non-hanging failures (run all four at once)
cargo test --package codelet-tools --lib \
  bridge_relay::tests::test_sess018_session_input_routes_by_envelope_session_id \
  bridge_relay::tests::test_sess018_session_control_routes_by_envelope_session_id \
  bridge_relay::tests::test_sess018_terminal_destroy_removes_and_responds \
  bridge_relay::tests::test_sess018_terminal_create_spawns_reader_emitting_data_envelopes \
  -- --nocapture

# Hanging test — needs timeout
timeout 15 cargo test --package codelet-tools --lib \
  bridge_relay::tests::test_sess018_terminal_input_writes_to_pty \
  -- --nocapture
```

---

## Failure 1 — `test_sess018_session_input_routes_by_envelope_session_id`

**Observed panic:**
```
assertion `left == right` failed: new session injector should receive the input
  left: 0
 right: 1
```

**What the test expects:**
Dashboard sends a `session:input` envelope with an explicit `session_id` that
targets the newly-created tab #2. The test registers a per-session injector via
`crate::set_bridge_session_context(new_session_id, …, new_injector, …)` and
expects the bridge to route the message to **`new_injector`** — NOT the
fallback `supervisor_injector` passed as a parameter to
`handle_multiplexed_inbound`.

**What the code actually does** (`bridge_relay.rs:434-450`):

```rust
InboundAction::SessionInput {
    session_id: _,       // ← envelope's session_id is DISCARDED
    message,
    images,
} => {
    // … builds InjectedInput …
    input_injector(injected);   // ← ALWAYS calls the caller-provided injector
    Ok(None)
}
```

The arm destructures `session_id` into `_` and never consults
`BRIDGE_SESSION_CONTEXTS`. Every inbound `session:input` envelope is routed to
the supervisor's fallback injector regardless of which tab sent it.

**Fix required:**
1. Capture the envelope's `session_id`.
2. Look it up via the existing `BRIDGE_SESSION_CONTEXTS` registry (same
   mechanism SessionCreate uses).
3. If a context is found, invoke `context.input_injector(injected)`.
4. If not found (or the envelope has no `session_id`), fall back to the
   parameter `input_injector` (this preserves
   `test_sess018_session_input_without_context_falls_back_to_parameter_injector`
   which currently passes).

---

## Failure 2 — `test_sess018_session_control_routes_by_envelope_session_id`

**Observed panic:**
```
assertion `left == right` failed: new session control handler should receive the interrupt
  left: 0
 right: 1
```

**What the test expects:**
The user clicks "Interrupt" on tab #2. The dashboard sends a `session:control`
envelope with `action: "interrupt"` and `session_id` of tab #2. The test
registers a per-session `ControlHandler` via `set_bridge_session_context` and
expects the bridge to dispatch the interrupt to **that** handler, not to the
supervisor's.

**What the code actually does** (`bridge_relay.rs:452-463`):

```rust
InboundAction::SessionControl {
    session_id: _,       // ← envelope's session_id is DISCARDED
    action,
    response,
} => {
    if let Some(handler) = control_handler {      // ← parameter handler only
        handler(&action, response.as_deref());
    } else {
        tracing::warn!("Received control '{}' but no handler configured", action);
    }
    Ok(None)
}
```

Same bug shape as Failure 1. The envelope's `session_id` is discarded.

**Fix required:**
1. Capture the envelope's `session_id`.
2. Look it up via `BRIDGE_SESSION_CONTEXTS`; if a context with a
   `control_handler` is registered, use it.
3. Otherwise fall back to the parameter `control_handler`.

---

## Failure 3 — `test_sess018_terminal_destroy_removes_and_responds`

**Observed panic:**
```
Expected terminal:destroyed response, got Ok(None)
```

**What the test expects:**
A `terminal:destroy` envelope with `terminal_id` and `request_id: "destroy-1"`
should:
1. Call `crate::destroy_terminal(&registry, &term_id)` (or equivalent).
2. Return `Ok(Some(Envelope { service: Terminal, msg_type: "destroyed",
   terminal_id, request_id }))`.
3. Leave the `PtyRegistry` empty (`registry.len() == 0`).

**What the code actually does** (`bridge_relay.rs:624-631`):

```rust
InboundAction::TerminalInput { .. }
| InboundAction::TerminalResize { .. }
| InboundAction::TerminalDestroy { .. } => {
    // Other terminal actions still require PTY-side wiring beyond
    // the scope of SESS-017 (which only fixes the create handshake).
    tracing::warn!("Terminal action received but not yet wired to PtyRegistry");
    Ok(None)
}
```

All three variants fall through to a stub that returns `Ok(None)`. SESS-017
wired only `TerminalCreate`; SESS-018 requires wiring the remaining three.

**Fix required:**
Split the combined arm into three separate handlers:

1. `InboundAction::TerminalDestroy { terminal_id, request_id }` →
   - Query `PtyRegistry` via `query_pty_registry()`.
   - Call the registry's destroy method (look for symmetry with `create_terminal`).
   - Return `Envelope::terminal_destroyed(&instance_id, &request_id, &terminal_id)`
     (helper may already exist; if not, add it alongside
     `Envelope::terminal_created`).

2. `InboundAction::TerminalInput { terminal_id, base64 }` → see Failure 5.

3. `InboundAction::TerminalResize { terminal_id, cols, rows }` → call the PTY
   registry's resize method. The test
   `test_sess018_terminal_resize_updates_pty_size` currently passes, meaning
   the resize path is either already wired or the test is asserting something
   weaker — verify before touching.

---

## Failure 4 — `test_sess018_terminal_create_spawns_reader_emitting_data_envelopes`

**Observed panic:**
```
expected at least one terminal:data envelope from the spawned PTY reader
```

(after a 5s timeout waiting on `outbound_rx.recv()`)

**What the test expects:**
When a `terminal:create` envelope arrives, the bridge must:
1. Respond with `terminal:created` (✅ already works).
2. **Spawn a background reader task** that drains the PTY master's output
   stream and forwards each chunk as a `terminal:data` envelope through
   `register_outbound_control`'s `OUTBOUND_CONTROL_SENDERS` registry.

**What the code actually does** (`bridge_relay.rs:566-622`):

The `TerminalCreate` handler correctly calls `crate::create_terminal` and
returns `terminal:created`, but **never spawns the reader task**. Without a
reader, the shell's startup prompt (e.g., `"$ "`) is buffered in the PTY master
forever, and the dashboard never sees any output.

**Fix required:**
Inside the `Ok((terminal_id, entry)) => { … }` branch at line 577, after
sending the `terminal:created` response, spawn a `tokio::task` that:
1. Acquires the PTY master via `entry.master`.
2. Clones the reader.
3. In a loop: reads bytes, base64-encodes them, wraps in an `Envelope`
   (`service: Terminal`, `msg_type: "data"`, `terminal_id`), and sends via the
   outbound control tx registered under the current connection owner.
4. Exits cleanly on read error or EOF (the PTY was destroyed).

Cross-reference: the test passes `register_outbound_control(connection_owner,
outbound_tx)` at line 1651 — whatever the production reader task does to
emit envelopes must go through the same
`OUTBOUND_CONTROL_SENDERS` registry keyed by `connection_owner` (the
`_session_id: Uuid` parameter of `handle_multiplexed_inbound`, currently
unused at line 422).

**Note on the `_session_id` parameter:** line 422 discards the caller's
connection owner Uuid into `_session_id`. The reader-spawn fix will need to
capture this — rename to `connection_owner` and thread it into the spawned
task.

---

## Failure 5 — `test_sess018_terminal_input_writes_to_pty` (HANG)

**Observed:** Test never completes; must be killed via `timeout`.

**What the test expects:**
A `terminal:input` envelope carries a base64-encoded `"echo
SESS018_SENTINEL_OUTPUT\n"`. The bridge must decode the base64 and write the
bytes to the PTY master. The shell's echo and the `echo` command's output then
appear on the reader side — the test reads from a cloned reader and asserts
the sentinel string appears.

**Why it hangs:** the stub at line 624-631 returns `Ok(None)` for
`TerminalInput`, so nothing is ever written to the PTY. The test then blocks
forever on `reader.read(&mut buf)` because the shell is sitting on an empty
stdin with no input to echo back. There is no timeout in the test's read loop.

**Fix required:**
1. **Primary fix:** Wire `TerminalInput { terminal_id, base64 }` to
   `PtyRegistry::write_terminal_input` (method probably already exists; if
   not, add one that locks the master and writes).
2. **Defensive fix (recommended):** Add a `tokio::time::timeout` around the
   `reader.read` loop in the test itself (similar to test 4's pattern at
   `bridge_relay.rs:1683-1694`) so a regression becomes a clean assertion
   failure instead of an infinite hang.

---

## Cross-cutting observations

### The `_session_id: Uuid` parameter at line 422

`handle_multiplexed_inbound`'s second parameter is documented as "the
connection owner" (compare to `register_outbound_control(connection_owner,
outbound_tx)` in the test at line 1651), but it's captured as `_session_id` and
ignored throughout the function. Several of the fixes above require it — don't
keep discarding it.

### `BRIDGE_SESSION_CONTEXTS` helpers already exist

The test invokes `crate::set_bridge_session_context(…)` and
`crate::remove_bridge_session_context(…)`, so the registry is already built.
The production code just needs a getter (likely already present — search for
`get_bridge_session_context` or `BRIDGE_SESSION_CONTEXTS.get` in the same
file).

### Don't touch `test_sess018_terminal_resize_updates_pty_size` or
### `test_sess018_session_input_without_context_falls_back_to_parameter_injector`

These pass on HEAD. The resize path may already be correctly wired elsewhere
(or the test may be weaker than the failing ones). Audit but don't regress
them.

### The `Envelope` helper methods

`Envelope::terminal_created` exists (used at line 582). You will likely need
`Envelope::terminal_destroyed` with the same shape. The test already asserts
the envelope shape explicitly at line 1619-1623, so match those fields:

```rust
service: Service::Terminal,
msg_type: "destroyed",
request_id: Some(<request_id>),
terminal_id: Some(<term_id>),
```

---

## Scope / boundary

**In scope:**
- Wire the four broken dispatcher arms listed above.
- Spawn the post-create PTY reader task.
- Add any missing `Envelope::terminal_*` helpers.
- Thread the `connection_owner` Uuid through to the reader task.
- Add the defensive `tokio::time::timeout` to the hanging test (Failure 5).

**Out of scope:**
- Any other behavioural changes to `bridge_relay.rs`.
- Anything in the compaction cluster (CMPCT-*).
- Refactoring `handle_multiplexed_inbound` into smaller helpers — nice to
  have but keep the diff minimal.

---

## Definition of done

```bash
cd codelet
cargo test --package codelet-tools --lib bridge_relay::
# All tests pass, including the 5 listed above.

cargo clippy --package codelet-tools --all-targets --tests -- -D warnings
# Clean.
```

And — critically — `cargo test` over the whole workspace must come back clean
with no leftover failures from this file.

---

## Provenance

This report was authored by the supervising agent (session
`735ac366-f69e-4ecd-a57c-4db103e5d136`) on 2026-04-16 after investigating
apparent test failures during validation of CMPCT-023. The failures were
initially misattributed to CMPCT-023 work; `git stash` over the in-flight
changes reproduced the same failures on HEAD, confirming they are pre-existing
and disjoint.
