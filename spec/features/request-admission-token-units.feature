@providers
@compaction
@context-management
@phase-1
Feature: Request admission uses token units instead of byte counts
  """
  Complete request admission shares the application's tokenizer for text,
  including instructions, conversation, schemas and framing. Inline image
  payloads are excluded from text tokenization and retain their explicit media
  reservation. Counting never rewrites the selected physical request.
  """

  Background: User Story
    As a user with a configured context window
    I want request admission to compare tokens with token limits
    So that safety checks do not compact a mostly empty context window

  Scenario: Large text requests below the token budget remain admissible
    Given a text request whose byte length exceeds the context capacity
    And its token count plus reserved output fits that capacity
    When the complete selected request is admitted
    Then admission succeeds with the tokenizer estimate and unchanged request bytes

  Scenario: Text requests exceeding the token budget remain blocked
    Given a text request whose token count exceeds the available input budget
    When the complete selected request is admitted
    Then admission returns the actual estimated token count in a budget failure

  Scenario: Inline image payloads do not inflate text token estimates
    Given a request containing inline image data and a tool schema
    When the complete selected request is admitted
    Then occupancy equals tokenized framing and schema plus the image reservation
    And increasing only the encoded image payload does not change occupancy

  Scenario: Invalid text encoding cannot authorize admission
    Given serialized request bytes that are not valid UTF-8
    When the serialized request is admitted with known media occupancy
    Then admission returns a typed preparation failure

  Scenario: Physical provider sends do not reject low token usage
    Given a Codex conversation below one quarter of its configured token window
    And the serialized conversation is larger than that window in bytes
    When streaming and nonstreaming completions send the conversation
    Then each request reaches the provider without a local overflow failure
