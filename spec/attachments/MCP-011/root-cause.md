# MCP-011 root cause: ConnectMCP still has three silent defaults

Found while fixing MCP-009 (the missing-transport default).

1. `McpConnectArgs.action` (`rust/tools/src/mcp/types.rs`) is `#[serde(default)]`, and `McpAction::default()` is `Connect`. A call that leaves out `action` is quietly treated as a connect. The tool description also says "action 'connect' (default)".
2. `ConnectMcpTool::call` (`rust/tools/src/mcp/connect_tool.rs`) builds the pre_tool_use hook payload with `serde_json::to_value(&args).unwrap_or_default()`. If serialization fails, the hook gets `Null` and the error disappears.
3. `ConnectMcpTool::definition` uses `serde_json::to_value(schemars::schema_for!(McpConnectArgs)).unwrap_or_else(|_| json!({"type": "object"}))`. If the schema fails to serialize, the model gets an empty, permissive schema.

Fix direction: make `action` required (drop the serde default and "(default)" from the description), and turn the two serialization failures into errors. A `ToolError` in `call` is fine. `definition` can't return an error, so find out how the other tools handle that and do the same.
