# BUG-425 AST research

- rust/rpc/src/service_declaration/workspace.rs:22 create_session; rust/rpc/src/service_declaration/supervision_credentials.rs:260 create_session_checked.
- rust/rpc/src/service_implementation/interaction_sessions.rs:89-112 (both impls).
- Callers: rust/fspec-tui/src/transport/{embedded/sessions.rs:45,websocket/provider_sessions.rs:59} (checked); rust/rpc/src/mcp_dispatch/group_01.rs:17-27 (create_session).
- rust/rpc/tests/fixtures/wire_order_frozen.txt lines 7 and 125.
