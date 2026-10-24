@session
@compaction-resume
@phase-1
@CMPCT-052
Feature: Production summary handoff preserves its captured durable source
  """
  The compaction-inference-handoff contract applies to the real background
  summary registration and deferred durable commit owner. Keep immutable
  originals, actual paired tool identity and exact admitted generation through
  cold replay. Rejection cannot authorize inference or publish completion.
  """

  Scenario: Production summary submission commits its captured durable source
    Given a live session with immutable originals and a selected physical serializer
    When its registered summary tool queues a complete DAG during streaming
    Then inference yields with the real tool call and result paired
    And the commit owner promotes and publishes exactly that captured generation
    And cold replay retains the original envelopes and admitted generation

  Scenario: A queued production candidate cannot be overwritten
    Given a live session with an accepted summary awaiting durable application
    When another summary submission tries to replace the candidate
    Then the second submission is rejected without changing the queued candidate
    And only the first candidate can become the committed generation

  Scenario: Production candidate rejection preserves memory and blocks inference
    Given a live session with immutable originals and a selected physical serializer
    When its registered summary tool submits an invalid DAG during streaming
    Then ordinary inference remains blocked without a committed generation
    And committed messages and counters remain unchanged without completion
