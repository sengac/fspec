@done
@RPC-062
@source-shape
@session-management
@mcp
@rpc
@rust
@rpc-062
Feature: RPC-062 MCP Injection Source Shape
  """
  Source-shape regression test for RPC-062. Pins the four MCP wiring touchpoints inside rust/sessions/src/session_manager.rs (McpInjection import, two init_mcp_session call sites, one cleanup_mcp_session call site, spawn_agent_loop trait signature) and asserts the NAPI-side consumer in rust/napi/src/agent_loop.rs still consumes the receiver. Also asserts the negative invariant that no MCP method has leaked into rust/core/src/session_manager_handle.rs, rust/rpc/src/lib.rs, or rust/fspec-tui/src/transport/mod.rs.

  Companion feature: spec/features/rpc-062-mcp-injection-lifecycle.feature
  """

  Background: User Story
    As a fspec TUI maintainer extending the supervisor surface to the Rust frontend
    I want every cross-crate MCP touchpoint pinned by a source-shape test
    So that the codelet-sessions wiring keeps init_mcp_session and cleanup_mcp_session reachable and no MCP method leaks into the RPC surface

  Scenario: codelet-sessions imports the NAPI-free McpInjection type from codelet-tools
    Given the file rust/sessions/src/session_manager.rs is compiled
    When I scan its source bytes after stripping Rust comments
    Then it contains exactly one occurrence of the substring "use codelet_tools::McpInjection;"
    And it contains zero local definitions of "enum McpInjection" or "struct McpInjection"

  Scenario: session_manager.rs calls init_mcp_session in both create paths
    Given the file rust/sessions/src/session_manager.rs is compiled
    When I scan its source bytes after stripping Rust comments
    Then it contains exactly two occurrences of the substring "codelet_tools::init_mcp_session(uuid)"
    And one occurrence sits inside the body of "pub async fn create_session_with_id"
    And the other occurrence sits inside the body of "pub async fn create_isolated_session_with_id"
    And each occurrence is followed by an invocation of "spawn_agent_loop(session.clone(), input_rx, mcp_injection_rx)"

  Scenario: session_manager.rs calls cleanup_mcp_session in destroy_session
    Given the file rust/sessions/src/session_manager.rs is compiled
    When I scan its source bytes after stripping Rust comments
    Then it contains exactly one occurrence of the substring "codelet_tools::cleanup_mcp_session(uuid)"
    And the occurrence sits inside the body of "pub fn destroy_session"

  Scenario: SessionManagerHooks trait declares spawn_agent_loop with the mcp_injection_rx parameter
    Given the file rust/sessions/src/session_manager.rs is compiled
    When I scan its source bytes after stripping Rust comments
    Then the SessionManagerHooks trait declares a method named spawn_agent_loop
    And the spawn_agent_loop signature contains the parameter "mcp_injection_rx: mpsc::Receiver<McpInjection>"

  Scenario: codelet-napi agent_loop consumes mcp_injection_rx inside its select! loop
    Given the file rust/napi/src/agent_loop.rs is compiled
    When I scan its source bytes after stripping Rust comments
    Then the file declares a function whose signature contains "mut mcp_injection_rx: mpsc::Receiver<McpInjection>"
    And the file contains at least one occurrence of "mcp_injection_rx.recv()" inside the agent loop body

  Scenario: No MCP injection methods leak into the RPC surface across handle, service, and backend traits
    Given the files rust/core/src/session_manager_handle.rs, rust/rpc/src/lib.rs, and rust/fspec-tui/src/transport/mod.rs are compiled
    When I scan their source bytes after stripping Rust comments
    Then no file contains the substring "init_mcp"
    And no file contains the substring "cleanup_mcp"
    And no file contains the substring "mcp_session"
    And no file contains the substring "mcp_injection"

  Scenario: codelet-sessions has no transitive napi dependency after the RPC-062 audit
    Given the existing test rust/sessions/tests/no_napi_dependency.rs is in the codelet-sessions test suite
    When I run cargo test -p codelet-sessions --test no_napi_dependency
    Then both scenarios in that test file pass without modification
    And the codelet-sessions transitive dependency graph contains zero occurrences of the codelet-napi package
