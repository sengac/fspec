# AST Research — AMGR-017 Profile Instrumentation Sites

**Work unit**: AMGR-017 (Add profile action to AgentManager for Rust runtime diagnostics)
**Research date**: 2026-04-07
**Scope**: Identify existing Rust code structures that the new `profile` submodule must integrate with, and enumerate every hot-loop and channel site that needs `profile_scope!()` markers or `TrackedBroadcast`/`TrackedMpsc` wrappers during the implementing phase.

---

## 1. AgentManagerAction — The Extension Point

**Path**: `codelet/tools/src/agent_manager/types.rs` lines 43–95
**Research tool**: `Read /Users/rquast/projects/fspec/codelet/tools/src/agent_manager/types.rs`

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AgentManagerAction {
    Spawn { #[serde(default)] role: Option<String> },
    List,
    GetStatus { session_id: String },
    Close { session_id: String },
    Message { session_id: String, message: String, #[serde(default)] context: Option<Vec<ContextReference>> },
    SetRole { #[serde(default)] session_id: Option<String>, role: String },
    AwaitIdle { session_id: SessionIdParam, #[serde(default)] timeout: Option<u64> },
}
```

**Implementation note**: Adding `Profile { duration_secs: Option<u32>, top_n: Option<usize>, label_prefix: Option<String> }` as the eighth variant requires:
1. New enum variant with serde-tagged `"action": "profile"` discriminator
2. New `AgentManagerResult::Profiled { … }` variant for the result shape
3. No changes to `AgentManagerArgs` — the `#[serde(flatten)]` + `#[serde(tag = "action")]` pattern already handles new variants automatically

---

## 2. AgentManagerTool Dispatch Path

**Path**: `codelet/tools/src/agent_manager/mod.rs` lines 160–185
**Research tool**: `Read /Users/rquast/projects/fspec/codelet/tools/src/agent_manager/mod.rs`

```rust
async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    // HOOK-013 pre_tool_use check elided
    let result = match &args.action {
        AgentManagerAction::AwaitIdle { .. } => {
            execute_agent_manager_async(self.session_id, args.action).await
        }
        _ => execute_agent_manager(self.session_id, args.action),
    };
    serde_json::to_string_pretty(&result).map_err(…)
}
```

**Implementation note**: `Profile` is ASYNC (tokio::time::sleep(duration_secs).await), so dispatch must match on `Profile { .. }` first and route to the async path exactly like `AwaitIdle`:

```rust
AgentManagerAction::AwaitIdle { .. } | AgentManagerAction::Profile { .. } => {
    execute_agent_manager_async(self.session_id, args.action).await
}
_ => execute_agent_manager(self.session_id, args.action),
```

The JSON schema in `definition()` lines 94–156 also needs the `"profile"` enum value added to the `action` property plus new `duration_secs`/`top_n`/`label_prefix` properties.

---

## 3. JSON Schema Tag Enum Update Site

**Path**: `codelet/tools/src/agent_manager/mod.rs` line 100
**Research tool**: `Read /Users/rquast/projects/fspec/codelet/tools/src/agent_manager/mod.rs`

```rust
"enum": ["spawn", "list", "get_status", "close", "message", "set_role", "await_idle"],
```

Becomes:

```rust
"enum": ["spawn", "list", "get_status", "close", "message", "set_role", "await_idle", "profile"],
```

New parameter blocks needed alongside existing `"action"`, `"role"`, `"session_id"`, `"message"`, `"context"`, `"timeout"`:
- `"duration_secs": { "type": ["integer", "null"], "minimum": 1, "maximum": 60, "description": "…" }`
- `"top_n": { "type": ["integer", "null"], "minimum": 1, "maximum": 200, "description": "…" }`
- `"label_prefix": { "type": ["string", "null"], "description": "…" }`

Description must explicitly warn agents that the call BLOCKS for duration_secs — not a hang.

---

## 4. Handler Plumbing — execute_agent_manager_async

**Path**: `codelet/tools/src/agent_manager/handler.rs` lines 135–161
**Research tool**: `Read /Users/rquast/projects/fspec/codelet/tools/src/agent_manager/handler.rs`

```rust
pub async fn execute_agent_manager_async(
    session_id: Uuid,
    action: AgentManagerAction,
) -> AgentManagerResult {
    let handler = AGENT_MANAGER_ASYNC_HANDLERS.read()…get(&session_id).cloned();
    match handler {
        Some(h) => h(action, session_id).await,
        None => AgentManagerResult::Error { … },
    }
}
```

**Implementation note**: This function can be reused as-is for the `Profile` dispatch. The per-session `AgentManagerAsyncHandler` registered via `set_agent_manager_async_handler()` will need to grow a new match arm for `AgentManagerAction::Profile { duration_secs, top_n, label_prefix }` that calls into the new `ProfileSession::run(…)` function. The registration site is in `codelet/napi/src/agent_manager_handler.rs::create_async_handler` (not yet inspected in this research pass, but the handler construction pattern is consistent with `handle_await_idle`).

---

## 5. handle_await_idle — Instrumentation Target #1 (The Prime Suspect)

**Path**: `codelet/napi/src/agent_manager_handler.rs` lines 789–941
**Complexity**: cyclomatic complexity 38 (highest in the file)
**Research tool**: GraphSearch ast_search query="handle_await_idle"

Key hot-loop structure:

```rust
async fn handle_await_idle(calling_session_id, session_ids, timeout) -> AgentManagerResult {
    // Phase 1: validate sessions, collect broadcast receivers for non-idle ones
    // …
    // Phase 2: spawn join_set task per pending session watching broadcast
    let mut join_set = tokio::task::JoinSet::new();
    for (id, mut rx) in pending {
        join_set.spawn(async move {
            loop {                                              // ← HOT LOOP A
                match rx.recv().await {
                    Ok(chunk) => {
                        if let StreamChunk::SessionStateChange { state } = &chunk {
                            if *state == SessionState::Idle {
                                return (id, AwaitOutcome::Idle);
                            }
                        }
                    }
                    Err(RecvError::Closed) => return (id, AwaitOutcome::Destroyed),
                    Err(RecvError::Lagged(_)) => continue,       // ← SUSPECT: silent continue
                }
            }
        });
    }

    // Phase 3: race join_set against deadline and interrupt
    loop {                                                       // ← HOT LOOP B
        if let Some(dl) = deadline { … timeout check … }
        // Build the select based on what's available
        // (tokio::select! with notify, sleep, join_next)
    }
}
```

**Instrumentation sites required** (per rule [11]):
1. `handle_await_idle::outer_select_loop` — wraps the Phase 3 `loop { tokio::select! … }`
2. `handle_await_idle::per_session_recv_loop` — wraps the per-task `loop { match rx.recv().await … }` inside each join_set spawn
3. `handle_await_idle::lagged_continue` — a counter-only scope inside the `Err(Lagged(_)) => continue` branch so we can see how often the receiver falls behind during a profile window (this is the suspected runaway-spin signature)

---

## 6. spawn_subordinate_forwarding_task — Instrumentation Target #2

**Path**: `codelet/napi/src/agent_manager_handler.rs` lines 198–278
**Complexity**: cyclomatic complexity 10
**Research tool**: GraphSearch ast_search query="spawn_subordinate_forwarding_task"

Key hot-loop structure:

```rust
fn spawn_subordinate_forwarding_task(session_manager, parent_session_id, subordinate_id) {
    let mut sub_rx = subordinate_session.subscribe_to_stream();
    let sub_id = subordinate_id;

    tokio::spawn(async move {
        loop {                                                   // ← HOT LOOP
            match sub_rx.recv().await {
                Ok(chunk) => {
                    // Convert StreamChunk to JSON, inject _relay_session_id
                    let senders = codelet_tools::get_subordinate_chunk_senders(root_parent_id);
                    if senders.is_empty() { continue; }          // ← SUSPECT: silent continue
                    for tx in &senders {
                        let _ = tx.send((sub_id, chunk_json.clone()));
                    }
                }
                Err(RecvError::Closed) => break,
                Err(RecvError::Lagged(n)) => {
                    tracing::warn!(…);                           // ← logged but loop continues
                    // Continue receiving — we'll catch up
                }
            }
        }
    });
}
```

**Instrumentation sites required**:
1. `spawn_subordinate_forwarding_task::recv_loop` — wraps the `loop { match sub_rx.recv().await … }`
2. `spawn_subordinate_forwarding_task::empty_senders_continue` — counter-only scope inside the `if senders.is_empty() { continue }` branch (so we can tell if a subordinate is emitting chunks into the void)
3. `spawn_subordinate_forwarding_task::lagged_warn` — counter-only scope inside the `Err(Lagged(n))` branch

---

## 7. bridge_relay::relay_loop — Instrumentation Target #3

**Path**: `codelet/tools/src/bridge_relay.rs` lines 666–715
**Complexity**: cyclomatic complexity 5
**Research tool**: GraphSearch ast_search query="relay_loop"

Key hot-loop structure:

```rust
async fn relay_loop(session_id, url, mut stream_rx, input_injector, control_handler, command_emitter) {
    let mut reconnect_delay = Duration::from_secs(INITIAL_RECONNECT_DELAY_SECS);

    loop {                                                       // ← HOT LOOP (reconnect)
        match connect_and_relay(…).await {
            Ok(()) => { tracing::info!(…); break; }
            Err(e) => {
                tracing::warn!(…);
                update_connection_state(…).await;
                tokio::time::sleep(reconnect_delay).await;
                reconnect_delay = min(reconnect_delay * 2, MAX_RECONNECT_DELAY_SECS);
            }
        }
        // Check if bridge was removed
        if !mgr.connections.contains_key(&url) { break; }
    }
}
```

**Instrumentation sites required**:
1. `bridge_relay::relay_loop::outer` — wraps the reconnect loop
2. `bridge_relay::relay_loop::connect_attempt` — wraps each `connect_and_relay(…).await` call
3. `bridge_relay::connect_and_relay::inbound_for_loop` — (requires deeper read; the rule [11] mentions "connect_and_relay inbound for-loop")
4. `bridge_relay::connect_and_relay::outbound_select_control_recv` — (inner select! arm)
5. `bridge_relay::connect_and_relay::outbound_select_stream_recv` — (inner select! arm)
6. `bridge_relay::connect_and_relay::outbound_select_subordinate_recv` — (inner select! arm)
7. `bridge_relay::connect_and_relay::outbound_select_shutdown_recv` — (inner select! arm)

**Note**: `connect_and_relay` was not fully inspected in this research pass (file is 49412 bytes); further investigation is needed during the testing phase to confirm the exact `tokio::select!` arm layout before writing the `profile_scope!()` markers. This is noted as a known-unknown rather than a blocker.

---

## 8. OUTBOUND_CONTROL_SENDERS — Channel Wrapper Migration Target #1

**Path**: `codelet/tools/src/bridge_relay.rs` line 144
**Research tool**: GraphSearch ast_search query="OUTBOUND_CONTROL_SENDERS"

```rust
static OUTBOUND_CONTROL_SENDERS: once_cell::sync::Lazy<RwLock<HashMap<_, _>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));
```

**Implementation note**: The `HashMap` stores per-session senders (likely `mpsc::UnboundedSender<_>` or similar). During the implementing phase, each value needs to be wrapped in a `TrackedUnboundedMpsc<T>` so that the ChannelRegistry in `codelet/tools/src/profile/channels.rs` can enumerate active control channels during a profile window and report `sender_count`, `receiver_count`, `queued_at_end`, and `lagged_during_window`. The wrapper must register on construction (insert into HashMap) and unregister on Drop (HashMap remove).

---

## 9. supervisor_broadcast — Channel Wrapper Migration Target #2

**Path**: `codelet/napi/src/session_manager.rs` line 1502 (const), line 509 (construction site — inferred)
**Research tool**: GraphSearch ast_search query="supervisor_broadcast"

```rust
const SUPERVISOR_BROADCAST_CAPACITY: usize = 256;

// The actual construction site (inferred from test at line 1502):
let (tx, mut rx) = broadcast::channel::<StreamChunk>(SUPERVISOR_BROADCAST_CAPACITY);
```

**Implementation note**: Every session owns a `broadcast::Sender<StreamChunk>` with capacity 256. Subordinates, the TUI, the relay bridge, and `handle_await_idle` all subscribe. This is the most-contended channel in the system and the likeliest source of the 42K+ lagged count from the PROV-054 CPU spike investigation. Migrating to `TrackedBroadcast<StreamChunk>` at the construction site (session_manager.rs:509, not yet confirmed by Read) will give the `profile` action visibility into per-session broadcast health.

The wrapper must:
1. Register itself in the ChannelRegistry with a stable name `supervisor_broadcast_<session_uuid>`
2. Intercept `send()` to increment a tracked `total_sent` counter
3. Expose `sender.receiver_count()` and `tx.len()` (queue depth) for the profile result builder
4. Increment a `lagged_during_window` counter whenever a subscriber returns `RecvError::Lagged(n)` — requires wrapping the `Receiver` as well, or using tokio's `broadcast::Sender::receiver_count()` + metrics from the tokio_unstable runtime

---

## 10. Module Layout — codelet/tools/src/

**Path**: `codelet/tools/src/`
**Research tool**: `Ls /Users/rquast/projects/fspec/codelet/tools/src`

Existing submodules (all `pub mod`):
- `agent_manager/` — the target for dispatch changes
- `apply_patch/`, `blocklist/`, `deep_search/`, `facade/`, `graph_search/`, `schedule/`, `session_search/`, `stage_permissions/`, `unified_exec/`
- Flat .rs files: `astgrep.rs`, `bash.rs`, `bridge.rs`, `bridge_relay.rs` (49412 bytes), `fspec.rs`, `inject_summary.rs`, `lib.rs`, `pre_tool_hook.rs`, etc.

**Implementation note**: The new `profile/` submodule slots in alongside the other tool submodules. The layout from the architecture notes stands:

```
codelet/tools/src/profile/
├── mod.rs         # re-exports + profile_scope!() macro_rules!
├── registry.rs    # ProfileRegistry singleton, DashMap<&'static str, ScopeMetrics>, PROFILING_ACTIVE: AtomicBool
├── scope.rs       # ProfileScope RAII guard
├── session.rs     # ProfileSession::run() — time-bounded orchestrator with CAS gate + sleep
├── result.rs      # ProfileResult, ScopeReport, ChannelReport (serde)
└── channels.rs    # TrackedBroadcast, TrackedMpsc, TrackedUnboundedMpsc wrappers + ChannelRegistry
```

The `lib.rs` at `codelet/tools/src/lib.rs` needs a `pub mod profile;` line (not yet inspected).

---

## 11. codelet/napi/src/ — Scheduler Loop Targets

**Path**: `codelet/napi/src/scheduler/`
**Research tool**: `Ls /Users/rquast/projects/fspec/codelet/napi/src` (subdirectory exists)

Known targets from rule [11] (not inspected in detail this pass):
- `codelet/napi/src/scheduler/engine.rs::spawn_scheduler` — 30s tick loop
- `codelet/napi/src/scheduler/loop_store.rs::register_with_task_and_idle_check` — per-entry interval loop

These two files require inspection during the testing/implementing phase to confirm the exact loop structures and pick stable `profile_scope!()` label names. This is a known gap in the research pass — the two files are known-unknowns but their existence is confirmed.

---

## Summary of Instrumentation Inventory

| # | File | Line range | Scope label(s) | Kind |
|---|------|-----------|----------------|------|
| 1 | `codelet/napi/src/agent_manager_handler.rs` | 789-941 | `handle_await_idle::outer_select_loop`, `::per_session_recv_loop`, `::lagged_continue` | profile_scope! |
| 2 | `codelet/napi/src/agent_manager_handler.rs` | 198-278 | `spawn_subordinate_forwarding_task::recv_loop`, `::empty_senders_continue`, `::lagged_warn` | profile_scope! |
| 3 | `codelet/tools/src/bridge_relay.rs` | 666-715 | `relay_loop::outer`, `::connect_attempt` | profile_scope! |
| 4 | `codelet/tools/src/bridge_relay.rs` | (connect_and_relay, TBD) | `connect_and_relay::inbound_for_loop` + 4 select! arms | profile_scope! (TBD) |
| 5 | `codelet/napi/src/scheduler/engine.rs` | (spawn_scheduler, TBD) | `scheduler::tick_loop` | profile_scope! (TBD) |
| 6 | `codelet/napi/src/scheduler/loop_store.rs` | (register_with_task_and_idle_check, TBD) | `loop_store::interval_loop` | profile_scope! (TBD) |
| 7 | `codelet/tools/src/bridge_relay.rs:144` | — | `OUTBOUND_CONTROL_SENDERS` | TrackedUnboundedMpsc migration |
| 8 | `codelet/tools/src/bridge_relay.rs` (SUBORDINATE_CHUNK_SENDERS) | — | `SUBORDINATE_CHUNK_SENDERS` | TrackedUnboundedMpsc migration |
| 9 | `codelet/napi/src/session_manager.rs:509` (inferred) | — | `supervisor_broadcast_<session_id>` | TrackedBroadcast migration |

**Confirmed via AST**: sites 1, 2, 3, 7 (4 hot-loop sites, 2 channel sites, 3 scope variants for rows 1 and 2 → 9 concrete `profile_scope!()` markers)

**Known-unknown (deferred to implementing phase)**: sites 4, 5, 6 (4 more markers in `connect_and_relay`, 2 more in scheduler) — estimated 6 additional markers, bringing the total to ~15 sites as called out in the architecture notes.

---

## Connection to PROV-053 / PROV-054 CPU Spike Investigation

The two prime suspects from the CPU spike investigation (PID 1728 burning 305-990% CPU with 9 hot tokio workers) map directly onto instrumentation sites #1 and #2 above:

- **Site #1** (`handle_await_idle` Phase 3 loop + per-session `rx.recv()` loops) matches the "handle_await_idle outer loop + join_next branches" smoking gun from session turns 0-211
- **Site #2** (`spawn_subordinate_forwarding_task::recv_loop` with `Err(Lagged(_)) => continue`) matches the "SUBORDINATE_CHUNK_SENDERS recv loop may not exit cleanly if session destroyed while receivers persist" smoking gun

Once the profile action lands and these sites are instrumented, running `AgentManager profile duration_secs=10` during a reproduction of the CPU spike will show exactly which of the two loops is spinning, and the `lagged_during_window` counter on the `supervisor_broadcast_<session>` channel will confirm or rule out the 42K-lagged hypothesis.
