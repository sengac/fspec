# AST research: MCP-008

- `timeout_secs` appears at: `rpc-types/src/session_mcp.rs:18` (wire field) and `:73` (Debug); `tools/src/mcp/host_config.rs:20`, `:63`, `:105` (field and positive check); `tools/src/mcp/http_and_session_registry.rs:60`, `:88-89` (connect timeout); `tools/src/mcp/remote_connect.rs:5`, `:10`, `:43` (timeout message).
- The "already connected" refusal is in `tools/src/mcp/host_config.rs`, with its test in `tools/src/mcp/tests_04.rs`.
- RPC: `rpc/src/service_declaration/session_mcp.rs` and `rpc/src/service_implementation/session_mcp.rs`. MCP-003's tests are in `rpc-server/tests/mcp003*`.
- Plugin consumer: fspec-paseo `test/session-mcp-servers.test.ts`, with goldens in `test/golden/golden-vectors.txt` (FSPEC-074, df8c2ba).
