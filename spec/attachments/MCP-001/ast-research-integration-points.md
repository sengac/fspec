# AST Research: MCP Integration Points

## Date: 2026-03-05

## 1. WatcherInput struct (session_manager.rs:333)

```rust
pub struct WatcherInput {
    pub source_session_id: String,
    pub role_name: String,
    pub authority: RoleAuthority,
    pub message: String,
    pub images: Option<Vec<BridgeImageData>>,
}
```

**Integration**: MCP server-initiated messages (notifications, sampling requests) will be injected via `watcher_input_tx` using the existing `WatcherInput` type. The `role_name` will be set to `"mcp:<server-name>"` and `authority` to `RoleAuthority::Peer`.

## 2. BackgroundSession channels (session_manager.rs:951)

```rust
watcher_input_tx: mpsc::Sender<WatcherInput>,
watcher_input_rx: Mutex<mpsc::Receiver<WatcherInput>>,
```

**Integration**: The `DynMcpHandler` (ClientHandler impl) will hold a clone of `watcher_input_tx` to inject notifications. New field `mcp_connections: HashMap<String, McpConnection>` will be added to `BackgroundSession`.

## 3. agent_loop select! (session_manager.rs:5003-5064)

```rust
let input_to_process: Option<InputWithImages> = tokio::select! {
    biased;
    result = input_rx.recv() => { ... }
    result = watcher_rx.recv() => { ... }
};
```

**Integration**: MCP messages flow through existing `watcher_input_rx` path - no changes needed to agent_loop multiplexing. For sampling/createMessage, a new `WatcherInput` variant or convention will carry the oneshot channel for round-trip response.

## 4. rig-core McpTool (patches/rig-core/src/tool/mod.rs:209)

```rust
pub struct McpTool {
    definition: rmcp::model::Tool,
    client: rmcp::service::ServerSink,
}
```

**Integration**: rig-core already has `McpTool` and `rmcp_tools()` builder methods. For Dynamic MCP, we'll use these directly but with namespaced tool names (`mcp__<server>__<tool>`). The `ServerSink` (from `RunningService::peer()`) provides `call_tool()`, `list_all_tools()` etc.

## 5. create_rig_agent patterns (claude.rs:492, openai.rs:305, codex/mod.rs:300, gemini.rs:101, zai.rs:192)

All providers follow the same pattern:
```rust
pub fn create_rig_agent(&self, session_id: uuid::Uuid, preamble: Option<&str>, thinking_config: Option<Value>) -> Agent<M> {
    self.rig_client.agent(&self.model_name)
        .tool(ReadTool::new(session_id))
        .tool(WriteTool::new(session_id))
        // ... all built-in tools ...
        .build()
}
```

**Integration**: Each provider's `create_rig_agent()` will accept an additional `mcp_tools: Option<Vec<(rmcp::model::Tool, rmcp::service::ServerSink)>>` parameter. When present, MCP tools are added via `.rmcp_tool()` builder method with namespaced names. Since `create_rig_agent()` is called fresh each turn in agent_loop, newly connected MCP tools appear immediately.

## 6. BridgeTool pattern (bridge.rs:200)

```rust
pub struct BridgeToolArgs {
    pub action: BridgeAction,  // discriminated union: connect/disconnect/list
}
```

**Integration**: ConnectMCP will follow the same action-based pattern as BridgeTool. Single tool with `action` field: `connect` (default), `disconnect`, `list`.

## 7. rmcp crate availability

- `rmcp = { version = "0.12", optional = true, features = ["client"] }` in rig-core patches
- NOT yet a workspace dependency — needs to be added to `codelet/tools/Cargo.toml`
- rmcp provides: `ServiceExt`, `ClientHandler`, `transport::TokioChildProcess`, `transport::StreamableHttpClientTransport`, `model::Tool`, `model::CallToolRequestParam`, `service::RunningService`, `service::RoleClient`

## 8. Tool registration pattern (tools/src/lib.rs)

```rust
pub mod bridge;
pub use bridge::{BridgeTool, BridgeToolArgs, ...};
```

**Integration**: New module `pub mod mcp;` will be added to `lib.rs`, exporting `McpConnectTool`, `McpConnection`, etc.

## Summary of Required Changes

1. **New file**: `codelet/tools/src/mcp.rs` — ConnectMCP tool, McpConnection, DynMcpHandler
2. **Modified**: `codelet/tools/src/lib.rs` — add `pub mod mcp;` and exports
3. **Modified**: `codelet/tools/Cargo.toml` — add `rmcp` dependency
4. **Modified**: `codelet/Cargo.toml` — add `rmcp` to workspace dependencies
5. **Modified**: `codelet/napi/src/session_manager.rs` — add `mcp_connections` to BackgroundSession
6. **Modified**: `codelet/providers/src/*.rs` — add MCP tools parameter to create_rig_agent()
