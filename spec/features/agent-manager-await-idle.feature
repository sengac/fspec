@done
@AMGR-015
Feature: AgentManager await_idle action — efficient blocking wait for subordinate agents to finish
  """
  The handler in agent_manager_handler.rs must subscribe to each target session's supervisor_broadcast channel (tokio::broadcast::Receiver<StreamChunk>) and use tokio::select! to wait for SessionStateChange(Idle) events rather than polling get_status()
  The handler must be async — unlike other AgentManager actions which are sync closures, await_idle needs to .await on broadcast receivers and tokio::time::timeout. The AgentManagerHandler type signature may need to return a Future or the handler dispatch must special-case await_idle as async
  For multi-session await, spawn one tokio::spawn per target session, each subscribing to that session's broadcast channel. Use tokio::select! with a shared deadline (Instant + Duration) to cancel all remaining waiters when timeout expires. Use a JoinSet or select_all pattern.
  The call() method in AgentManagerTool (mod.rs) currently calls execute_agent_manager() synchronously. For await_idle, it needs an async path. Options: (A) make execute_agent_manager async, (B) add a separate execute_agent_manager_async for await_idle, (C) pass a oneshot::Sender into the handler and await the receiver in call(). Option B is cleanest — minimal blast radius.
  The interrupt check should use the calling session's interrupt_notify (Arc<Notify>) as a cancellation signal in the tokio::select! alongside the broadcast receivers and timeout
  Result type: add AwaitResult variant to AgentManagerResult containing results: Vec<AwaitSessionResult> where each entry has { session_id, status: idle|timed_out|destroyed|interrupted }
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The action name must be `await_idle` and be added to the AgentManagerAction enum alongside spawn/list/get_status/close/message/set_role
  #   2. The `session_id` parameter accepts either a single session ID string or an array of session ID strings to await multiple agents simultaneously
  #   3. An optional `timeout` parameter (in seconds) sets a maximum wait duration; if omitted, waits indefinitely until all sessions are idle
  #   4. Must use notification-based waiting (subscribe to supervisor_broadcast channel for SessionStateChange events), NOT polling with sleep
  #   5. If a target session is already idle at call time, it is immediately resolved without waiting
  #   6. The result must report which sessions became idle and which timed out, using a structured JSON response
  #   7. If a target session is destroyed while being awaited, it counts as resolved (not an error) with a `destroyed` status
  #   8. The await must be interruptible — if the calling session is interrupted (Esc), the await should cancel and return partial results
  #   9. Non-existent session IDs must return an immediate error (session_not_found), not wait until timeout
  #   10. Pre-tool hooks (HOOK-013) must be checked before execution, consistent with all other AgentManager actions
  #
  # EXAMPLES:
  #   1. Supervisor spawns 3 workers, sends tasks, calls await_idle with all 3 session IDs, all 3 finish within timeout → result shows all 3 as `idle`
  #   2. Supervisor calls await_idle on a single session that is already idle → returns immediately with `idle` status, no wait
  #   3. Supervisor calls await_idle with timeout=10 on a session that takes 30s → after 10s returns with that session marked as `timed_out`
  #   4. Supervisor calls await_idle on 3 sessions, 2 finish quickly and 1 times out → result shows 2 as `idle` and 1 as `timed_out`
  #   5. Supervisor calls await_idle with a non-existent session ID → immediate error response with session_not_found code
  #   6. Supervisor calls await_idle on a session that gets destroyed mid-wait → that session resolves as `destroyed`, remaining sessions continue being awaited
  #   7. User presses Esc while supervisor is awaiting 3 sessions (1 already idle, 2 still running) → returns partial result: 1 idle, 2 interrupted
  #   8. Supervisor calls await_idle with timeout omitted → waits indefinitely until session becomes idle
  #   9. Supervisor calls await_idle with a single string session_id (not array) → works the same as passing a single-element array
  #
  # ========================================
  Background: User Story
    As a supervisor agent
    I want to await one or more subordinate agents becoming idle
    So that I can coordinate parallel work without wasteful polling or sleep loops

  @happy-path
  Scenario: Await multiple subordinates that all complete within timeout
    Given I have spawned 3 subordinate agent sessions
    And each subordinate has been sent a task and is running
    When I call await_idle with all 3 session IDs
    Then the tool should block until all 3 sessions become idle
    And the result should contain 3 entries each with status "idle"

  @happy-path
  Scenario: Await a single session that is already idle
    Given I have a subordinate agent session that has finished its task
    And the subordinate session status is "idle"
    When I call await_idle with that session ID
    Then the tool should return immediately without waiting
    And the result should contain 1 entry with status "idle"

  @timeout
  Scenario: Timeout expires before session becomes idle
    Given I have a subordinate agent session that is actively running a long task
    When I call await_idle with that session ID and timeout of 10 seconds
    And the subordinate does not finish within 10 seconds
    Then the result should contain 1 entry with status "timed_out"

  @timeout
  Scenario: Mixed results when some sessions finish and others timeout
    Given I have spawned 3 subordinate agent sessions
    And 2 subordinates will finish quickly
    And 1 subordinate is running a long task
    When I call await_idle with all 3 session IDs and timeout of 10 seconds
    Then the result should contain 2 entries with status "idle"
    And the result should contain 1 entry with status "timed_out"

  @error-handling
  Scenario: Non-existent session ID returns immediate error
    Given I have a session ID that does not correspond to any active session
    When I call await_idle with that non-existent session ID
    Then the tool should return immediately with an error
    And the error code should be "session_not_found"

  @edge-case
  Scenario: Session destroyed during await resolves as destroyed
    Given I have a subordinate agent session that is running
    When I call await_idle with that session ID
    And the subordinate session is destroyed while being awaited
    Then the result should contain 1 entry with status "destroyed"

  @interruption
  Scenario: User interrupt cancels await and returns partial results
    Given I have spawned 3 subordinate agent sessions
    And 1 subordinate has already finished and is idle
    And 2 subordinates are still running
    When I call await_idle with all 3 session IDs
    And the calling session is interrupted before the running sessions finish
    Then the result should contain 1 entry with status "idle"
    And the result should contain 2 entries with status "interrupted"

  @defaults
  Scenario: No timeout by default — waits indefinitely when omitted
    Given I have a subordinate agent session that is running
    When I call await_idle with that session ID and no timeout parameter
    Then the tool should wait indefinitely until the session becomes idle

  @input-format
  Scenario: Single string session_id works the same as single-element array
    Given I have a subordinate agent session that is idle
    When I call await_idle with session_id as a plain string
    Then the result should be identical to calling with a single-element array

  @integration
  Scenario: Pre-tool hooks are checked before await_idle executes
    Given a pre-tool hook is configured to block the AgentManager tool
    When I call await_idle with a valid session ID
    Then the tool should return a blocked error before any waiting occurs
