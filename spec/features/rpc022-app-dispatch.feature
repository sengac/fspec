@done
@RPC-022
@rust
@tui
@dispatch
@agent-view
@slash-command
Feature: App::dispatch wiring for ModelSelected / ThinkingLevelSelected / SetSessionRole
  """
  App::dispatch in src/app/dispatch.rs gains five new match arms wired
  through helpers in src/app/dispatch_rpc022.rs (mirroring the
  dispatch_rpc020.rs / dispatch_rpc024.rs / dispatch_rpc026.rs split
  pattern from earlier RPC cards):

  - Action::ModelSelected(sid, provider, model)        → handle_model_selected
  - Action::ThinkingLevelSelected(sid, level)          → handle_thinking_level_selected
  - Action::SetSessionRole(sid, Option<String>)        → handle_set_session_role
  - Action::SessionRoleLoaded(sid, Option<String>)     → handle_session_role_loaded
  - Action::ListProvidersLoaded(Vec<ProviderInfo>)     → handle_list_providers_loaded

  The existing RPC-018 Action::ModelInfoLoaded /
  Action::ThinkingLevelLoaded arms (unchanged) are re-used after
  set_session_model / set_thinking_level resolve so the SessionHeader
  badges repaint automatically.

  In addition, App::dispatch::handle_slash_command in dispatch_rpc020.rs
  is amended so SlashCommandAction::Model dispatches `Action::OpenModelDialog`
  (which spawns backend.list_providers() → Action::ListProvidersLoaded
  AND pushes a fresh ModelSelectorDialog onto the Compositor) and
  SlashCommandAction::Thinking pushes a fresh ThinkingLevelDialog
  directly (no backend call needed at open time — initial selection
  comes from AgentViewStore.thinking_level_for(sid)).

  SlashCommandAction::Role STILL routes through the [notice] fallback
  in the popup path because the popup auto-closes once the user types
  a space (popups.rs `classify_buffer` → Close), so the actual /role
  text is parsed by `handle_input_submitted` via parse_slash_command,
  not by the popup-selection arm.
  """

  Background: User Story
    As a developer maintaining the Rust ratatui TUI
    I want the App::dispatch surface to host all the action-routing logic for the new modal dialogs
    So that every store mutation happens on the App task per the RPC-009 single-task invariant

  @model-selection
  @dispatch
  Scenario: Action::ModelSelected spawns set_session_model and refreshes SessionHeader chrome
    Given an App attached to an EmbeddedFspecBackend wrapping a SharedFspecService with a session manager attached
    And an open session SessionId("s-1") with current_session_index = 0
    When the App dispatches Action::ModelSelected(SessionId("s-1"), "openai", "gpt-5.1-codex")
    Then a tokio task is spawned that calls backend.set_session_model(SessionId("s-1"), "openai", "gpt-5.1-codex")
    When the spawned task completes
    Then a follow-up tokio task is spawned that calls backend.get_model_info(SessionId("s-1"))
    And Action::ModelInfoLoaded(SessionId("s-1"), <fresh ModelInfo>) is dispatched

  @thinking-level
  @dispatch
  Scenario: Action::ThinkingLevelSelected spawns set_thinking_level and refreshes the [T:] badge
    Given an App attached to an EmbeddedFspecBackend with a session manager attached
    And an open session SessionId("s-1")
    When the App dispatches Action::ThinkingLevelSelected(SessionId("s-1"), ThinkingLevel::High)
    Then a tokio task is spawned that calls backend.set_thinking_level(SessionId("s-1"), ThinkingLevel::High)
    When the spawned task completes
    Then a follow-up tokio task is spawned that calls backend.get_thinking_level(SessionId("s-1"))
    And Action::ThinkingLevelLoaded(SessionId("s-1"), ThinkingLevel::High) is dispatched

  @set-role
  @dispatch
  Scenario: Action::SetSessionRole(Some) spawns set_session_role and updates AgentViewStore.role_by_session
    Given an App attached to an EmbeddedFspecBackend with a session manager attached
    And an open session SessionId("s-1") whose role_for is None
    When the App dispatches Action::SetSessionRole(SessionId("s-1"), Some("You are a security reviewer".to_string()))
    Then AgentViewStore.role_for(&SessionId("s-1")) equals Some("You are a security reviewer")
    And a tokio task is spawned that calls backend.set_session_role(SessionId("s-1"), Some("You are a security reviewer".to_string()))

  @set-role
  @dispatch
  Scenario: Action::SetSessionRole(None) clears the role and persists via backend
    Given an App attached to an EmbeddedFspecBackend with a session manager attached
    And an open session SessionId("s-1") whose role_for is Some("Reviewer A")
    When the App dispatches Action::SetSessionRole(SessionId("s-1"), None)
    Then AgentViewStore.role_for(&SessionId("s-1")) equals None
    And a tokio task is spawned that calls backend.set_session_role(SessionId("s-1"), None)

  @session-role-loaded
  @dispatch
  Scenario: Action::SessionRoleLoaded folds a backend-fetched role into AgentViewStore
    Given an App attached to an EmbeddedFspecBackend with a session manager attached
    And an open session SessionId("s-1") whose role_for is None
    When the App dispatches Action::SessionRoleLoaded(SessionId("s-1"), Some("Reviewer A".to_string()))
    Then AgentViewStore.role_for(&SessionId("s-1")) equals Some("Reviewer A")
    And no backend task is spawned in response

  @list-providers
  @dispatch
  Scenario: Action::ListProvidersLoaded folds the provider list into the open ModelSelectorDialog
    Given an App with a ModelSelectorDialog already pushed onto the Compositor against SessionId("s-1") with empty provider list
    When the App dispatches Action::ListProvidersLoaded(vec![ProviderInfo { key: "openai", display_name: "OpenAI", models: vec![ModelEntry{ id: "gpt-5.1-codex", display_name: "gpt-5.1-codex", context_window: 200_000, supports_reasoning: true, supports_vision: false, is_custom: false }]}])
    Then the ModelSelectorDialog's provider list has length 1 with key "openai"

  @bootstrap
  @session-role-loaded
  Scenario: Action::SessionCreated triggers a backend.get_session_role spawn that fills AgentViewStore.role_by_session
    Given an App attached to an EmbeddedFspecBackend with a session manager that returns Some("Reviewer A") from get_session_role
    When the App dispatches Action::SessionCreated(SessionId("s-9"))
    Then refresh_session_chrome(SessionId("s-9")) is called
    And a tokio task is spawned that calls backend.get_session_role(SessionId("s-9"))
    When the spawned task completes
    Then Action::SessionRoleLoaded(SessionId("s-9"), Some("Reviewer A".to_string())) is dispatched
    And AgentViewStore.role_for(&SessionId("s-9")) equals Some("Reviewer A")

  @no-session-manager
  Scenario: Action::ModelSelected against a service with no session manager is a silent no-op
    Given an App attached to an EmbeddedFspecBackend wrapping a SharedFspecService with NO session manager attached
    And no open sessions
    When the App dispatches Action::ModelSelected(SessionId("any"), "openai", "gpt-5.1-codex")
    Then no panic occurs
    And no spawned task fails

  @line-budget
  @source-shape
  Scenario: dispatch_rpc022.rs stays under 300 lines
    Given the file codelet/fspec-tui/src/app/dispatch_rpc022.rs after RPC-022 lands
    When a test counts the line-count of the file
    Then the file has fewer than 300 lines
