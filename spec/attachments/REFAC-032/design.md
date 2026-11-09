# REFAC-032 design: split tools/src/mcp.rs under 300 lines

## Problem

`rust/tools/src/mcp.rs` is 3375 lines: ~1440 lines of production code (connection types, the rmcp
client handler, stdio/HTTP connect, routing, the per-session `MCP_SESSIONS` registry, tool wrappers
and the ConnectMCP tool) and a ~1940-line `#[cfg(test)] mod tests` with a nested `helper_tests`
module. MCP-003 (daemon-configured session MCP servers), MCP-004 (HTTP headers), MCP-005
(structured stdio arguments) and MCP-006 (legacy SSE transport) all change it; AGENTS.md requires
touched files to be under 300 lines.

## Chosen split

`mcp.rs` keeps its module docs and imports and `include!`s, in the original order, six production
fragments under `rust/tools/src/mcp/`: `types.rs`, `client_handler.rs`, `stdio_and_routing.rs`,
`http_and_session_registry.rs`, `tool_wrappers.rs`, `connect_tool.rs`. The `tests` module (same
attributes) `include!`s `tests_01..04.rs`, and its nested `helper_tests` module (same `use` lines)
`include!`s `helper_tests_01..05.rs`. Fragments are cut by a script at blank lines between
top-level items (no item is split) and test fragments are dedented by their module depth. A
script checked that the non-blank lines of the reassembled module equal the original exactly.

Because `include!` pastes items into the same module, every item keeps its
`codelet_tools::mcp` path and its privacy (the private helpers stay visible to their callers) —
the same technique REFAC-028 used for ProviderManager.

## Alternatives rejected

- Real submodules (`mcp/stdio.rs` as `mod stdio;`): would need `pub(super)`/re-exports for the
  private helpers and statics, changing visibility and touching every item — a larger, riskier
  diff for a refactor that must not change behaviour. MCP-003..006 can still promote a fragment
  to a submodule when they change it.

## Guard

`spec/features/mcp-module-split.feature` and `rust/tools/tests/refac032_mcp_module_split.rs`:
every file of the module is under 300 lines and the session-registry/tool-name surface works
through `codelet_tools::mcp`.

## Coverage

Mappings into mcp.rs are relocated to the fragment and line where each moved line now lives.

## Wire / API changes

None.
