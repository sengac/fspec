@done
@CONT-011
@cli
@completion
@integration
Feature: Completion goal teardown layout guards
  """
  Keep every original CONT-006 early-exit, shared teardown, single announcement
  and reminder-owner predicate. Session::clear_goal lives in reminders.rs,
  registered by session/mod.rs. Runtime native Done and goal tests remain
  necessary; a source guard alone is not provider completion proof.
  """

  Scenario: Goal teardown guards preserve a single reminder owner
    Given the source file rust/cli/src/interactive/stream_loop.rs
    Then the ToolResult arm consults the early-exit decision without any goal gate
    And the FinalResponse fallback routes through the same shared teardown helper as the early exit
    And the goal announcement formatting lives only in the goal acceptance helper
    And CompletionContract reminder removal lives only in the session goal clearing method
