# AST research: MCP-011
- `rust/tools/src/mcp/types.rs`: `McpConnectArgs.action` has `#[serde(default)]`, and `McpAction` derives `Default` with `Connect`.
- `rust/tools/src/mcp/connect_tool.rs`: `definition()` has the schema `unwrap_or_else` fallback, and `call()` has the `to_value(&args).unwrap_or_default()` hook payload.
- The tests that send ConnectMCP JSON without `action` need it added: `rust/tools/src/mcp/tests_03.rs` (the timeout test) and the rpc-server `mcp005`/`mcp006` tests (check each).
