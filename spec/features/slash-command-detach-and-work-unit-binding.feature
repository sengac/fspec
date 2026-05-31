@done
@session-management
@rust
@multi-session
@rpc
@agent-view
@tui
@slash-command
@RPC-050
Feature: /detach slash command and work-unit context teardown
  """
  Phase 6.4 of the RPC-030 roadmap. `SlashCommandAction::Detach`
  currently lands in `handle_slash_command`'s `other => notice not yet
  implemented` catch-all. RPC-050 replaces that with a concrete arm
  that calls backend.set_work_unit_context(session_id, None), mirroring
  the TS AgentView.tsx detachFromWorkUnit + prepareForNewSession flow.

  On Ok the dispatcher dispatches Action::WorkUnitDetached(session)
  which clears the per-session binding in AgentViewStore, resets the
  focused session's scrollback, and resets the per-session TokenState.

  On Err the dispatcher emits a `[error] /detach failed: {reason}`
  notice into the originating session's scrollback via the existing
  Action::EmitSessionNotice path and preserves the local binding so
  the user can retry without losing state.

  Companion to spec/features/work-unit-attach-binding.feature which
  holds the BoardView attach side of the same binding.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SlashCommandAction::Detach with an active session AND a bound work unit calls backend.set_work_unit_context(session, None) and on Ok dispatches Action::WorkUnitDetached(session) which clears the binding, resets scrollback and per-session TokenState
  #   2. /detach with NO active session is a silent no-op
  #   3. /detach with an active session but NO work unit attached emits `[notice] /detach: no work unit attached` and DOES NOT call the backend
  #   4. On Err from backend.set_work_unit_context(None), the dispatcher emits `[error] /detach failed: {reason}` and does NOT reset local state
  #
  # ========================================
  Background: User Story
    As a fspec user with an AgentView session bound to a work unit
    I want to use `/detach` to clear the binding and reset my conversation state
    So that I can start a fresh session against the same work unit without restarting the TUI — mirroring the TS AgentView.tsx detachFromWorkUnit flow

  Scenario: /detach with a bound work unit clears the binding, resets scrollback and TokenState
    Given an App with open session s-1 bound to AUTH-001
    And s-1's scrollback has 3 chunks
    And s-1's TokenState has input_tokens=42 and output_tokens=7
    And the MockBackend's set_work_unit_context returns Ok(())
    When SlashCommandSelected(SlashCommandAction::Detach) is dispatched
    Then within 1 second backend.set_work_unit_context is called exactly once with (s-1, None)
    And within 1 second AgentViewStore.work_unit_context_for(s-1) returns None
    And within 1 second s-1's scrollback chunk_count becomes 0
    And within 1 second s-1's TokenState equals TokenState::default()

  Scenario: /detach with no active session is a silent no-op
    Given an App with NO open session
    When SlashCommandSelected(SlashCommandAction::Detach) is dispatched
    Then backend.set_work_unit_context is NEVER called
    And no scrollback chunk is appended to any session

  Scenario: /detach with a session but no work unit attached emits a notice
    Given an App with open session s-1 NOT bound to any work unit
    When SlashCommandSelected(SlashCommandAction::Detach) is dispatched
    Then within 1 second s-1's scrollback contains a chunk whose text equals "[notice] /detach: no work unit attached"
    And backend.set_work_unit_context is NEVER called

  Scenario: /detach failure surfaces an error notice and preserves local state
    Given an App with open session s-1 bound to AUTH-001
    And s-1's scrollback has 3 chunks
    And the MockBackend's set_work_unit_context returns Err("corrupt manifest")
    When SlashCommandSelected(SlashCommandAction::Detach) is dispatched
    Then within 1 second s-1's scrollback contains a chunk whose text equals "[error] /detach failed: corrupt manifest"
    And AgentViewStore.work_unit_context_for(s-1) still returns Some(ctx) with id "AUTH-001"
