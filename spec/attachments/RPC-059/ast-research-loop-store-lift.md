# RPC-059 AST Research — Loop store lift surface area

## Goal

Catalogue every AST-level call site that has to change for RPC-059 (lift `loop_store` from `codelet/napi/src/scheduler/loop_store.rs` into `codelet/core/src/loops/mod.rs` + new RPC surface `loop_add` / `loop_cancel` / `loop_list`).

## Methodology

Used `AstGrep` against `codelet/napi`, `codelet/core`, `codelet/sessions`, `codelet/fspec-tui`, `codelet/rpc-types`, `codelet/rpc` to find all functions, structs, and trait methods that touch the loop surface.

## Findings

### A. NAPI loop bindings (to be reduced to thin shims)

`AstGrep --pattern='pub async fn $NAME($$$ARGS) -> Result<$RET> { $$$BODY }' --path=codelet/napi/src/session_bindings.rs` returns:

- `pub async fn loop_register(session_id, loop_id, prompt, interval_seconds) -> Result<()>` at session_bindings.rs:2192
- `pub async fn loop_cancel(loop_id: String) -> Result<bool>` at session_bindings.rs:2255
- `pub async fn loop_list(session_id: String) -> Result<String>` at session_bindings.rs:2263

These remain as `#[napi]` async functions but `loop_register` will become a thin shim that constructs a `LoopEntry` (now from `codelet_core::loops`) and registers it on the shared singleton.

### B. NAPI loop store impl block

`AstGrep --pattern='impl LoopStore { $$$BODY }' --path=codelet/napi/src/scheduler`:

- One impl block at `codelet/napi/src/scheduler/loop_store.rs:58`. All methods move verbatim into `codelet/core/src/loops/mod.rs` — they have no `napi` references.

### C. codelet-core scheduler surface (precedent — already lifted by RPC-058)

`Grep "pub use codelet_core::scheduler"` in `codelet/napi/src/scheduler/mod.rs`:

- mod.rs already re-exports the scheduler engine. Same pattern applies for loops: add a `pub use codelet_core::loops::{LoopEntry, LoopStore, IdleCheckFn};` line and an empty `pub mod loop_store { pub use codelet_core::loops::*; }` shim for absolute-path compatibility.

### D. SessionManagerHandle (codelet/core/src/session_manager_handle.rs)

`Grep "fn schedule_add|fn schedule_list|fn schedule_pause"`:

- The RPC-058 trait methods sit at lines 666, 671, 676 (default impls) and 1790, 1804, 1812 (stub overrides).
- New methods `loop_add`, `loop_cancel`, `loop_list` slot into the same section. Per-call counters (`loop_add_calls`, `loop_cancel_calls`, `loop_list_calls`) and stub state (`Arc<Mutex<Vec<RegisteredLoop>>>`) follow the RPC-058 layout exactly.

### E. FspecService (codelet/rpc/src/lib.rs)

`Grep "async fn schedule_add"`:

- trait declaration block lines 407–419; impl block lines 1511–1556.
- Three new async fn declarations + three new impl blocks follow the same pattern.

### F. FspecBackend trait + transports

`Grep "schedule_add|schedule_list"` across `codelet/fspec-tui/src/transport/`:

- Trait default impls in `mod.rs:573-595`.
- Embedded forwarders in `embedded.rs:631-660`.
- WebSocket forwarders in `websocket.rs:1011-1055`.

Three new methods slot into the same layout.

### G. codelet-sessions handle_impl

`Grep "fn schedule_add|fn schedule_list|fn schedule_pause"` in `codelet/sessions/src/handle_impl.rs`:

- Section starts at line 1084, ends at line 1135.
- Three new impl blocks follow. Each resolves `self.get_session(session_id.value.as_str())` to grab a `BackgroundSession` Arc, then wires `on_fire` / `idle_check` closures (capturing the session) and registers on `codelet_core::loops::LoopStore::instance()`.

### H. Slash command surface

`Grep "SlashCommandAction::Schedule|SlashCommandAction::Loop"` in `codelet/fspec-tui/src`:

- `SlashCommandAction::Loop` is already declared in `views/agent/slash_commands.rs:39, 65, 156-159`.
- `dispatch_rpc020.rs` currently falls through to the catch-all "[notice] /loop not yet implemented" arm. RPC-059 replaces this with a `self.handle_slash_loop_help()` call (same pattern as `SlashCommandAction::Schedule` at line 130-134).
- A new `dispatch_rpc059.rs` file mirrors `dispatch_rpc058.rs`.

### I. slash_parser interception

`Grep "ScheduleSubcommand" in slash_parser.rs`:

- Lines 16, 45 declare the variant + carry it.
- Lines 106-108 detect `/schedule …` prefix and route through `parse_schedule_command`.
- A new `LoopSubcommand(LoopSubcommand)` variant + matching `/loop …` arm mirror this.

### J. Action enum (components/mod.rs)

`Grep "ScheduleSubcommandParsed"`:

- Line 678 declares `Action::ScheduleSubcommandParsed(ScheduleSubcommand)`.
- A new `LoopSubcommandParsed(LoopSubcommand)` variant follows.

### K. dispatch.rs orchestrator

`Grep "try_dispatch_rpc058"`:

- Line 291 in the catch-all chain: `… || self.try_dispatch_rpc058(&action)`.
- Extended to `… || self.try_dispatch_rpc058(&action) || self.try_dispatch_rpc059(&action)`.

### L. MockBackend (tests/common/mod.rs)

`Grep "schedule_add_result|schedule_add_calls"`:

- ~50 lines of seeded results + per-call counters + trait impls between lines 515-2510.
- A parallel block of ~50 lines for `loop_add_result` / `loop_cancel_result` / `loop_list_result` and three new trait impl arms.

### M. cleanup_session_loops hook

`Grep "cleanup_session_loops"` in `codelet/sessions/src/session_manager.rs`:

- Line 109 trait declaration, line 140 noop impl, line 934 call site.
- The NAPI hooks impl in `codelet/napi/src/session_hooks.rs` already calls `LoopStore::instance().remove_for_session(uuid).await` — this continues to work via the re-export shim.
- The codelet-sessions `NoopSessionManagerHooks` impl needs no change (loops are auto-cancelled by tokio task abort when the LoopStore drops references).

## Files to touch (count: 14)

| # | File | Action |
|---|------|--------|
| 1 | `codelet/core/src/loops/mod.rs` | NEW — full loop_store contents lifted from NAPI |
| 2 | `codelet/core/src/lib.rs` | Add `pub mod loops;` |
| 3 | `codelet/napi/src/scheduler/mod.rs` | Replace `pub mod loop_store; pub use loop_store::LoopStore;` with re-export + shim module |
| 4 | `codelet/napi/src/scheduler/loop_store.rs` | DELETE |
| 5 | `codelet/rpc-types/src/lib.rs` | Add `RegisteredLoop` wire struct |
| 6 | `codelet/core/src/session_manager_handle.rs` | Add 3 trait methods + stub overrides + counters + seeders |
| 7 | `codelet/rpc/src/lib.rs` | Add 3 service fns + 3 service-impl blocks |
| 8 | `codelet/fspec-tui/src/transport/mod.rs` | Add 3 trait defaults |
| 9 | `codelet/fspec-tui/src/transport/embedded.rs` | Add 3 forwarders |
| 10 | `codelet/fspec-tui/src/transport/websocket.rs` | Add 3 forwarders |
| 11 | `codelet/sessions/src/handle_impl.rs` | Add 3 impl blocks |
| 12 | `codelet/fspec-tui/src/app/loop_parser.rs` | NEW — parser + LoopSubcommand enum |
| 13 | `codelet/fspec-tui/src/app/slash_parser.rs` | Add LoopSubcommand variant + /loop arm |
| 14 | `codelet/fspec-tui/src/app/dispatch_rpc059.rs` | NEW — slash dispatch + notice formatters |
| 15 | `codelet/fspec-tui/src/app/mod.rs` | Add `mod dispatch_rpc059;` |
| 16 | `codelet/fspec-tui/src/app/dispatch.rs` | Extend catch-all chain |
| 17 | `codelet/fspec-tui/src/app/dispatch_rpc020.rs` | Replace `SlashCommandAction::Loop` notice fallback with `self.handle_slash_loop_help()` |
| 18 | `codelet/fspec-tui/src/components/mod.rs` | Add `Action::LoopSubcommandParsed` |
| 19 | `codelet/fspec-tui/tests/common/mod.rs` | Extend MockBackend with loop seeders + counters + trait impls |

## Test files (count: 4)

| # | File | Purpose |
|---|------|---------|
| 1 | `codelet/fspec-tui/tests/loop_store_lift_rpc059.rs` | Source-shape pin for the lift |
| 2 | `codelet/fspec-tui/tests/source_shape_rpc059.rs` | Source-shape pin for the RPC surface |
| 3 | `codelet/fspec-tui/tests/rpc059_cross_transport_parity.rs` | Cross-transport parity for the 3 new methods |
| 4 | `codelet/fspec-tui/tests/loop_dispatch_rpc059.rs` | Parser + dispatch end-to-end |

## Out of scope

- Reverting NAPI's `loop_register` semantics. Only the storage location changes; the JS-facing API stays byte-identical via the re-export shim.
- The /loop view (TS uses a notice, not a view).
- The scheduler engine lift (done in RPC-058).
