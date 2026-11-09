@done
@BUG-407
@wt-004
@REFAC-053
@BUG-398
@session-management
@bug-fix
@session
@tools
@git-integration
Feature: WT-004 tool-callbacks wiring: work-unit stage without a work unit, block notifications and build_service registration
  """
  Split from isolation-path-validation-callback-is-never-registered-in-the-fspec-binary-isolated-sessions-are-not-actually-pinned-to-their-worktree.feature by REFAC-053 so each feature has one test file under 300 lines. Test file: rust/sessions/tests/wt004_tool_callbacks_wiring.rs; shared helpers: rust/sessions/tests/wt004_session_tool_callbacks_support/mod.rs.
  codelet_sessions::session_tool_callbacks exposes work_unit_stage (session -> work unit context status) and emit_block_notification (UserNotification(Warning) chunk "AI was blocked from {action} - {reason}" on the footer_poller chunk-sender slot); build_service (rust/fspec/src/common.rs) calls register_manager with its non-singleton SessionManager and registers the three shared callbacks into codelet-tools' OnceLocks after the manager is constructed and before the service is returned.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The fspec binary's build_service MUST register its non-singleton SessionManager as the tool-callbacks manager and wire the three shared callbacks before any session can be created.
  #   2. The work-unit-stage lookup returns the session's work unit context status, or no stage when the session has no work unit context.
  #   3. A blocked tool action emits a UserNotification(Warning) chunk "AI was blocked from {action} - {reason}" on the registered manager's chunk stream.
  #   4. BUG-398/BUG-405: the tool-callback session lookup consults only the registered manager; with none registered the lookup is refused. SessionManager::instance() no longer exists.
  #   5. BUG-398: each scenario sets up the process state it needs (tool callbacks installed, data directory set) itself and never relies on a sibling test.
  #
  # EXAMPLES:
  #   1. A session without a work unit context has no work unit stage.
  #   2. A block notification for "writing /project/src/lib.rs" with reason "path outside worktree" arrives as a Warning UserNotification with that exact message.
  #   3. build_service's source registers the manager and all three callbacks between constructing the SessionManager and returning the service.
  #   4. BUG-398/BUG-405: with the callbacks installed but no manager registered, the isolation lookup for an isolated session in an unregistered manager is refused.
  #
  # ========================================
  Background: User Story
    As a developer running the fspec binary (combined/daemon mode)
    I want to create an isolated (worktree) session and have every per-session tool honor that isolation
    So that the worktree is an actual boundary, not a cosmetic badge

  Scenario: build_service registers the tool-callbacks manager and all three tool callbacks
    Given the fspec binary crate source after this fix
    When build_service is inspected
    Then it calls session_tool_callbacks::register_manager with its non-singleton SessionManager
    And it registers the shared isolation_context function as the effective-cwd callback
    And it registers the shared work_unit_stage function as the work-unit-stage callback
    And it registers the shared emit_block_notification function as the block-notification callback
    And the registration happens after the SessionManager is constructed and before the service is returned

  Scenario: Work unit stage lookup returns None for sessions without a work unit
    Given a non-singleton session manager registered as the tool-callbacks manager
    And a session without a work unit context
    When the tool layer asks for that session's work unit stage
    Then it returns no stage

  Scenario: A blocked tool action emits a warning notification on the manager's chunk stream
    Given a non-singleton session manager registered as the tool-callbacks manager with its chunk sender registered
    And a subscriber on that manager's chunks broadcast
    When a block notification is emitted for a session with an action and a reason
    Then a UserNotification chunk with Warning severity is delivered on the broadcast
    And its message is "AI was blocked from {action} - {reason}"

  Scenario: With no manager registered the isolation lookup is refused
    Given the tool callbacks are installed the way build_service installs them
    And no tool-callbacks manager is registered
    And an isolated session created in a second, unregistered manager against a fresh git repository
    When the tool layer resolves the isolation context for that session id
    Then the lookup fails instead of finding the unregistered manager's session

  Scenario: The shared callbacks are accepted as the tool layer's plain function pointers
    Given the shared isolation, stage and block-notification functions
    When they are bound to the codelet-tools callback types and called with a malformed session id
    Then the isolation and stage callbacks both refuse the lookup
