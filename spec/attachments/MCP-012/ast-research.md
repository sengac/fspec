# AST research: MCP-012
- `rust/tools/src/mcp/stdio_and_routing.rs`, `establish_stdio`: `tokio::time::timeout(..., handler.serve(transport))`, then `list_all_tools()` with `Err(e) => { warn!(...); Vec::new() }`.
- `rust/tools/src/mcp/remote_connect.rs`: the same pattern with a `remaining` budget for the handshake only.
- `rust/tools/src/mcp/sse_connect.rs`: computes `timeout.saturating_sub(started.elapsed())` for the remote helper.
- `rust/tools/src/mcp/host_establish.rs` (MCP-010): `establish_bounded`, the outer timeout to remove once MCP-012 lands.
- Test fixture: `rust/rpc-server/tests/support/mcp_fixture_server.rs` `--no-tools` (answers initialize, never tools/list).
