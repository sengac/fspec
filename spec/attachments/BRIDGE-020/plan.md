# BRIDGE-020 — Implementation Plan

## Goal

Wire the 4 unwired dispatcher arms and the missing PTY reader task inside
`handle_multiplexed_inbound` at `codelet/tools/src/bridge_relay.rs:420-636`
so that all `test_sess018_*` tests pass on a clean `cargo test` run.

Read `spec/attachments/BRIDGE-020/failure-report.md` first — it contains the
per-failure root cause and expected behaviour. This file is the implementation
playbook.

**All helpers already exist** — this task is pure wiring, no new APIs.

## Existing API Surface (confirmed)

| Helper | Location | Notes |
|---|---|---|
| `crate::get_bridge_session_context(sid) -> Option<Arc<BridgeSessionContext>>` | `bridge_handler.rs:110` | Holds `input_injector`, `control_handler`, etc. |
| `crate::destroy_terminal(&registry, &terminal_id).await -> Result<(), String>` | `bridge_pty.rs:207` | Kills child + removes from registry |
| `crate::write_terminal_input(&entry, &bytes).await -> Result<(), String>` | `bridge_pty.rs:192` | Decodes nothing — caller must base64-decode first |
| `crate::resize_terminal(&entry, cols, rows).await -> Result<(), String>` | `bridge_pty.rs:167` | |
| `Envelope::terminal_destroyed(&instance_id, &request_id, &terminal_id)` | `bridge_multiplexed.rs:168` | |
| `Envelope::terminal_data(&instance_id, &terminal_id, &base64)` | `bridge_multiplexed.rs:134` | |
| `query_pty_registry() -> Option<Arc<PtyRegistry>>` | same file, private | Already used by TerminalCreate arm |
| `OUTBOUND_CONTROL_SENDERS` | same file, private static | Snapshot-and-send pattern at `broadcast_metadata_update:263-278` |

## Changes Required

### 1. Rename `_session_id: Uuid` → `connection_owner: Uuid`

Line 422. Thread it into the reader task (spawn step 5) so the task can look
up `OUTBOUND_CONTROL_SENDERS[connection_owner]`.

### 2. SessionInput arm (lines 434-450) — per-session routing

```rust
InboundAction::SessionInput { session_id, message, images } => {
    let injected = /* unchanged InjectedInput construction */;

    // SESS-018: route by envelope's session_id if a per-session context is registered
    let routed = Uuid::parse_str(&session_id)
        .ok()
        .and_then(crate::get_bridge_session_context)
        .map(|ctx| {
            (ctx.input_injector)(injected.clone());
            true
        })
        .unwrap_or(false);

    if !routed {
        input_injector(injected);
    }
    Ok(None)
}
```

Preserve the fallback so `test_sess018_session_input_without_context_falls_back_to_parameter_injector`
(which already passes) stays green.

### 3. SessionControl arm (lines 452-463) — per-session routing

Same shape as SessionInput. If `get_bridge_session_context(sid)` returns a
context AND `ctx.control_handler.is_some()`, dispatch to the per-session
handler. Otherwise fall back to the parameter `control_handler`.

### 4. Split the combined stub (lines 624-631) into three arms

Replace the combined arm with three separate handlers. Match the shapes in
`InboundAction` at `bridge_multiplexed.rs:309-323`:

```rust
InboundAction::TerminalInput { terminal_id, base64_data } => {
    let Some(registry) = query_pty_registry() else {
        tracing::warn!("terminal:input received but no PtyRegistry registered");
        return Ok(None);
    };
    let Some(entry) = registry.get(&terminal_id) else {
        tracing::warn!("terminal:input for unknown terminal {}", terminal_id);
        return Ok(None);
    };
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&base64_data) else {
        tracing::warn!("terminal:input base64 decode failed for {}", terminal_id);
        return Ok(None);
    };
    if let Err(e) = crate::write_terminal_input(&entry, &bytes).await {
        tracing::warn!("terminal:input write failed: {}", e);
    }
    Ok(None)
}

InboundAction::TerminalResize { terminal_id, cols, rows } => {
    let Some(registry) = query_pty_registry() else { return Ok(None); };
    let Some(entry) = registry.get(&terminal_id) else { return Ok(None); };
    if let Err(e) = crate::resize_terminal(&entry, cols, rows).await {
        tracing::warn!("terminal:resize failed: {}", e);
    }
    Ok(None)
}

InboundAction::TerminalDestroy { terminal_id, request_id } => {
    let metadata = get_instance_metadata();
    let instance_id = metadata.name;
    let Some(registry) = query_pty_registry() else {
        tracing::warn!("terminal:destroy received but no PtyRegistry registered");
        // Still respond so the dashboard doesn't hang
        return Ok(Some(Envelope::terminal_destroyed(&instance_id, &request_id, &terminal_id)));
    };
    if let Err(e) = crate::destroy_terminal(&registry, &terminal_id).await {
        tracing::warn!("terminal:destroy failed for {}: {}", terminal_id, e);
    }
    Ok(Some(Envelope::terminal_destroyed(&instance_id, &request_id, &terminal_id)))
}
```

### 5. Spawn the PTY reader task inside TerminalCreate (line 577 area)

After the successful `Ok((terminal_id, entry)) => { ... }` branch sends the
`terminal:created` response, spawn a reader BEFORE returning:

```rust
Ok((terminal_id, entry)) => {
    tracing::info!("terminal:create handled — new terminal {}", terminal_id);
    spawn_pty_reader_task(connection_owner, instance_id.clone(), terminal_id.clone(), entry);
    Ok(Some(Envelope::terminal_created(&instance_id, &request_id, &terminal_id)))
}
```

Add a new module-private helper at the bottom of the `// ── Inbound processing` block:

```rust
/// Spawn a background reader that drains a PTY master and emits
/// `terminal:data` envelopes through OUTBOUND_CONTROL_SENDERS keyed by the
/// connection owner.
///
/// Uses `spawn_blocking` because portable-pty readers are synchronous.
/// The inner loop exits on EOF or read error (PTY destroyed).
fn spawn_pty_reader_task(
    connection_owner: Uuid,
    instance_id: String,
    terminal_id: String,
    entry: Arc<crate::PtyEntry>,
) {
    tokio::spawn(async move {
        // Clone the blocking reader from the master (must be sync — do this up front).
        let reader = {
            let master = entry.master.lock().await;
            match master.try_clone_reader() {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("failed to clone PTY reader for {}: {}", terminal_id, e);
                    return;
                }
            }
        };

        // Move the blocking read loop to a blocking thread.
        let _ = tokio::task::spawn_blocking(move || {
            use base64::Engine;
            use std::io::Read;

            let mut reader = reader;
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&buf[..n]);
                        let env = Envelope::terminal_data(&instance_id, &terminal_id, &b64);

                        // Snapshot senders for this connection_owner and fan out.
                        let senders: Vec<OutboundControlTx> = match OUTBOUND_CONTROL_SENDERS.read() {
                            Ok(guard) => guard
                                .get(&connection_owner)
                                .cloned()
                                .unwrap_or_default(),
                            Err(_) => break,
                        };
                        if senders.is_empty() {
                            // Connection owner gone — terminal is orphaned, drop the loop.
                            break;
                        }
                        for tx in senders {
                            let _ = tx.send(env.clone());
                        }
                    }
                    Err(_) => break,
                }
            }
            tracing::debug!("PTY reader for {} exiting", terminal_id);
        })
        .await;
    });
}
```

**Envelope::clone** — confirm `Envelope` is `Clone`. It is: `#[derive(Debug, Clone, Serialize, Deserialize)]` at `bridge_multiplexed.rs` (already used by `broadcast_metadata_update`).

### 6. Defensive timeout in the hanging test

The failure report recommends adding a `tokio::time::timeout` around the
`reader.read` loop in `test_sess018_terminal_input_writes_to_pty` so a future
regression surfaces as a clean failure instead of a CI hang. This is OPTIONAL
— once the handler is wired correctly, the test will pass. Skip unless you
can do it as a small, obviously-correct addition.

## Files Touched

- `codelet/tools/src/bridge_relay.rs` — only file that needs editing

## Out of Scope

- Any other handler behaviour in `bridge_relay.rs`
- Compaction system (CMPCT-*)
- Refactoring `handle_multiplexed_inbound` into smaller helpers
- Changes to `bridge_pty.rs`, `bridge_multiplexed.rs`, or `bridge_handler.rs`
  (all needed helpers already exist)

## Verification Checklist (supervisor will run)

```bash
cd codelet
cargo test --package codelet-tools --lib bridge_relay::
# All 5 previously-failing tests pass.

cargo clippy --package codelet-tools --all-targets --tests -- -D warnings
# Clean.

cargo test --workspace
# No regressions.
```

## Status Hand-off

- Start: `backlog` (will be moved to `implementing` by the supervisor)
- Subordinate: write the code, mark `validating` when you believe it's done
- Supervisor: runs tests, moves to `done` on green.
