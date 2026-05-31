@done
@RPC-018
@rust
@tui
@app
@bootstrap
@dispatch
Feature: RPC-018 App bootstrap + dispatch wiring for AgentView chrome state
  """
  RPC-018 extends `App::bootstrap` to fetch the workspace snapshot once
  via `backend.get_workspace_info()` and dispatch
  `Action::WorkspaceInfoLoaded(info)` so the SessionFooter paints the
  cwd + branch on the very first frame.

  Per-session model info and thinking level are fetched lazily in
  `App::dispatch` on `Action::SessionCreated` (and on
  `Action::EnterWorkUnit` when `current_session` is already set). Each
  fetch is a spawned task that, on success, dispatches
  `Action::ModelInfoLoaded(session_id, info)` or
  `Action::ThinkingLevelLoaded(session_id, level)`. The matching
  dispatch arms write into AgentViewStore.

  Token state derivation lives in the existing `Action::ChunkReceived`
  arm — see rpc018-agent-chrome.feature for the chunk-driven scenarios.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want the AgentViewStore to be populated lazily on bootstrap + session creation
    So that the SessionHeader paints model badges + thinking level as soon as a session exists, and the SessionFooter paints cwd + branch from the very first frame

  Scenario: Bootstrap fetches workspace info and stores it in AgentViewStore
    Given an App constructed against a SharedFspecService bound to a temp git repo on branch "main"
    When App::bootstrap is invoked
    Then App::dispatch has matched an Action::WorkspaceInfoLoaded(info)
    And app.agent_view_store().workspace() returns Some(info) with cwd = <tmp_path> and git_branch = Some("main")

  Scenario: Bootstrap is best-effort — get_workspace_info failures do not abort
    Given an App whose backend.get_workspace_info() returns an error
    When App::bootstrap is invoked
    Then App::bootstrap returns Ok(()) (failure is non-fatal)
    And app.agent_view_store().workspace() returns None

  Scenario: Action::SessionCreated spawns get_model_info + get_thinking_level fetches
    Given an App with no current_session yet
    When App::dispatch receives Action::SessionCreated(SessionId::new("s-1"))
    Then App::dispatch sets agent_view_store.current_session = Some("s-1")
    And two new tasks are pending — one for backend.get_model_info("s-1") and one for backend.get_thinking_level("s-1")
    When both tasks complete and emit Action::ModelInfoLoaded("s-1", info) and Action::ThinkingLevelLoaded("s-1", level)
    Then app.agent_view_store().model_info_for(SessionId("s-1")) returns Some(info)
    And app.agent_view_store().thinking_level_for(SessionId("s-1")) returns Some(level)

  Scenario: Action::ChunkReceived TokenUpdate updates token_state for the current session only
    Given an App with current_session = Some(SessionId::new("s-1"))
    And token_state_by_session["s-1"] starts at TokenState::default()
    When App::dispatch receives Action::ChunkReceived("s-1", StreamChunk::TokenUpdate { tokens: TokenTracker with input_tokens = 1234 and output_tokens = 567 })
    Then agent_view_store.token_state_for(SessionId("s-1")) returns Some(TokenState with input_tokens = 1234 and output_tokens = 567)

  Scenario: Action::ChunkReceived ContextFillUpdate updates only context_fill_pct
    Given an App with current_session = Some(SessionId::new("s-1"))
    And token_state_by_session["s-1"] is TokenState { input_tokens: 100, output_tokens: 50, context_fill_pct: 0 }
    When App::dispatch receives Action::ChunkReceived("s-1", StreamChunk::ContextFillUpdate { context_fill: ContextFillInfo with fill_percentage = 45 })
    Then agent_view_store.token_state_for(SessionId("s-1")) has context_fill_pct = 45
    And input_tokens still equals 100
    And output_tokens still equals 50

  Scenario: Action::WorkspaceInfoLoaded updates the AgentViewStore.workspace slot
    Given an App with agent_view_store.workspace() returning None
    When App::dispatch receives Action::WorkspaceInfoLoaded(WorkspaceInfo { cwd: "/x", git_branch: Some("dev") })
    Then app.agent_view_store().workspace() returns Some(WorkspaceInfo { cwd: "/x", git_branch: Some("dev") })

  Scenario: Action::ModelInfoLoaded for a session NOT current_session still updates the by-session map
    Given an App with current_session = Some(SessionId::new("s-1"))
    When App::dispatch receives Action::ModelInfoLoaded(SessionId::new("s-2"), ModelInfo { display_name: "Other", supports_reasoning: false, supports_vision: false, context_window: 100000 })
    Then agent_view_store.model_info_for(SessionId("s-2")) returns Some(info)
    And agent_view_store.model_info_for(SessionId("s-1")) returns None
