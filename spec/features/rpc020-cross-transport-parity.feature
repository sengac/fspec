@done
@agent-view
@RPC-020
@rust
@rpc
@cross-transport
@parity
Feature: RPC-020 cross-transport parity for search_files

  """
  RPC-020 adds a new `search_files(prefix, limit)` RPC method to the
  shared FspecService trait. Both FspecBackend impls
  (EmbeddedFspecBackend + WebSocketFspecBackend) delegate to the same
  SharedFspecService — so a scripted scenario driven against both
  transports must return identical results.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want search_files to behave identically across embedded and WebSocket transports
    So that the AgentView's @file popup looks the same regardless of which transport the Rust TUI is attached to

  Scenario: EmbeddedFspecBackend::search_files delegates through the shared service
    Given a SharedFspecService constructed via with_cwd against a tempdir containing files ["README.md", "src/main.rs"]
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.search_files("README".to_string(), 10).await is invoked
    Then the awaited result is Ok with at least one entry containing "README.md"

  Scenario: WebSocketFspecBackend::search_files crosses tarpc cleanly
    Given an rpc-server bound to a SharedFspecService whose cwd contains files ["README.md", "src/main.rs"]
    And a WebSocketFspecBackend connected to that server
    When backend.search_files("README".to_string(), 10).await is invoked
    Then the awaited result is Ok with at least one entry containing "README.md"

  Scenario: Both transports return identical Vec<String> for the same SharedFspecService
    Given a SharedFspecService constructed via with_cwd against a tempdir containing 5 files matching the prefix "src"
    And an rpc-server bound to that shared service
    And an EmbeddedFspecBackend wrapping the same shared service
    And a WebSocketFspecBackend connected to the rpc-server
    When backend.search_files("src".to_string(), 10).await is invoked on BOTH backends
    Then both awaited results are equal

  Scenario: search_files returns an empty Vec when no cwd is attached
    Given a SharedFspecService constructed via SharedFspecService::new (no cwd attached)
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.search_files("anything".to_string(), 10).await is invoked
    Then the awaited result is Ok with an empty Vec

  Scenario: search_files honours the limit argument across transports
    Given a SharedFspecService constructed via with_cwd against a tempdir containing 25 files matching the prefix "doc"
    And an rpc-server bound to that shared service
    And a WebSocketFspecBackend connected to that server
    When backend.search_files("doc".to_string(), 5).await is invoked
    Then the awaited result is Ok with exactly 5 entries
