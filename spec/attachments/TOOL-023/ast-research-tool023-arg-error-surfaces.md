# AST research — TOOL-023 (tool-call arg error recovery surfaces)

## Findings (AstGrep + Grep, 2026-09-16)

### map_params call sites (codelet-tools/src/facade/wrapper.rs)
All 4 Fspec provider facades funnel through `self.facade.map_params(args.0)`:
- `wrapper.rs:1065` — `FspecToolFacadeWrapper::call` (the LLM-facing entry point)
The other 8 hits (lines 88, 231, 409, 1181, 1311, 1519, 1708, 1917) are the
web-search/file/ls/exec/bridge/HITL facades — out of scope for TOOL-023.

### UnknownCommand construction sites (codelet-fspec-core)
- `src/dispatch.rs:159,174` — unknown name in `dispatch_command` (main path)
- `src/help_dispatch.rs:75` — unknown name via `<name> --help` / `help {command: name}`
- `src/dispatch.rs:891` — unreachable stub arm
All three now route through `unknown_command_with_recovery` (or carry an empty
recovery payload for the unreachable arm), so every UnknownCommand surface gets
the did-you-mean + help guidance.

### InvalidArgs construction sites
156 command modules build `FspecCoreError::InvalidArgs { command, reason }`
from `serde_json::from_str(args_json).map_err(...)`. Instead of touching all
156 sites, `DispatchResult::from_error` (dispatch.rs) appends the usage-hint
line to any `InvalidArgs` display — single choke point, zero command edits.

### dispatch_command callers
- `agent-loop/src/agent_loop.rs:623` — standalone-binary fallback (no chunk
  callback registered) → result.error flows to the LLM tool result verbatim
- `fspec/src/common.rs` CLI bridge — same `err.to_string()` contract
- NAPI/TS delegation path (chunk callback registered) bypasses the Rust
  dispatcher entirely — the TOOL-023 errors are therefore specific to the
  standalone Rust binary + Rust-dispatched sessions, which matches scope.
