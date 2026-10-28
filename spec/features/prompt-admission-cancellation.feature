@done
@agent-core
@session-management
@phase-1
@interrupt
@BUG-276
Feature: Preserve Escape interruption received during prompt persistence and startup
  """
  Reuse interruption epochs and define ownership for PromptInput, supervisor input, manual compaction and retries. Test deterministic admission barriers, not a huge recovery scan. Review the post-persistence reset in agent_loop/prompt.rs and the pre-queue reset in BackgroundSession::send_input.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. An input interrupted during queueing, hooks, persistence or startup never reaches provider inference or tool dispatch after that cancellation.
  #   2. A genuinely new user input may start normally, but it cannot clear cancellation ownership for an earlier queued or executing input.
  #   3. Cancellation preserves durable original input and truthful terminal state across persistence failure and cold resume without claiming successful completion.
  #
  # EXAMPLES:
  #   1. While a submitted prompt waits for storage recovery, the user presses Escape; when storage recovers the assistant does not answer or execute any tool for that prompt.
  #   2. After cancelling a blocked prompt, the user submits a new prompt; only the new prompt runs once and the cancelled prompt stays cancelled.
  #
  # ========================================
  Background: User Story
    As a session owner
    I want to cancel submitted input throughout startup
    So that cancelled work never reaches inference or tools

  Scenario: Queued cancellation survives a later user submission
    Given an accepted prompt waiting in the input queue
    When I cancel it and submit a genuinely new prompt
    Then only the new prompt reaches the provider once
    And the cancelled original remains recoverable without successful completion

  Scenario: Cancellation during prompt hooks prevents dispatch
    Given an accepted prompt held by a submission hook
    When I cancel it before the hook releases
    Then no provider or tool is dispatched for that prompt
    And the original input remains recoverable with truthful interruption

  Scenario: Cancellation during persistence survives startup
    Given an accepted prompt held at its durable persistence boundary
    When I cancel it before storage releases
    Then unrelated asynchronous work remains responsive
    And no provider or tool is dispatched for that prompt
    And cold resume retains the original input exactly once

  Scenario: Failed persistence retains cancelled input for recovery
    Given an accepted prompt whose archive is unavailable
    When I cancel it before admission finishes
    Then inference is refused without successful completion
    And explicit storage recovery retains the original input exactly once

  Scenario: Supervisor and recovery inputs retain cancellation ownership
    Given queued supervisor input and staged recovery continuations
    When I cancel them before admission and submit new user input
    Then stale supervisor and recovery work never reaches inference
    And only the genuinely new user input may start normally
