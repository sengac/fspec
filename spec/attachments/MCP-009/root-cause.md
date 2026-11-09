# MCP-009 root cause: ConnectMCP silently defaults a missing transport to stdio

`ConnectMcpTool::call` (in `rust/tools/src/mcp/connect_tool.rs`, inside the `McpAction::Connect` arm) does `let transport = args.transport.unwrap_or(McpTransport::Stdio);`.
- `McpConnectArgs.transport` is `Option<McpTransport>`, so a connect call that omits `transport` is treated as stdio without saying so.
- A model that meant http and forgot the field gets a stdio validation error ("command required"), not "transport is required".
- This breaks the no-silent-defaults rule (found during MCP-008).

Fix direction: make `transport` required for `connect`. Return `ToolError::Validation` "transport is required for connect action", the same way `name` is handled. Update the ConnectMCP schema and the tests that omit it.
