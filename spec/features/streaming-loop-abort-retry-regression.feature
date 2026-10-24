@done
@BUG-216
@rust
@agent-loop
@session
@high
@streaming-loop-detection
@integration
Feature: Sustained repetition still cancels and recovers within bounded retries
  """
  Architecture: The real agent loop runs against the existing stub provider,
  exercising BackgroundOutput detection, the existing interrupt path, persisted
  marker, corrective context, and profile-resolved bounded auto-continue policy.
  Both warnings and aborts must be backed by independent word evidence.
  """

  Scenario: Genuine sustained loops still cancel persist recover and stop retrying
    Given a stub provider streaming a normal prefix followed by sustained complete word repetition
    When the production agent loop processes the input and exhausts its retry cap
    Then every attempt is cancelled before consuming the full provider stream
    And persisted assistant output retains the prefix and loop abort marker
    And retry context contains a corrective note and retries are bounded
