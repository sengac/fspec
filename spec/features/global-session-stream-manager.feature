@done
@session-management
@tui
@REFAC-008
Feature: Global Session Stream Subscription for FspecCommandRequest Handling
  """
  GlobalSessionStreamManager is the SOLE subscriber - it owns the global chunk callback
  for ALL sessions and multiplexes events to registered handlers. AgentView does NOT
  call NAPI directly for streaming - it registers handlers with the manager via hook.

  The global callback (sessionSetGlobalChunkCallback) receives chunks from ALL sessions
  with (sessionId, chunk) and routes to appropriate handlers.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. GlobalSessionStreamManager must subscribe to ALL active sessions, not just the displayed one
  #   2. FspecCommandRequest handling must be extracted from AgentView into a dedicated handler
  #   3. AgentView must contain ONLY UI rendering logic - no event handling business logic
  #   4. Session subscriptions must be added on session creation and removed on session destruction
  #   5. Event handlers must be composable via registerHandler/unregisterHandler pattern
  #   6. AgentView must NOT call NAPI streaming functions directly - it registers with GlobalSessionStreamManager
  #   7. Tests MUST use real NAPI bindings and fixtures - NO MOCKS for GlobalSessionStreamManager
  #   8. Tests MUST use universal-test-setup.ts from src/test-helpers for temp directories and automatic cleanup
  #
  # ARCHITECTURE DECISION:
  #   GlobalSessionStreamManager is the SOLE subscriber (Option B).
  #   - Global callback receives all chunks from all sessions
  #   - Manager owns the callback and forwards events to registered handlers
  #   - AgentView registers via useSessionStreamManager() hook
  #   - FspecCommandHandler is a global handler that processes FspecCommandRequest from any session
  #
  # EXAMPLES:
  #   1. User starts Session A, sends message invoking fspec tool, then navigates to Session B - fspec command in Session A completes successfully
  #   2. User is on BoardView with 3 detached sessions running - all 3 can invoke fspec tools concurrently without deadlock
  #   3. New session created - GlobalSessionStreamManager automatically subscribes to it
  #   4. Session destroyed - GlobalSessionStreamManager automatically unsubscribes from it
  #   5. AgentView receives Text/Thinking/ToolCall chunks for UI rendering - does NOT handle FspecCommandRequest
  #
  # ========================================
  Background: User Story
    As a developer
    I want to have fspec commands execute successfully from detached sessions
    So that agents running in background can use fspec tools without deadlocking

  # ===========================================
  # Scenario 1: Fspec command completes in detached session
  # ===========================================
  @integration
  Scenario: Fspec command completes successfully after user navigates away
    Given I have Session A running with an agent
    And I send a message that invokes the fspec tool in Session A
    When I navigate to Session B before the fspec command completes
    Then the fspec command in Session A should complete successfully
    And Session A should not deadlock

  # ===========================================
  # Scenario 2: Multiple concurrent fspec invocations
  # ===========================================
  @integration
  Scenario: Multiple detached sessions can invoke fspec tools concurrently
    Given I have 3 detached sessions running agents
    And each session sends a message invoking the fspec tool
    When I am viewing the BoardView
    Then all 3 fspec commands should complete successfully
    And no sessions should deadlock

  # ===========================================
  # Scenario 3: Auto-subscription on session creation
  # ===========================================
  @unit
  Scenario: GlobalSessionStreamManager subscribes to new sessions automatically
    Given the GlobalSessionStreamManager is initialized
    When a new session is created
    Then the GlobalSessionStreamManager should subscribe to the new session
    And the session should be tracked in the subscriptions map

  # ===========================================
  # Scenario 4: Auto-unsubscription on session destruction
  # ===========================================
  @unit
  Scenario: GlobalSessionStreamManager unsubscribes when session is destroyed
    Given the GlobalSessionStreamManager is initialized
    And a session exists with an active subscription
    When the session is destroyed
    Then the GlobalSessionStreamManager should unsubscribe from the session
    And the session should be removed from the subscriptions map

  # ===========================================
  # Scenario 5: AgentView receives only UI events
  # ===========================================
  @unit
  Scenario: AgentView receives UI chunks but not FspecCommandRequest
    Given the GlobalSessionStreamManager is handling events for a session
    And AgentView is displaying that session
    When the session emits a Text chunk
    Then AgentView should receive the Text chunk for UI rendering
    When the session emits a FspecCommandRequest chunk
    Then AgentView should NOT receive the FspecCommandRequest chunk
    And the GlobalSessionStreamManager should handle the FspecCommandRequest

  # ===========================================
  # Scenario 6: Tests use real NAPI bindings without mocks
  # ===========================================
  @unit
  Scenario: Tests use real NAPI bindings without mocks
    Given the test environment using universal-test-setup.ts for temp directories
    When a test creates a session via persistenceCreateSessionWithProvider
    And subscribes to it via GlobalSessionStreamManager
    And simulates a FspecCommandRequest chunk via simulateChunk
    Then the fspec command should execute successfully
    And no mocks should be used for GlobalSessionStreamManager
    And temp directories should be automatically cleaned up
