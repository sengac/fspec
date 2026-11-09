# MCP-012 root cause: tools/list after the MCP handshake is unbounded and its failure is swallowed

`establish_stdio` (`rust/tools/src/mcp/stdio_and_routing.rs`) and the shared remote serve helper (`rust/tools/src/mcp/remote_connect.rs`, used by `establish_http` and `establish_sse`) work like this:

1. Only `handler.serve(transport)` (the initialize handshake) runs under `tokio::time::timeout(MCP_HANDSHAKE_TIMEOUT_SECS)`.
2. After that, `service.peer().list_all_tools().await` runs with no bound. A server that finishes the handshake and then never answers tools/list keeps a ConnectMCP call (through `establish_and_cache`) waiting forever.
3. If `list_all_tools()` returns an Err, the code does `warn!` and uses `Vec::new()`. The connection then reports success with zero tools. This is a silent default.

MCP-010 bounds the daemon path from outside: `host_establish.rs` wraps the whole `establish` in `MCP_HANDSHAKE_TIMEOUT_SECS`. ConnectMCP does not get that bound.

Fix direction: put the handshake and tool discovery under one deadline inside the `establish_*` functions, so both front doors get it. Then MCP-010's outer wrapper can go. Treat a tools/list failure as a connect failure that names the server.
