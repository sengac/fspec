# MCP-008 design: declarative session MCP sets with an fspec-owned connect timeout

Both decisions were made by the user on 2026-11-02, following findings from fspec-paseo FSPEC-074 and FSPEC-075.

## Decision 1: fspec owns the MCP connect timeout
- Remove `timeout_secs` from `SessionMcpServer` (`rust/rpc-types/src/session_mcp.rs`). The wire becomes `SessionMcpServer { name, transport }`. No backward compatibility is needed.
- fspec has exactly ONE MCP handshake timeout, a named constant defined in one place. ConnectMCP and the daemon path (`host_config.rs`, `http_and_session_registry.rs`, `remote_connect.rs`) both use it.
- If ConnectMCP's tool schema also takes a timeout from the model, decide during specifying whether that stays. The default is to remove it, so there is one policy.
- Remove the "needs a positive timeout_secs" validation (`host_config.rs:63/105`), since the field no longer exists.
- The timeout error message names the constant's value.
- No fallbacks: this is a fixed policy, not a default for a missing value.

## Decision 2: configure_session_mcp_servers replaces the set
- `configure_session_mcp_servers(session_id, servers)` becomes declarative: the given list becomes the session's whole MCP server set.
  - A server in the current set but not in the list is disconnected, and its tools are removed from the session.
  - A server whose name and transport are identical to the current one is kept connected and is not reconnected.
  - A server with the same name and a different transport is disconnected and reconnected with the new transport.
  - A new name is connected.
- **Atomicity:** if any connect fails, the call returns `Err` naming the server. Define during specifying whether the session keeps its previous set (preferred: connect the new and changed ones first, then commit the swap) or is left without the failed one. Never leave a partial set without reporting it.
- An empty list is now valid: it means "no MCP servers", and disconnects all of them. The rejection of an empty list goes away.
- The "already connected" refusal (`host_config.rs`, `mcp/tests_04.rs`) no longer applies to this RPC.
- The returned `Vec<SessionMcpServerTools>` describes the resulting set.
- The wire order is unchanged: same method, same index (the last line of `wire_order_frozen.txt`). The argument shape changes with Decision 1.

## Follow-up
fspec-paseo:
- Drop the hard-coded 10 s timeout.
- Always call configure on open and resume, including with an empty list.
- Close FSPEC-075.
- Regenerate the golden vectors.

## Resolved during specifying and implementing
- ConnectMCP's model-supplied `timeout` is removed (one policy). `MCP_HANDSHAKE_TIMEOUT_SECS = 10` lives in `rust/tools/src/mcp/types.rs`.
- `SessionMcpServer` also gets `#[serde(deny_unknown_fields)]`, so a JSON client that still sends `timeout_secs` gets an explicit error instead of having it ignored.
- Atomicity: connecting is split into `establish_*` (handshake and discovery, nothing cached) and `cache_connection` (`mcp/establish.rs`). `host_config` establishes every new and changed server first. Only when all of them succeed does it commit, under one write lock: unlisted servers are removed, changed ones replaced, identical ones kept. On a failure the established connections are cancelled and the previous set is untouched.
- "Identical" means `McpConnection.configured_transport == Some(requested transport)`. A connection made by ConnectMCP has `None`, so it is always reconnected when it is listed.
- If a kept server changes between the read snapshot and the commit, the call fails with `ChangedDuringConfiguration(name)` instead of committing a set it did not describe.
- The `NoServers`, `AlreadyConnected` and `ZeroTimeout` errors are removed.
- Client deadline: tarpc's default deadline is 10 s, the same as the handshake timeout. A client that configures a server which might time out needs a longer RPC deadline. See the follow-up card for configuring servers concurrently.
