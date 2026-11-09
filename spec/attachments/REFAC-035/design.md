# REFAC-035 design: split host_config.rs before MCP-010

`rust/tools/src/mcp/host_config.rs` is 294 lines. MCP-010 replaces its sequential establish loop with concurrent establishment and an error that names every failing server, which would take the file over 300 lines.

- Move `establish()` (the per-server handshake under `MCP_HANDSHAKE_TIMEOUT_SECS`) and `redact()` (removing secrets from connect errors) verbatim into `rust/tools/src/mcp/host_establish.rs`.
- `host_config.rs` pulls that file in with `include!("host_establish.rs")` in the same place, so both functions stay private to `codelet_tools::mcp::host_config` and nothing else changes. This is the include! pattern from REFAC-032.
- Acceptance: a new scenario in `spec/features/mcp-module-split.feature`, tested in REFAC-032's `rust/tools/tests/refac032_mcp_module_split.rs` (that card owns the feature's test file). The existing scenario that every MCP file stays under 300 lines keeps checking the result, and the rpc-server mcp003/005/006/008 suites check behaviour.
- Coverage mappings that point into lines 225-294 of `host_config.rs` move to the matching lines of `host_establish.rs`.
