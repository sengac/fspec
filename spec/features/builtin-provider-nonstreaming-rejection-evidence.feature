@CMPCT-071
@providers
@context-management
@compaction
@phase-1
Feature: Builtin provider nonstreaming rejection evidence
  """
  Physical builtin rejection retains its typed HTTP status and original body.
  Recovery can inspect machine-readable codes without interpreting display text.
  A rejected request cannot subsequently report successful completion or usage.
  """

  Scenario: Nonstreaming OpenAI Chat preserves physical HTTP rejection evidence
    Given a selected nonstreaming OpenAI Chat model whose physical endpoint rejects requests
    When requests receive structured overflow, authentication, and plain text rejection bodies
    Then each stream exposes the original typed HTTP status and exact body
    And no rejected stream emits a successful final response
