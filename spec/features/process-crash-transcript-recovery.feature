@done
@agent-core
@session-management
@persistence
@BUG-249
Feature: Recover acknowledged transcript after isolated process termination
  """
  Durable acknowledgement precedes visible publication and permission to dispatch
  tools. The subprocess uses real BackgroundOutput and the shared recovery owner.
  Killing it cannot prove side-effect cancellation. Reopen is display-only and
  never invokes a tool or automatically resubmits a provider request.
  """

  Scenario: A killed process preserves its acknowledged prefix
    Given an isolated streaming process with a durable transcript store
    When a known text prefix is acknowledged and the process is killed before completion
    Then a fresh process recovers that exact prefix and its original identities

  Scenario: A killed tool process never repeats an unknown side effect
    Given a tool invocation durably recorded before dispatch
    When the tool writes a sentinel and the process is killed before result capture
    Then recovery shows outcome-unknown and never writes the sentinel again

  Scenario: A killed process excludes its aborted response attempt
    Given an isolated streaming process that aborts one response attempt and retries
    When the successful replacement prefix is acknowledged before process termination
    Then cold recovery exposes the replacement prefix once and excludes aborted text

  Scenario: A killed process preserves an invocation that never started
    Given an isolated streaming process with an acknowledged tool invocation
    When the process is terminated before dispatch admission
    Then cold recovery marks the invocation not-started without executing its side effect
