@done
@CONT-011
@agent-core
@completion
@integration
Feature: Completion persisted history composition
  """
  Reuse the real isolated Copilot builder and shared agent-loop output owner.
  Reset cached stores only after accepted completion, then inspect real disk
  message envelopes. This complements provider wire/memory history and persisted
  billing checks; it does not substitute a fake file tool or a new persistence path.
  """

  Background: User Story
    As a Fspec maintainer
    I want accepted native completion retained in the conversation on disk
    So that subsequent restoration retains paired tool results without transport secrets

  Scenario: Accepted native completion persists a paired transcript
    Given an isolated native provider session has continuation armed
    When a serialized response accepts native Done through the shared output owner
    And cached persistence stores are reset before reloading the conversation
    Then disk message envelopes retain exactly one matching Done invocation and result
    And accepted completion is retained once without owned opaque transport state
