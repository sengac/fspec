# AST research: MCP-010
- `rust/tools/src/mcp/host_config.rs`: `configure_host_mcp_servers` runs a sequential `for` loop over `establish(server, ...)`, then calls `commit`.
- `rust/tools/src/mcp/establish.rs`: `establish_stdio`, `establish_http` and `establish_sse` return uncached connections, so they are safe to run concurrently.
- `rust/rpc-server/tests/mcp008_declarative_session_mcp_set.rs`: the timeout scenario sets `ctx.deadline` to 3× the handshake timeout.
