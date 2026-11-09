@BUG-582
@done
@session-management
@rust
@session
@daemon
@persistence
@input
@concurrency
@BUG-554
Feature: Starting a turn clears only the submitted draft
  """
  codelet-sessions only, no RPC or wire change. BackgroundSession gains clear_pending_input_if(submitted: &str): under the pending_input write lock it sets None only when the draft equals submitted. start_turn calls it with the turn's input instead of set_pending_input(None) (covers send_input, idle submit_input, resume_queued, steers joined into a turn, loop and schedule fires). submit_input calls it for the busy steer and queue branches. set_pending_input, get_pending_input and every transport stay as they are, so the RPC-037 cross-transport parity is unchanged. Tests drive a real BackgroundSession whose input channel has no agent loop behind it (RPC-409 shape), so a send is fully deterministic.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Starting a turn clears the session's pending-input draft only while that draft is exactly the text of the turn being started (compare-and-clear under the draft lock); any other draft is a newer one and is kept
  #   2. Steering into a running turn and queueing behind it follow the same rule at submission: the draft is cleared only when it is exactly the submitted text
  #   3. Turns the user did not submit from a draft (a queued input resumed when the running turn ends, steers joined into a new turn, a loop or schedule firing) never clear a draft that holds different text
  #   4. Consistent with BUG-549: the TUI ends a submitted draft itself with an immediate clear ordered before any newer save, so a draft the daemon holds after that clear is always newer than the submission and must survive
  #
  # EXAMPLES:
  #   1. The user submits 'deploy the fix' and, before the slow send reaches the daemon, the TUI has saved the new draft 'next idea': when the turn starts the daemon's draft is still 'next idea'
  #   2. Another client left the draft 'deploy the fix' and sends 'deploy the fix': when the turn starts the daemon's draft is empty
  #   3. The session is running and its draft is 'next idea'; steering 'also check the logs' leaves 'next idea'; steering 'next idea' clears it
  #   4. The session is running and its draft is 'next idea'; queueing 'task two' leaves 'next idea'; when the turn ends and 'task two' starts, the draft is still 'next idea'
  #
  # ========================================
  Background: User Story
    As a user typing a new prompt while my previous one is still being sent
    I want to have the daemon clear only the draft I submitted when it starts the turn, steers it or queues it
    So that the new draft I started typing survives a slow send and is still there after a reattach

  # ─────────────────────────────────────────────────────────────────────
  # A new turn (rules 1 and 4)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: A draft saved while a slow send was in flight survives the turn start
    Given an idle session whose draft was cleared when "deploy the fix" was submitted
    And the new draft "next idea" was saved before the send reached the daemon
    When the send of "deploy the fix" starts the turn
    Then the session's draft is "next idea"
    And a client attaching to the session reads the draft "next idea"

  Scenario: Starting a turn clears the draft that was submitted
    Given an idle session whose draft is "deploy the fix"
    When the send of "deploy the fix" starts the turn
    Then the session has no draft

  # ─────────────────────────────────────────────────────────────────────
  # Steer and queue (rule 2)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Steering a running turn keeps a newer draft and clears the submitted one
    Given a running session whose draft is "next idea"
    When "also check the logs" is steered into the running turn
    Then the session's draft is "next idea"
    When "next idea" is steered into the running turn
    Then the session has no draft

  Scenario: Queueing behind a running turn keeps a newer draft and clears the submitted one
    Given a running session whose draft is "next idea"
    When "task two" is queued behind the running turn
    Then the session's draft is "next idea"
    When "next idea" is queued behind the running turn
    Then the session has no draft

  # ─────────────────────────────────────────────────────────────────────
  # Turns not started from the draft (rule 3)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: A queued input starting after the running turn keeps the current draft
    Given a running session with "task two" queued behind its turn
    And the session's draft is "next idea"
    When the running turn ends and "task two" starts as the next turn
    Then the session's draft is "next idea"
