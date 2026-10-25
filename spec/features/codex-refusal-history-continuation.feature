@done
@BUG-260
@providers
@provider-abstraction
@integration
Feature: Codex Refusal History Continuation
  """
  Only the readable lineage comparison normalizes refusal sections to output text.
  The trusted checkpoint and physical follow-up request retain native refusal,
  phased messages, tool identities and opaque reasoning. Strict metadata and
  ancestry comparisons remain intact. This does not change protocol replay policy.
  """

  Background: User Story
    As a Codex session user
    I want to send another message or continue tools after a refusal answer
    So that native refusal and encrypted history survive without false ancestry errors

  Scenario: User messages and nudges retain native refusal history
    Given refusal only or mixed assistant output with optional readable reasoning
    When a user message or continuation nudge follows the displayed answer
    Then the provider receives the original refusal and opaque history unchanged

  Scenario: Tool results retain refusal and call identity
    Given mixed refusal output followed by a completed tool call
    When the genuine tool result continues the displayed answer
    Then the physical follow up preserves refusal reasoning and the original call exactly once

  Scenario: Altered readable refusal ancestry remains rejected
    Given a committed native refusal answer
    When a follow up changes visible text or omits unknown content metadata
    Then ancestry validation rejects the request before inference
