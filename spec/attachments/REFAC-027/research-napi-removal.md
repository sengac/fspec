# Research: removing the legacy codelet-napi crate (REFAC-027)

## Why it can go
- rust/napi/src/lib.rs: "This module exposes rust's Rust AI agent functionality to Node.js via NAPI-RS. It enables fspec's Ink/React TUI to serve as the frontend for codelet." The Ink/React TUI has been replaced by the pure-Rust TUI (codelet-fspec-tui).
- `cargo metadata`: none of the other 20 workspace members depends on codelet-napi (no reverse dependencies).
- .github/workflows/release.yml builds only `-p codelet-fspec`; no CI job builds codelet-napi.
- Nothing in the repository (bridge/, scripts/, other crates) consumes the @sengac/codelet-napi npm package.

## The separation does not depend on napi
- Engine: codelet-core, codelet-sessions, codelet-agent-loop, codelet-tools, codelet-providers.
- Service + transports: codelet-rpc (FspecService), codelet-rpc-server (WebSocket daemon/client), codelet-rpc-embedded (in-process).
- Client: codelet-fspec-tui behind the FspecBackend trait; binary: codelet-fspec (combined / daemon / client).
- The no_napi_dependency tests (agent-loop, core, fspec, fspec-tui, graph, ...) only forbid an arrow to a crate that will no longer exist.

## Size
- rust/napi: 146 .rs files (100 of them tests), 2.5M.
- rpc-types `napi` feature (cfg_attr napi(object) on wire types): 69 attributes; only codelet-napi enables it.

## Tests outside rust/napi that read napi source paths (retire or retarget)
```
agent-loop/tests/bug238_deep_search_severity.rs
agent-loop/tests/cmpct041_pre_compaction_basis_test.rs
agent-loop/tests/prov160_napi_session_recovery.rs
agent-loop/tests/prov160_napi_turn_routing.rs
agent-loop/tests/rpc082_role_injection.rs
agent-loop/tests/rpc083_tool_registry.rs
agent-loop/tests/support/cont011_delegation.rs
agent-loop/tests/wt002_footer_poller_parity.rs
cli/tests/cmpct050_orphan_tool_call_recovery.rs
common/src/debug_capture/tests/part_2.rs
core/tests/fixtures/indexing_legacy_v1.rs
core/tests/legacy_compaction_cleanup_test.rs
core/tests/message_store_lifted_test.rs
core/tests/work_units_watcher_test.rs
core/tests/work_units_write_test.rs
fspec-tui/tests/checkpoint_counts_rpc015.rs
fspec-tui/tests/cont007_live_continue_status_test.rs
fspec-tui/tests/cont008_goal_back_sync_test.rs
fspec-tui/tests/core_history_lift_rpc025.rs
fspec-tui/tests/loop_store_lift_rpc059.rs
fspec-tui/tests/move_work_unit_rpc017.rs
fspec-tui/tests/rpc026_cross_transport_parity.rs
fspec-tui/tests/scheduler_engine_lift_rpc058.rs
fspec-tui/tests/source_shape_rpc015.rs
fspec-tui/tests/source_shape_rpc017.rs
fspec-tui/tests/source_shape_rpc017/shared_helpers.rs
fspec-tui/tests/source_shape_rpc018.rs
fspec-tui/tests/source_shape_rpc018/surface_invariants.rs
fspec-tui/tests/source_shape_rpc025.rs
graph/tests/codelet_graph_crate_lift.rs
graph/tests/graph_reset_test.rs
napi/tests/blob_store_lift_shim_test.rs
napi/tests/deprecate_old_graph_test.rs
napi/tests/global_chunk_callback_napi_test.rs
napi/tests/learnings_query_interface_test.rs
napi/tests/message_envelope_lift_shim_test.rs
napi/tests/message_store_lift_shim_test.rs
napi/tests/navigation_hierarchy_test.rs
napi/tests/rpc421_honest_ack_test.rs
napi/tests/session_bindings_shape.rs
napi/tests/session_bindings_smoke.rs
napi/tests/session_store_lift_shim_test.rs
providers/tests/copilot_rig_agent_test.rs
providers/tests/copilot_select_model_stale_cache_test.rs
rpc-embedded/tests/architecture_invariants.rs
rpc-embedded/tests/rpc_007_source_shape.rs
rpc-types/tests/bug146_napi_attribute_scoping.rs
rpc-types/tests/rpc036_widen_types.rs
sessions/tests/background_session_shape.rs
sessions/tests/background_session_shape/broadcast.rs
sessions/tests/background_session_shape/isolation.rs
sessions/tests/background_session_shape/subprocess.rs
sessions/tests/background_session_shape/typescript.rs
sessions/tests/bug228_shared_dispatch_wiring.rs
sessions/tests/mcp_injection_source_shape.rs
sessions/tests/mcp_injection_source_shape/consumer_and_surface.rs
sessions/tests/prov144_max_images_wiring_shape.rs
sessions/tests/rpc073_list_providers_wiring.rs
sessions/tests/session_manager_shape.rs
sessions/tests/wt004_session_tool_callbacks.rs
tools/tests/block_notifications_test.rs
```

## Source files outside rust/napi mentioning napi paths (mostly lift-history doc comments)
```
agent-loop/src/agent_manager_handler.rs
agent-loop/src/background_output.rs
agent-loop/src/bridges.rs
agent-loop/src/dispatch.rs
agent-loop/src/inject_summary_handler.rs
agent-loop/src/lib.rs
agent-loop/src/stream_chunk_json.rs
agent-loop/src/thinking_config.rs
agent-loop/src/thinking_level_detection.rs
common/src/debug_capture/tests/part_2.rs
common/src/file_lock.rs
core/src/persistence/blob_processing.rs
core/src/persistence/blob.rs
core/src/persistence/history.rs
core/src/persistence/lazy_init_tests.rs
core/src/persistence/manifest_messages.rs
core/src/persistence/messages.rs
core/src/persistence/mod.rs
core/src/persistence/tests.rs
core/src/scheduler/agent_job.rs
core/src/scheduler/catch_up.rs
core/src/scheduler/crud.rs
core/src/scheduler/mod.rs
core/src/scheduler/trigger.rs
core/src/session_manager_handle.rs
core/src/work_units.rs
fspec/src/blocklist_init_tests.rs
fspec/src/common.rs
graph/src/lib.rs
napi/src/agent_manager_handler.rs
napi/src/bridges.rs
napi/src/graph.rs
napi/src/interjection.rs
napi/src/lib.rs
napi/src/scheduler/mod.rs
napi/src/session_bindings.rs
napi/src/session_hooks.rs
napi/src/types.rs
providers/src/custom/model_limits.rs
rpc-types/src/chrome.rs
rpc-types/src/chunk_support.rs
rpc-types/src/dialogs.rs
rpc-types/src/lib.rs
rpc-types/src/operations.rs
rpc-types/src/sessions.rs
rpc-types/src/stream_chunk.rs
rpc/src/log_layer.rs
sessions/src/background_session.rs
sessions/src/chain_of_command.rs
sessions/src/credentials/mod.rs
sessions/src/footer_poller.rs
sessions/src/handle_impl/handle_impl_2.rs
sessions/src/handle_impl/handle_impl_3.rs
sessions/src/session_manager.rs
sessions/src/session_tool_callbacks.rs
tools/src/deep_search/mod.rs
tools/src/graph_search/mod.rs
tools/src/profile/diagnostics_tests.rs
```

## Cargo manifests mentioning napi
```
agent-loop/Cargo.toml:10:# This crate MUST NOT depend on codelet-napi — the
agent-loop/Cargo.toml:11:# tests/no_napi_dependency.rs binary enforces that contract.
agent-loop/Cargo.toml:41:# Learnings extraction code (was rust/napi/src/graph/). The agent loop
core/Cargo.toml:18:# never back on codelet-core or codelet-napi) — mirrors the
core/Cargo.toml:76:# same crate codelet-napi already uses; not yet in workspace.dependencies.
core/Cargo.toml:89:# RPC-067: shared no-codelet-napi dependency-rule helpers used by
core/Cargo.toml:90:# tests/no_napi_dependency.rs.
graph/Cargo.toml:7:description = "Embedded graph databases for the AST and Learnings dual-graph architecture (RPC-092 lift from rust/napi/src/graph/)"
graph/Cargo.toml:9:# RPC-092: do NOT inherit workspace lints. The source crate (codelet-napi)
graph/Cargo.toml:64:# RPC-067: shared no-codelet-napi dependency-rule helpers used by
graph/Cargo.toml:65:# tests/no_napi_dependency.rs.
git/Cargo.toml:26:# struct so the additive `napi::count_checkpoints` export and the new
fspec/Cargo.toml:109:# RPC-067: shared no-codelet-napi dependency-rule helpers used by
fspec/Cargo.toml:110:# tests/no_napi_dependency.rs.
Cargo.toml:30:    "napi",
Cargo.toml:40:    # tests. Lives outside the napi/sessions arrow graph so any test
Cargo.toml:76:# RPC-039/RPC-040; downstream crates (codelet-fspec, codelet-napi) start
Cargo.toml:81:# that needs to enforce a no-codelet-napi (or any other forbidden-arrow)
Cargo.toml:237:# (the lifted WorkUnitsWatcher) and rust/napi (the legacy ThreadsafeFunction
Cargo.toml:317:# meaningful attribution instead of phantom `_napi_register_module_v1`
Cargo.toml:324:# `/var/folders/.../T/fspec-sea-<pid>/codelet-napi.node`. macOS's
Cargo.toml:326:# `codelet-napi.node.dSYM` bundle in that temp dir — which never exists
Cargo.toml:347:# that the macOS-SEA pprof contract requires for the `codelet-napi` `.node`
Cargo.toml:363:# `[profile.release]` where the napi `.node` artifact needs it.
Cargo.toml:370:# Local profiling builds (and any napi build) still use `--release` so the
Cargo.toml:391:#   1. `codelet-napi` has 89 integration test files in tests/*.rs, each
Cargo.toml:397:#   3. Shape tests (rust/napi/tests/session_bindings_shape.rs) spawn
Cargo.toml:398:#      nested `cargo test --no-run -p codelet-napi` from within an
rpc/Cargo.toml:24:# arrow rpc → napi remains forbidden (enforced by the source-shape
rpc/Cargo.toml:35:# primitives the napi *_oauth.rs wrappers use (claude_auth path / codex
rpc/Cargo.toml:37:# is permitted (rpc → the napi crate remains forbidden); this keeps the clear
rpc/Cargo.toml:38:# logic napi-direct in spirit without that forbidden dependency.
sessions/Cargo.toml:16:# both the Rust `fspec` binary and the `codelet-napi` adapter consume via
sessions/Cargo.toml:18:# crates that the current `rust/napi/src/session_manager.rs` already
sessions/Cargo.toml:20:# napi/napi-derive entry.
sessions/Cargo.toml:27:# codelet-rpc-types is depended on WITHOUT the `napi` feature so that the
sessions/Cargo.toml:28:# transitive napi crate is excluded from the codelet-sessions graph. The
sessions/Cargo.toml:29:# forbidden arrow `sessions → napi` is enforced by
sessions/Cargo.toml:77:# RPC-067: shared no-codelet-napi dependency-rule helpers used by
sessions/Cargo.toml:78:# tests/no_napi_dependency.rs.
fspec-tui/Cargo.toml:30:# depend on codelet-core/codelet-napi, so this adds no forbidden arrow.
fspec-tui/Cargo.toml:116:# RPC-067: shared no-codelet-napi dependency-rule helpers used by
fspec-tui/Cargo.toml:117:# tests/no_napi_dependency.rs.
fspec-tui/Cargo.toml:141:# agent-loop is napi-free (its no_napi_dependency contract), so no cycle.
test-helpers/Cargo.toml:8:description = "Shared test-only helpers for the codelet workspace (dependency-rule regression tests, source-scan utilities). Lives outside the napi/sessions arrow graph so any test crate can consume it without fear of pulling forbidden dependencies."
rpc-types/Cargo.toml:12:# When enabled, applies the NAPI-RS `#[napi(object)]` decoration to types
rpc-types/Cargo.toml:14:# preserved without redefining types in `rust/napi`. This is the
rpc-types/Cargo.toml:16:# was a wrapper struct in `rust/napi`).
rpc-types/Cargo.toml:18:# Default builds remain free of any napi/tokio/tarpc dependency, satisfying
rpc-types/Cargo.toml:20:napi = ["dep:napi", "dep:napi-derive"]
rpc-types/Cargo.toml:27:napi = { version = "3", default-features = false, features = ["napi6"], optional = true }
rpc-types/Cargo.toml:28:napi-derive = { version = "3", optional = true }
rpc-types/Cargo.toml:31:# RPC-067: shared no-codelet-napi dependency-rule helpers used by
rpc-types/Cargo.toml:32:# tests/no_napi_dependency.rs.
tools/Cargo.toml:80:napi = "3.8.2"
tools/Cargo.toml:81:napi-derive = "3.5.1"
```

## Spec
- 335 feature files mention napi; only scenarios asserting napi behaviour need change, the rest are history in descriptions.
