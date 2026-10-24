@done
@agent-core
@session-management
@persistence
@BUG-246
Feature: Retain authentic tool results when recovery admission is exhausted
  """
  Uses the single recovery owner from recoverable-conversation-persistence.feature.
  Three bounded retained ownership slots hold the blocked journal event and its
  assistant/result originals, with a combined 16 MiB bound. They do not claim
  durable admission and must report shutdown unsafe until explicit recovery.
  """

  Scenario: A captured tool result survives exhausted recovery capacity
    Given a dispatched sentinel tool and a session at recovery admission capacity
    When its authentic result settles and the output owner is dropped
    Then further dispatch is refused with a visible scoped persistence error
    And repairing storage and explicit recovery archives the authentic result once without repeating the tool
