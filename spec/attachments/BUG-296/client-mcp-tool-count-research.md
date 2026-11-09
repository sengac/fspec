# Research: `client_mcp_full` hard-codes the MCP tool count

Target: `cargo test --profile ci-test -p codelet-fspec --test client_mcp_full`
Failing: `discover_all_and_continue_without_reexecution` (client_mcp_full.rs:117)
`assert_eq!(tools.len(), 125)`

| Tree | `tools.len()` |
|---|---|
| clean HEAD (81ee9f15) | 127 (already failing) |
| current tree (after BOARD-024 added `get_work_unit_details`) | 128 |

## How the list is built (`rust/fspec/src/client_mcp_full.rs:74-100`)
`3` read tools from `client_mcp_service::McpFrontend::list_tools`
(`fspec_health`, `fspec_work_units`, `fspec_work_unit_detail`)
`+ mcp_dispatch::methods().len()` typed `fspec_rpc_<name>` tools
`+ 1` `fspec_rpc_result` tool.

Every additive RPC method therefore changes the total, so the literal `125` breaks on each
new method (two methods were added before HEAD, one by BOARD-024).

## Suggested fix
Assert against the derived value (`3 + mcp_dispatch::methods().len() + 1`) and separately
assert that specific tool names exist, so the test keeps checking pagination/continuation
without encoding the RPC inventory size.
