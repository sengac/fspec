@done
@pause-integration
@bug-fix
@rust
@session
@BUG-605
Feature: Destroying or interrupting a session waiting on a question or approval leaves its turn blocked forever
  """
  Reproduced 2026-11-07: interrupting or closing a session whose turn waits on a question or approval does end the turn (the stream loop drops the running tool and records 'Tool execution cancelled by user interruption'), but nothing answers the wait: the tool's thread stays blocked in a std mpsc recv forever, holding the session, and the question or approval stays pending on the session, so a stale prompt remains answerable after the turn is over. Fix in codelet-sessions: interrupt(), which destroy_session calls, calls cancel_pending_interactions, which answers a pending question {cancelled: true} and a pending approval Interrupted under the InteractionSlot lock and sends nothing when no request is pending (InteractionSlot::cancel_pending); wait_for_hitl_response and wait_for_pause_response cancel a request raised after the interrupt flag was set (by interrupt() or the loop detector) instead of waiting; resume_after_interaction moves the session from Paused to Running only, so a wait answered after the turn ended never marks an idle session Running. agent-loop: the per-turn pause and HITL handlers call resume_after_interaction after the wait, and the HITL handler no longer clears the request itself (the answer or the cancel already did, so a late wake can never clear a newer question).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Interrupting a session answers the question it is waiting on with the cancelled response ({"cancelled": true}) and the approval it is waiting on with Interrupted, which every approval treats as a denial; the session's turn then finishes (product owner, 2026-11-07, decision 4)
  #   2. Destroying (closing) a session interrupts it, so a question or approval it is waiting on is resolved the same way and its turn finishes
  #   3. A question or approval raised after the session was interrupted, before its turn ended, is resolved as cancelled at once and never waits
  #   4. Only a pending request is resolved: interrupting a session that is not waiting sends no response, so no stale answer is left for a later question or approval
  #   5. A wait answered after its turn already ended leaves the session's status as the turn set it: answering moves a session from Paused to Running only
  #
  # EXAMPLES:
  #   1. A session is interrupted and, before its turn ends, a tool asks a question; the question is answered cancelled without waiting
  #   2. A session not waiting on anything is interrupted, then its next turn asks a question; the question waits for a real answer instead of reading a leftover cancelled response
  #   3. A session's turn calls request_user_input; while it waits, the session is interrupted; the question's wait returns {"cancelled": true}, the turn finishes and no question is left pending
  #   4. A session's turn runs Bash 'echo bug605 git reset --hard', which the blocklist holds for approval; while it waits, the session is closed; the approval's wait returns Interrupted, the turn finishes and no approval is left pending
  #
  # ========================================
  Background: User Story
    As a user closing or interrupting a session
    I want to have a question or approval the session is waiting on resolved as cancelled
    So that the session's turn finishes instead of staying blocked forever

  Scenario: Interrupting a session waiting on a question answers it cancelled
    Given a session waiting for the answer to a question
    When the session is interrupted
    Then the wait returns {"cancelled": true}
    And the session has no pending question

  Scenario: Interrupting a session waiting on an approval answers it Interrupted
    Given a session waiting for the answer to an approval
    When the session is interrupted
    Then the wait returns Interrupted
    And the session has no pending approval

  Scenario: A question or approval raised after the session was interrupted resolves at once
    Given a session that was interrupted and has not started a new turn
    When it asks a question
    Then the wait returns {"cancelled": true} without anyone answering
    When it asks for an approval
    Then the wait returns Interrupted without anyone answering

  Scenario: Closing a session whose turn waits on an approval ends the turn with nothing pending
    Given a session whose turn runs Bash "echo bug605 git reset --hard" and is waiting for the blocklist approval
    When the session is closed
    Then the session's turn finishes
    And the session has no pending approval and is neither paused nor running

  Scenario: Interrupting a session whose turn waits on a question ends the turn with nothing pending
    Given a session whose turn called request_user_input and is waiting for the answer
    When the session is interrupted
    Then the session's turn finishes
    And the session has no pending question and is neither paused nor running

  Scenario: Interrupting a session that is not waiting leaves no answer behind
    Given a session that was interrupted while it was not waiting on a question or approval
    When its next turn calls request_user_input
    Then the session waits for the answer
    And the answer the user gives is the request_user_input tool result
