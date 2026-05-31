@done
@RPC-018
@rust
@rpc
@cross-transport
@parity
Feature: RPC-018 cross-transport parity for get_model_info / get_thinking_level / get_workspace_info
  """
  RPC-018 adds three new RPC methods to the shared FspecService trait:

  * get_model_info(session_id) -> ModelInfo
  * get_thinking_level(session_id) -> ThinkingLevel
  * get_workspace_info() -> WorkspaceInfo

  Both FspecBackend impls (EmbeddedFspecBackend + WebSocketFspecBackend)
  delegate to the same SharedFspecService — so a single scripted scenario
  driven against both transports must return identical results.

  RPC-018 ships these methods with default `SessionManagerHandle` impls
  that return safe defaults — the concrete override in
  codelet/napi/src/session_manager.rs is deferred to RPC-022. So the
  parity scenarios assert default values; the SharedFspecService.with_cwd
  attachment plus codelet_git::status::get_current_branch wires the
  `get_workspace_info` path to live data.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want the three new RPC methods to behave identically across embedded and WebSocket transports
    So that the AgentView's SessionHeader and SessionFooter look the same regardless of which transport the Rust TUI is attached to

  Scenario: EmbeddedFspecBackend::get_workspace_info delegates through the shared service
    Given a SharedFspecService constructed via with_cwd against a temp git repo on branch "main"
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.get_workspace_info().await is invoked
    Then the awaited result is Ok(WorkspaceInfo { cwd: <tmp_path>, git_branch: Some("main") })

  Scenario: WebSocketFspecBackend::get_workspace_info crosses tarpc cleanly
    Given an rpc-server bound to the SAME shared service (cwd is a temp git repo on branch "main")
    And a WebSocketFspecBackend connected to that server
    When backend.get_workspace_info().await is invoked
    Then the awaited result is Ok(WorkspaceInfo { cwd: <tmp_path>, git_branch: Some("main") })

  Scenario: Both transports return identical WorkspaceInfo for the same SharedFspecService
    Given a SharedFspecService constructed via with_cwd against a temp git repo on branch "feature/test-branch"
    And an rpc-server bound to that shared service
    And an EmbeddedFspecBackend wrapping the same shared service
    And a WebSocketFspecBackend connected to the rpc-server
    When backend.get_workspace_info().await is invoked on BOTH backends
    Then both awaited results are equal

  Scenario: get_workspace_info returns the process cwd with no branch when no cwd was attached
    Given a SharedFspecService constructed WITHOUT with_cwd (no cwd attached)
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.get_workspace_info().await is invoked
    Then the awaited result is Ok with git_branch = None
    And the cwd field is non-empty (defaults to std::env::current_dir())

  Scenario: get_workspace_info returns git_branch = None when cwd is not a git repository
    Given a SharedFspecService constructed via with_cwd against a tempdir that is NOT a git repository
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.get_workspace_info().await is invoked
    Then the awaited result is Ok with git_branch = None
    And the cwd field equals the supplied tempdir path

  Scenario: get_model_info returns safe defaults when no session manager is attached
    Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.get_model_info(SessionId::new("anything")).await is invoked
    Then the awaited result is Ok(ModelInfo::default()) with display_name = "" and supports_reasoning = false and supports_vision = false and context_window = 0

  Scenario: get_thinking_level returns ThinkingLevel::Off when no session manager is attached
    Given a SharedFspecService constructed via SharedFspecService::new (no session manager attached)
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.get_thinking_level(SessionId::new("anything")).await is invoked
    Then the awaited result is Ok(ThinkingLevel::Off)

  Scenario: get_model_info / get_thinking_level cross tarpc cleanly with safe defaults
    Given an rpc-server bound to a SharedFspecService with NO session manager attached
    And a WebSocketFspecBackend connected to that server
    When backend.get_model_info(SessionId::new("anything")).await is invoked
    Then the awaited result is Ok(ModelInfo::default())
    When backend.get_thinking_level(SessionId::new("anything")).await is invoked
    Then the awaited result is Ok(ThinkingLevel::Off)

  Scenario: StubSessionManagerHandle inherits the default SessionManagerHandle implementations
    Given a SharedFspecService constructed via with_session_manager(stub_handle, watcher)
    And an EmbeddedFspecBackend wrapping that shared service
    When backend.get_model_info(SessionId::new("stub-1")).await is invoked
    Then the awaited result is Ok(ModelInfo::default())
    When backend.get_thinking_level(SessionId::new("stub-1")).await is invoked
    Then the awaited result is Ok(ThinkingLevel::Off)
