# AST research: MCP-009
- `rust/tools/src/mcp/connect_tool.rs`, in the `McpAction::Connect` arm: `args.transport.unwrap_or(McpTransport::Stdio)`.
- `rust/tools/src/mcp/types.rs`, `McpConnectArgs.transport: Option<McpTransport>` (it is also `None` for disconnect and list).
- Tests that build `McpConnectArgs` are in `rust/tools/src/mcp/tests_0*.rs` and `helper_tests_0*.rs`. Tests that send ConnectMCP JSON are in `rust/tools/tests/tool025_schema_contract/mcp.rs`, `rust/sessions/tests/tool026_admission/delegation.rs`, and `rust/rpc-server/tests/mcp005*` and `mcp006*`.
