@AMGR-008
Feature: Remove old supervisor infrastructure

  """
  session_manager.rs is the main file (~7000 lines). Removal targets are spread across: supervisor_agent_loop fn (~100 lines at ~L5710), ObservationBuffer struct/impl (~100 lines at ~L383), evaluate_and_maybe_inject (~100 lines at ~L570), SupervisorRole/SupervisorInput structs (~100 lines at ~L259), format_supervisor_input (~30 lines at ~L360), NAPI functions session_create_supervisor/supervisor_inject (~100 lines at ~L6920), and supervisor-specific tests (~500 lines at ~L2770+)
  TUI removal targets: src/tui/components/SupervisorTemplateList.tsx, src/tui/components/SupervisorCreateView.tsx, src/tui/components/SupervisorTemplateForm.tsx, src/tui/types/supervisorTemplate.ts, src/tui/utils/supervisorTemplateStorage.ts. In AgentView.tsx: remove imports and /supervisor command handler. In slashCommands.ts: remove supervisor entry.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. supervisor_agent_loop, ObservationBuffer, format_evaluation_prompt, evaluate_and_maybe_inject, SupervisorRole struct, SupervisorInput struct, create_supervisor_session_with_id, session_create_supervisor, supervisor_inject, format_supervisor_input — all removed from session_manager.rs
  #   2. SupervisorInputImage, StreamChunk::SupervisorInput variant, and supervisor_input_with_images — all removed from types.rs
  #   3. BackgroundSession loses supervisor_input_tx, supervisor_input_rx fields, receive_supervisor_input method, and supervisor_input_sender method. Role field stays but changes from Option<SupervisorRole> to Option<String>
  #   4. TUI removes: /supervisor command handler, SupervisorTemplateList component, SupervisorCreateView component, SupervisorTemplateForm component, supervisorTemplate.ts types, supervisorTemplateStorage.ts utils
  #   5. ChainOfCommand stays but is simplified — tracks spawner→spawned ownership only for close permission checks and list/get_status. No observation streaming.
  #   6. broadcast::channel on BackgroundSession stays — still needed for TUI display of session output. Supervisors no longer subscribe to it for observation.
  #   7. SplitSessionView stays (still needed for split pane display) but references to supervisor_agent_loop observation pipeline are removed. Correlation mapping utils stay (still used for cross-pane highlighting).
  #   8. SupervisorRoleInfo NAPI type and session_get_role, session_set_role NAPI functions are simplified — role becomes a plain string instead of SupervisorRole struct with auto_inject/breakpoint_config
  #   9. Test files removed: watcher_interjection_test.rs tests for ObservationBuffer/SupervisorRole/breakpoint detection. All supervisor-specific tests in session_manager.rs test module removed. message_duplication_test.rs tests referencing TestSupervisorInput updated or removed.
  #   10. index.d.ts type declarations updated to remove SupervisorRoleInfo, SupervisorInputImage, StreamChunk SupervisorInput variant, session_create_supervisor, supervisor_inject exports
  #   11. Conversation type 'supervisor-input' stays in TUI (messages from other sessions still need display). The role 'supervisor' in ConversationLine stays. chunkProcessor.ts parseSupervisorPrefix stays. These are display concerns, not pipeline.
  #
  # EXAMPLES:
  #   1. cargo build succeeds with zero errors after all removals — no dangling references to removed types/functions
  #   2. cargo test passes — all remaining tests pass, supervisor-specific test modules are gone
  #   3. grep -r 'supervisor_agent_loop\|ObservationBuffer\|SupervisorInput\|format_evaluation_prompt\|SupervisorRole' codelet/napi/src/ returns zero matches (except comments explaining what was removed)
  #   4. TUI launches, /supervisor command no longer exists in slash command list, no supervisor template views reachable
  #   5. ChainOfCommand.add_supervisor and ChainOfCommand.get_subordinates still work for ownership tracking — tested via existing unit tests
  #   6. Regular agent_loop sessions (non-supervisor) continue to work identically — creating a normal session, sending a prompt, receiving streaming response all unchanged
  #
  # ========================================

  Background: User Story
    As a developer
    I want to remove old supervisor infrastructure code
    So that the codebase is clean for the new AgentManager tool implementation

  Scenario: Rust codebase compiles after supervisor removal
    Given the supervisor_agent_loop function has been removed from session_manager.rs
    And the ObservationBuffer struct and impl have been removed
    And the SupervisorRole and SupervisorInput structs have been removed
    And the format_evaluation_prompt and evaluate_and_maybe_inject functions have been removed
    And the format_supervisor_input function has been removed
    And the create_supervisor_session_with_id method has been removed from SessionManager
    And the session_create_supervisor and supervisor_inject NAPI functions have been removed
    And the SupervisorInputImage struct and StreamChunk::SupervisorInput variant have been removed from types.rs
    And the supervisor_input_tx and supervisor_input_rx fields have been removed from BackgroundSession
    And the receive_supervisor_input and supervisor_input_sender methods have been removed from BackgroundSession
    When I run cargo build
    Then the build should succeed with zero errors

  Scenario: All Rust tests pass after supervisor removal
    Given all supervisor-specific production code has been removed
    And the supervisor-specific test modules in session_manager.rs have been removed
    And the watcher_interjection_test.rs file has been removed or updated
    And the message_duplication_test.rs TestSupervisorInput references have been updated
    When I run cargo test
    Then all remaining tests should pass

  Scenario: No supervisor infrastructure references remain in Rust source
    Given all supervisor infrastructure has been removed from session_manager.rs and types.rs
    When I search for supervisor_agent_loop in codelet/napi/src/
    And I search for ObservationBuffer in codelet/napi/src/
    And I search for SupervisorInput in codelet/napi/src/
    And I search for format_evaluation_prompt in codelet/napi/src/
    And I search for SupervisorRole in codelet/napi/src/
    Then no matches should be found in production code

  Scenario: TUI supervisor command and views removed
    Given the /supervisor entry has been removed from slashCommands.ts
    And the SupervisorTemplateList component file has been deleted
    And the SupervisorCreateView component file has been deleted
    And the SupervisorTemplateForm component file has been deleted
    And the supervisorTemplate.ts types file has been deleted
    And the supervisorTemplateStorage.ts utils file has been deleted
    And the supervisor imports and command handler have been removed from AgentView.tsx
    When the TUI builds successfully
    Then the /supervisor command should not appear in slash command autocomplete

  Scenario: ChainOfCommand ownership tracking still works
    Given the ChainOfCommand data structure has been preserved
    And observation streaming through ChainOfCommand has been removed
    When a supervisor-subordinate relationship is tracked via add_supervisor
    Then get_subordinates should return the correct subordinate sessions
    And the ownership relationship is used for close permission checks

  Scenario: Regular agent_loop sessions work unchanged
    Given the supervisor pipeline has been removed
    And the regular agent_loop function is unchanged
    When a normal session is created via the standard path
    Then the session runs agent_loop as before
    And streaming responses work identically
    And the broadcast channel still emits chunks for TUI display

  Scenario: Role simplified from struct to plain string
    Given the SupervisorRole struct has been replaced by Option<String> on BackgroundSession
    And the SupervisorRoleInfo NAPI type has been simplified to return a plain string
    When session_get_role is called on a session with a role set
    Then it returns the role as a simple string
    And auto_inject and breakpoint_config fields no longer exist

  Scenario: NAPI type declarations updated
    Given the supervisor infrastructure has been removed from Rust code
    When index.d.ts is regenerated
    Then SupervisorRoleInfo should be simplified to a string role type
    And SupervisorInputImage type should not exist
    And StreamChunk should not have a SupervisorInput variant
    And session_create_supervisor function should not be exported
    And supervisor_inject function should not be exported
