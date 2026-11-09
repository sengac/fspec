# BUG-425 root cause

RPC create_session and create_session_checked are now identical methods

After BUG-399, FspecService::create_session (service_declaration/workspace.rs) and create_session_checked (service_declaration/supervision_credentials.rs) both return Result<SessionId, String> and both delegate to SessionManagerHandle::create_session_checked (service_implementation/interaction_sessions.rs). The TUI transports call create_session_checked, MCP group_01 calls create_session. Keep one RPC member and remove the other (wire-order change, wire_order_frozen.txt and refac019 contract snapshots move).

Found while running the BUG-399 regression targets (2026-11-03); filed to the backlog untouched per the user's instruction.
