@wip
@session
@rpc
@RPC-409
Feature: Paused SessionStateChange chunk stranded on blocked tokio worker — inline pause prompt never appears

  """
  Root cause (RPC-409): tokio broadcast::Sender::send wakes the subscriber task into the current worker's non-stealable LIFO slot; the agent-loop pause/fspec/hitl handlers then block that worker in a std mpsc recv, stranding the subscriber until the wait resolves. Fix: BackgroundSession wait_for_pause_response / wait_for_fspec_response / wait_for_hitl_response wrap the blocking recv in tokio::task::block_in_place, guarded on RuntimeFlavor::MultiThread with a direct-recv fallback off-runtime. See spec/attachments/RPC-409/investigation.md.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Blocking waits on tokio worker threads (pause/fspec/hitl) must hand off the worker's queues via block_in_place before blocking
  #   2. Chunks emitted immediately before a blocking wait must be delivered to broadcast subscribers while the wait is still pending
  #   3. The wait must still work when called off-runtime (unit tests / plain threads): falls through to a direct blocking recv
  #
  # EXAMPLES:
  #   1. Agent-loop pause handler emits SessionStateChange{Paused} then blocks in wait_for_pause_response on a tokio task; a chunks_rx subscriber receives the Paused chunk within 1s, before any response is sent
  #   2. FspecCommandRequest chunk emitted before wait_for_fspec_response is delivered while the wait is pending (same stranding pattern)
  #   3. SessionStateChange{Paused} emitted before wait_for_hitl_response is delivered while the wait is pending (same stranding pattern)
  #
  # ========================================

  Background: User Story
    As a fspec TUI user
    I want to see the inline tool-approval prompt the moment a tool pauses for permission
    So that I can allow or deny sensitive file access instead of the session hanging forever

  Scenario: Paused chunk reaches subscribers while the pause wait is still pending
    Given a BackgroundSession on a multi-thread tokio runtime with a chunks broadcast subscriber
    When a tokio task runs the agent-loop pause handler which emits SessionStateChange Paused and blocks in wait_for_pause_response
    Then the subscriber receives the Paused chunk within 1 second while the pause is still pending
    Then sending a pause response afterwards unblocks the handler with that response


  Scenario: Fspec request chunk reaches subscribers while the fspec wait is still pending
    Given a BackgroundSession on a multi-thread tokio runtime with a chunks broadcast subscriber
    When a tokio task emits an FspecCommandRequest chunk and blocks in wait_for_fspec_response
    Then the subscriber receives the FspecCommandRequest chunk within 1 second while the wait is still pending
    Then sending an fspec result afterwards unblocks the waiter with that result


  Scenario: Paused chunk reaches subscribers while the HITL wait is still pending
    Given a BackgroundSession on a multi-thread tokio runtime with a chunks broadcast subscriber
    When a tokio task emits SessionStateChange Paused and blocks in wait_for_hitl_response
    Then the subscriber receives the Paused chunk within 1 second while the wait is still pending
    Then sending a HITL response afterwards unblocks the waiter with that response


  Scenario: Waits fall back to a direct blocking recv when called off-runtime
    Given a BackgroundSession and a plain OS thread outside any tokio runtime context
    When the thread calls wait_for_pause_response and a pause response is sent from another thread
    Then the waiter returns that response without panicking

