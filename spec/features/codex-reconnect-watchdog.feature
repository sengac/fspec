@done
@phase-1
@provider-abstraction
@providers
@PROV-160
@critical
Feature: Codex reconnect watchdog
  """
  Codex-owned network waiting suspends only the generic generation-stall watchdog while remaining cancellable and protected by raw-network deadlines. Typed exhausted errors bypass the generic network retry layer even through error wrapping. Ordinary providers retain their existing deadlines and retry policy; local tool execution and hard task budgets are unchanged.
  """

  Background: User Story
    As a Codex session owner
    I want intentional provider network waiting to remain active
    So that the generic watchdog does not abort a recoverable turn

  Scenario: Provider offline waiting is not mistaken for a stalled generation
    Given a Codex session intentionally waiting for network reception
    When offline waiting outlasts the generic generation stall deadline
    Then the caller keeps polling cancellably until recovery while ordinary provider stall detection remains enabled
