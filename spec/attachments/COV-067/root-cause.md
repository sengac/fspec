# Root cause: dynamic-mcp-connect coverage is stale

Found while working on MCP-007.

## What is wrong
`spec/features/dynamic-mcp-connect.feature.coverage` maps:
- "Route tool call through cached MCP connection", "Connect to MCP server via HTTP transport", "Multi-server workflow with independent connections" and "Receive server tool list changed notification" to line ranges in `rust/tools/src/mcp/connect_tool.rs`, which is production code, not a test.
- "Handle server sampling/createMessage request", "List active MCP connections", "Disconnect an active MCP connection", "Handle HTTP authentication failure" and "Session cleanup kills all MCP connections" to ranges in `rust/tools/src/mcp/tests_01.rs` that hold other scenarios' tests (for example, 1-62 is the file header and helpers).
- The `tests_02.rs` ranges for the last three scenarios have not been checked.

The mappings appear to predate the split of the MCP module into `tests_0N.rs` and `helper_tests_0N.rs` (REFAC-032) and were never relinked afterwards.

## Already fixed by MCP-007
"Connect to MCP server via stdio transport" (`tests_01.rs:92-153`), "Handle spawn failure with structured error" (`tests_01.rs:199-236`) and "Handle connection timeout during initialization" (`tests_03.rs:4-42`).

## Fix
For every other scenario, find its test function (search for the `// Scenario:` banner in `rust/tools/src/mcp/tests_0*.rs`), run `fspec unlink-coverage --all`, then relink the test and implementation ranges with `fspec link-coverage`.
