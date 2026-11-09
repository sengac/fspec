@PROV-151
@provider-abstraction
@providers
@streaming
@integration
Feature: Codex tool input streaming and safe dispatch
  Codex users can inspect tool input as it arrives without executing partial or unsafe calls.

  """
  Codex alone presents tool-input deltas through an opt-in delegating hook and sink.
  No tool dispatches at a completed output item; every tool, including read_file, list_dir and grep_files, waits for successful response completion. Arguments must be JSON objects.
  Both output item IDs and call IDs are deduplicated for the response lifetime.
  Existing tool permission hooks and execution limits remain authoritative.
  On late failure no tool runs and no result is returned; failed streams are not replayed.
  No exactly-once guarantee is made across process crashes or explicit user retries.
  """

  Scenario: Duplicate items and terminal reconciliation execute once
    Given two interleaved completed calls with repeated item and call identities
    When a successful terminal response repeats their output
    Then each call executes once with its own arguments and matching result

  Scenario: Mutation tools remain terminal gated
    Given a completed mutation item followed by a failed Codex response
    When the agent consumes the response
    Then no mutation executes and the failure is surfaced

  Scenario: Invalid and unfinished calls never execute
    Given malformed arguments or a non-completed tool item
    When the Codex stream terminates
    Then no invalid or unfinished tool executes

  Scenario: Permission cancellation still prevents execution
    Given a completed read-only call rejected by the existing hook
    When the agent dispatches the call
    Then cancellation prevents tool execution

  Scenario: Ordinary Responses does not opt into early dispatch
    Given an ordinary Responses completed tool item on an open stream
    When the agent waits for output
    Then the tool does not execute before the ordinary provider finishes

  Scenario: Completed read-only calls wait for response validation
    Given a completed read-only Codex item on an open stream
    When the agent consumes the stream before the response completes
    Then the tool does not run and no hook call or tool result is observed

  Scenario: Late failure prevents read-only execution without replay
    Given a completed read-only call followed by a failed response
    When the agent observes the failure
    Then the tool never runs, no result is returned and the request is not replayed
