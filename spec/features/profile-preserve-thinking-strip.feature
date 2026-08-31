@done
@agent-core
@provider-settings
@PROV-143
Feature: Profile preserve-thinking history strip (PROV-143)
  """
  architecture:
  - strip_reasoning_from_history (rust/core/src/history_strip.rs) is the pure
  strip helper: a copy of the history with every AssistantContent::Reasoning
  block removed when preserve-thinking is disabled; reasoning-only assistant
  messages are dropped entirely (empty assistant messages are invalid on the
  OpenAI-compat wire format).
  - The live session history is never mutated.
  - RigAgent::outgoing_history is the single outgoing-history choke point that
  applies the flag before each LLM call.
  """

  Background: User Story
    As a provider profile user with Preserve Thinking disabled
    I want old thinking blocks to be removed from the conversation sent back to the LLM
    So that the model is not confused by stale reasoning while my saved history keeps it

  Scenario: History handed to the LLM is stripped of thinking when disabled
    Given a history containing an assistant message with Reasoning and Text content
    When preserve-thinking is disabled for the session
    Then the history clone passed to the LLM keeps the Text content
    And the clone contains no Reasoning content in that message
    And the original session history is not mutated

  Scenario: A reasoning-only assistant message is dropped from the outgoing history
    Given a history containing an assistant message with Reasoning and Text content
    When preserve-thinking is disabled for the session
    Then the clone passed to the LLM contains no empty assistant message

  Scenario: Preserve-thinking enabled returns the history unchanged
    Given a history containing an assistant message with Reasoning and Text content
    When preserve-thinking is enabled for the session
    Then the clone passed to the LLM still contains the Reasoning content

  Scenario: Stripping keeps the message count and user messages intact
    Given a history containing an assistant message with Reasoning and Text content
    When preserve-thinking is disabled for the session
    Then the clone passed to the LLM keeps the same number of messages
