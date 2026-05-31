@done
@RPC-062
@agent-loop
@session-management
@mcp
@rpc
@rust
@rpc-062
Feature: RPC-062 MCP Injection Lifecycle
  """
  Lifecycle regression test for RPC-062. Drives codelet_tools::init_mcp_session, get_mcp_connections, and cleanup_mcp_session directly against the process-global MCP_SESSIONS registry that codelet-sessions::SessionManager depends on. Confirms init→Some, cleanup→None, idempotent re-init, and unknown-uuid cleanup is a silent no-op.

  Companion feature: spec/features/rpc-062-mcp-injection-source-shape.feature
  """

  Background: User Story
    As a fspec TUI maintainer extending the supervisor surface to the Rust frontend
    I want every MCP per-session lifecycle helper covered by runtime tests against the global MCP_SESSIONS registry
    So that init_mcp_session and cleanup_mcp_session remain wired through codelet-sessions::SessionManager after the NAPI extraction

  Scenario: Idempotent re-init replaces the entry without leaking the previous receiver
    Given I have called codelet_tools::init_mcp_session(uuid) once and held onto its receiver
    When I call codelet_tools::init_mcp_session(uuid) a second time for the same uuid
    Then the second call returns a fresh mpsc::Receiver<McpInjection> distinct from the first
    And codelet_tools::get_mcp_connections(uuid) still returns Some after both calls
    And calling codelet_tools::cleanup_mcp_session(uuid) afterwards drops the entry and returns None

  Scenario: cleanup_mcp_session on an unknown uuid is a silent no-op
    Given I generate a fresh uuid that has never been registered via init_mcp_session
    When I call codelet_tools::cleanup_mcp_session(uuid) for the unknown uuid
    Then the call returns without panicking
    And codelet_tools::get_mcp_connections(uuid) returns None

  Scenario: init_mcp_session registers per-session state and get_mcp_connections returns Some
    Given the codelet-tools crate is compiled and the process-global MCP_SESSIONS registry is in its initial state
    And I generate a fresh uuid via uuid::Uuid::new_v4()
    When I call codelet_tools::init_mcp_session(uuid)
    Then the returned tuple yields an mpsc::Receiver<McpInjection> and an McpConnectionMap
    And codelet_tools::get_mcp_connections(uuid) returns Some(map)

  Scenario: cleanup_mcp_session removes per-session state and get_mcp_connections returns None
    Given I have called codelet_tools::init_mcp_session(uuid) for a fresh uuid
    And codelet_tools::get_mcp_connections(uuid) currently returns Some
    When I call codelet_tools::cleanup_mcp_session(uuid)
    Then the cleanup call does not panic
    And codelet_tools::get_mcp_connections(uuid) returns None

  Scenario: MCP_SESSIONS registry isolates entries per session uuid
    Given I have called codelet_tools::init_mcp_session(uuid_a) for a fresh uuid_a
    And I have called codelet_tools::init_mcp_session(uuid_b) for a separate fresh uuid_b
    When I call codelet_tools::cleanup_mcp_session(uuid_a)
    Then codelet_tools::get_mcp_connections(uuid_a) returns None
    And codelet_tools::get_mcp_connections(uuid_b) still returns Some
