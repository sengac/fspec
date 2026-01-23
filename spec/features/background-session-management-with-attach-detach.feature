@session
@done
@session-management
@napi
@NAPI-009
Feature: Background Session Management with Attach/Detach

  """
  Architecture notes:
  - SessionManager is a Rust singleton owning HashMap<Uuid, Arc<BackgroundSession>>
  - Each BackgroundSession spawns a tokio task running agent_loop that waits on mpsc channel for input
  - Attach stores ThreadsafeFunction callback; detach clears it but session continues
  - AgentView.tsx refactored to use NAPI session bindings instead of direct CodeletSession ownership
  - Integrates with existing persistence system (persistenceStoreMessageEnvelope) for session recovery
  - Output buffering uses RwLock<Vec<StreamChunk>> unbounded buffer for full session history
  - Key files: codelet/napi/src/session_manager.rs, src/tui/components/AgentView.tsx
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Sessions must continue executing agent tasks when detached
  #   2. Output produced while detached must be buffered and available on reattach
  #   3. Multiple sessions can run concurrently in background
  #   4. Only one session can be attached at a time per AgentView
  #   5. Session state must persist to disk for recovery
  #   6. Each session runs in its own tokio task with isolated memory
  #   7. Interrupt must work on both attached and detached sessions
  #
  # ========================================

  Background: User Story
    As a developer using fspec TUI
    I want AI agent sessions to run in the background with attach/detach capability
    So that I can multitask between sessions without interrupting agent execution, similar to tmux/screen for terminal sessions

  Scenario: Session creation with persistence ID
    Given I have a persistence session ID
    When I create a background session with that ID
    Then the session is created with the persistence ID


  Scenario: Detach while agent is running
    Given I have an active session running in AgentView
    When I press ESC and select "Detach" from the modal
    Then the session is detached (callback removed)
    And the session continues running in background


  Scenario: Destroy session from modal
    Given I have an active session running in AgentView
    When I press ESC and select "Close Session" from the modal
    Then the session is destroyed


  Scenario: List background sessions with status in resume view
    Given I have detached sessions running in background
    When I view /resume
    Then I see background sessions with their status


  Scenario: Show buffered output when attaching to detached session
    Given I have a session that ran while I was detached
    When I select the session from /resume
    Then I see all the buffered output
    And I can attach to receive live streaming


  Scenario: Restore messages when attaching to session
    Given I have a session with persisted conversation history
    When I attach to the session via /resume
    Then the messages are restored to the session


  Scenario: Restore token state when attaching to session
    Given I have a session with persisted token usage
    When I attach to the session via /resume
    Then the token state is restored to the background session


  Scenario: Send input with thinking config
    Given I have an attached session
    When I send input with thinking config
    Then the input is sent with thinking config to the background session


  Scenario: Send input without thinking config
    Given I have an attached session
    When I send input without thinking config
    Then the input is sent without thinking config


  Scenario: Interrupt a running session
    Given I have a session that is currently running
    When I send an interrupt signal
    Then the session is interrupted
    And the status changes to idle after interrupt completes


  Scenario: Full detach and reattach workflow
    Given I create a session with persistence ID
    And I attach a callback for streaming
    And I send input to start the agent (with thinking config)
    When I detach (ESC + Detach)
    Then the session continues running
    And the session appears in the list
    When I reattach via /resume
    Then I can continue the conversation

