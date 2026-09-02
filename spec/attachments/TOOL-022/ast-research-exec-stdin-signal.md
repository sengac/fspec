# TOOL-022 AST Research — deterministic exec-stdin signal & overlay

Date: 2026-09-02. Performed with the AstGrep tool during discovery, in support of the
deterministic (vtcode-aligned, no-heuristic) redesign.

## 1. UnifiedExecResult construction sites (codelet-tools)

Pattern: `UnifiedExecResult { $$$FIELDS }` in `rust/tools/src` — 7 sites:

| File | Line | Branch | quiet_seconds treatment (P1) |
|---|---|---|---|
| `unified_exec/tool.rs` | 189 | `handle_run` — process exited | `None` |
| `unified_exec/tool.rs` | 213 | `handle_run` — still running | `Some(quiet_secs)` + steering line |
| `unified_exec/tool.rs` | 293 | `handle_list` | `None` |
| `unified_exec/tool.rs` | 325 | `handle_close` | `None` |
| `unified_exec/tool.rs` | 369 | `poll_session` — exited | `None` |
| `unified_exec/tool.rs` | 380 | `poll_session` — still running | `Some(quiet_secs)` + steering line |
| `unified_exec/tool.rs` | 395 | `poll_session` — reaper race (exit -1) | `None` |

## 2. ExecOperationResult construction sites (facade)

`rust/tools/src/facade/wrapper.rs`:
- `pub struct ExecOperationResult` — line 1752 (gains `quiet_seconds: Option<u64>`,
  `skip_serializing_if Option::is_none`).
- `ExecToolFacadeWrapper::call` Ok-branch — line 1920 (copies `quiet_seconds` from
  `UnifiedExecResult` into the facade result so Codex sees it).
- Blocked-branch (1931) and Err-branch (1940) — `None`.

## 3. Output-timestamp anchor points (quiet-time source of truth)

- `spawning.rs::spawn_reader_task` (line 32): the reader task appends to
  `output_buffer` then calls `notify_ref.notify_waiters()` (line 49) — this is the
  single per-read stamp point for `last_output_micros`.
- `process_store.rs::ProcessEntry` (line 15): gains `last_output_micros: AtomicU64`
  (tokio monotonic micros). `ProcessStore` is a `Lazy` global singleton
  (`global_store()`, line ~193) reachable from codelet-sessions (codelet-tools is
  already a workspace dependency of codelet-sessions).
- `tool.rs::handle_run` sets the initial stamp at entry creation (line ~200–209).
- `output.rs::collect_output_until_deadline` (line 11): drains the buffer; when it
  drains nothing for the whole window, quiet time ≈ the yield window elapsed.

## 4. HITL mirror surfaces for P2 (verified present, to be mirrored)

- `sessions/src/background_session.rs` ~388: `hitl_request:
  RwLock<Option<HitlRequest>>` → new `exec_stdin_request:
  RwLock<Option<ExecStdinRequest>>` (no status flip, no response channel).
- `sessions/src/handle_impl.rs:968` `get_hitl_request` / `:938`
  `send_hitl_response` → new `get_exec_stdin_request` + `write_exec_stdin`.
- `rpc-types/src/lib.rs` 1202–1258: `HitlRequest` wire object family → new
  `ExecStdinRequest { exec_session_id, command, quiet_seconds, ts_ms }`.
- `tools/src/tool_progress.rs:54` `set_tool_progress_callback` registry → new
  per-agent-session exec-stdin request registry (same `SessionRegistry` pattern).
- `fspec-tui`: `store/agent_view/hitl_state.rs` slot, `views/agent/hitl_keys.rs`
  key handler, `views/agent/input_area.rs::paint_input_area` precedence chain →
  mirrored `exec_stdin_state.rs` / `exec_stdin_keys.rs` / precedence insertion.

## 5. Determinism notes (why no heuristic)

- vtcode exec core (`/tmp/vtcode/crates/codegen/vtcode-core/src/tools/registry/
  executors/exec_support.rs:244-258`): `attach_long_command_wait_steering`
  attaches `next_wait_args` + `next_action_hint` to ANY still-running command —
  no output-content inspection.
- vtcode UI (`vtcode-ui/.../session/state.rs:337-343`): "Input required" status is
  driven by HITL modal title text, never by child-process output.
- Consequence adopted: P1 exposes only the deterministic `quiet_seconds` timing
  fact + a fixed steering string; P2 trigger is time-based (quiet ≥ 3s + alive);
  nothing content-derived crosses any surface (decisions §9.1, §9.3 re-resolved).
