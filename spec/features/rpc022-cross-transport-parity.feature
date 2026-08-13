@done
@RPC-022
@rust
@rpc
@cross-transport
@parity
@model-selection
Feature: RPC-022 cross-transport parity for list_providers, set_session_model, set_thinking_level, get/set_session_role
  """
  RPC-022 adds five new RPC methods to the shared FspecService trait:

  * list_providers() -> Vec<ProviderInfo>
  * set_session_model(SessionId, String, String) -> Result<(), String>
  * set_thinking_level(SessionId, ThinkingLevel) -> Result<(), String>
  * get_session_role(SessionId) -> Option<String>
  * set_session_role(SessionId, Option<String>) -> Result<(), String>

  Both FspecBackend impls (EmbeddedFspecBackend + WebSocketFspecBackend)
  delegate to the same SharedFspecService — so a single scripted scenario
  driven against both transports must return identical results.

  Like RPC-018, RPC-022 ships these methods with default
  `SessionManagerHandle` impls returning safe defaults:
  - list_providers (free-standing on SharedFspecService): Vec::new() when no SessionManagerHandle is attached
  - set_session_model / set_thinking_level / set_session_role: Ok(()) — silent no-ops, idempotent
  - get_session_role: None

  The concrete rust/napi SessionManager override is in scope of this
  card (additive NAPI exports) but the parity scenarios pin the default
  contract.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want the five new RPC methods to behave identically across embedded and WebSocket transports
    So that the AgentView's ModelSelectorDialog / ThinkingLevelDialog / RoleBanner look the same regardless of transport

  @list-providers
  @embedded
  Scenario: list_providers returns empty Vec when no session manager is attached (embedded)
    Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.list_providers().await is invoked
    Then the awaited result is Ok(vec![])

  @list-providers
  @websocket
  Scenario: list_providers crosses tarpc cleanly when no session manager is attached
    Given an rpc-server bound to a SharedFspecService with NO session manager attached
    And a WebSocketFspecBackend connected to that server
    When backend.list_providers().await is invoked
    Then the awaited result is Ok(vec![])

  @list-providers
  @parity
  Scenario: Both transports return identical providers for the same SharedFspecService
    Given a SharedFspecService with a session manager that returns [ProviderInfo{ key: "openai", display_name: "OpenAI", models: vec![ModelEntry{ id: "gpt-5.1-codex", display_name: "gpt-5.1-codex", context_window: 200_000, supports_reasoning: true, supports_vision: false, is_custom: false }]}]
    And an rpc-server bound to that shared service
    And an EmbeddedFspecBackend wrapping the same shared service
    And a WebSocketFspecBackend connected to the rpc-server
    When backend.list_providers().await is invoked on BOTH backends
    Then both awaited results are equal

  @set-session-model
  @embedded
  Scenario: set_session_model returns Ok when no session manager is attached (embedded)
    Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.set_session_model(SessionId::new("anything"), "openai".to_string(), "gpt-5.1-codex".to_string()).await is invoked
    Then the awaited result is Ok(())

  @set-session-model
  @websocket
  Scenario: set_session_model crosses tarpc cleanly when no session manager is attached
    Given an rpc-server bound to a SharedFspecService with NO session manager attached
    And a WebSocketFspecBackend connected to that server
    When backend.set_session_model(SessionId::new("anything"), "openai".to_string(), "gpt-5.1-codex".to_string()).await is invoked
    Then the awaited result is Ok(())

  @set-thinking-level
  @embedded
  Scenario: set_thinking_level returns Ok when no session manager is attached (embedded)
    Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.set_thinking_level(SessionId::new("anything"), ThinkingLevel::High).await is invoked
    Then the awaited result is Ok(())

  @set-thinking-level
  @websocket
  Scenario: set_thinking_level crosses tarpc cleanly with safe default
    Given an rpc-server bound to a SharedFspecService with NO session manager attached
    And a WebSocketFspecBackend connected to that server
    When backend.set_thinking_level(SessionId::new("anything"), ThinkingLevel::Medium).await is invoked
    Then the awaited result is Ok(())

  @get-session-role
  @embedded
  Scenario: get_session_role returns None when no session manager is attached (embedded)
    Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.get_session_role(SessionId::new("anything")).await is invoked
    Then the awaited result is Ok(None)

  @get-session-role
  @websocket
  Scenario: get_session_role crosses tarpc cleanly with safe default
    Given an rpc-server bound to a SharedFspecService with NO session manager attached
    And a WebSocketFspecBackend connected to that server
    When backend.get_session_role(SessionId::new("anything")).await is invoked
    Then the awaited result is Ok(None)

  @set-session-role
  @embedded
  Scenario: set_session_role returns Ok when no session manager is attached (embedded)
    Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.set_session_role(SessionId::new("anything"), Some("Reviewer A".to_string())).await is invoked
    Then the awaited result is Ok(())
    When backend.set_session_role(SessionId::new("anything"), None).await is invoked
    Then the awaited result is Ok(())

  @set-session-role
  @websocket
  Scenario: set_session_role crosses tarpc cleanly with safe default
    Given an rpc-server bound to a SharedFspecService with NO session manager attached
    And a WebSocketFspecBackend connected to that server
    When backend.set_session_role(SessionId::new("anything"), Some("Reviewer A".to_string())).await is invoked
    Then the awaited result is Ok(())

  @stub
  @session-manager
  Scenario: StubSessionManagerHandle inherits the default SessionManagerHandle implementations for the five new RPC methods
    Given a SharedFspecService constructed via with_session_manager(stub_handle, watcher)
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.list_providers().await is invoked
    Then the awaited result is Ok(vec![])
    When backend.get_session_role(SessionId::new("stub-1")).await is invoked
    Then the awaited result is Ok(None)
    When backend.set_session_model(SessionId::new("stub-1"), "openai".to_string(), "gpt-5.1-codex".to_string()).await is invoked
    Then the awaited result is Ok(())
    When backend.set_thinking_level(SessionId::new("stub-1"), ThinkingLevel::Off).await is invoked
    Then the awaited result is Ok(())
    When backend.set_session_role(SessionId::new("stub-1"), None).await is invoked
    Then the awaited result is Ok(())
