# MCP-010 design: establish session MCP servers concurrently

Found during MCP-008. `configure_host_mcp_servers` (`rust/tools/src/mcp/host_config.rs`) establishes each new or changed server in turn, and each one may take up to `MCP_HANDSHAKE_TIMEOUT_SECS` (10 s).
- tarpc's default client deadline (`context::current()`) is also 10 s. A single server that times out already reaches that deadline, and N slow servers take N×10 s.
- The MCP-008 test therefore sets a 30 s deadline.

Options to decide in specifying:
1. Establish all new and changed servers concurrently (`futures::future::join_all`). The call then takes at most about one handshake timeout, and the atomic commit is unchanged.
2. Document the deadline that daemon clients need (fspec-paseo).

Either way, a failure still names the server, and the previous set stays intact.
