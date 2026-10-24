@done
@BUG-216
@rust
@agent-loop
@session
@high
@streaming-loop-detection
Feature: Streaming loop detection requires complete words and fresh evidence
  """
  Architecture: StreamLoopDetector retains incomplete whitespace-delimited
  words across provider deltas, normalizes completed words case-insensitively,
  and bounds unfinished-word storage. Its latched diagnostic remains available
  until reset. Long suffix evidence counts non-overlapping earlier copies,
  excluding the trailing copy: the default three requires a fourth copy.
  BackgroundOutput consumes warning evidence by rearming that channel, without
  altering interruption protocols, persistence, or the bounded retry policy.
  This reconciles the older streaming-loop-detection narrative: two copies of
  a paragraph alone are not sufficient long-suffix evidence.
  """

  Background: User Story
    As an operator reading streamed technical reasoning
    I want warnings and aborts to require genuine repetition
    So that ordinary references survive while sustained loops still stop

  Scenario: Detection is invariant under arbitrary subword chunk boundaries
    Given equivalent technical prose or repeating text containing paths punctuation and Unicode
    When the text is streamed whole or split at arbitrary character boundaries
    Then the first loop signal is identical for every chunking

  Scenario: Two legitimate pathname references are not repetition collapse
    Given technical prose referencing the same long pathname twice with distinct surrounding words
    When every character is streamed as a separate delta
    Then no loop signal is reported

  Scenario: Long suffix detection honors the configured earlier copy threshold
    Given a long suffix detector requiring between one and five earlier copies
    When a distinct sixteen word passage is repeated up to the configured threshold
    Then no long suffix signal is reported before the trailing copy
    When the trailing copy completes
    Then a long suffix signal is reported

  Scenario: Overlapping matches do not inflate long suffix evidence
    Given a long suffix detector requiring three earlier sixteen word copies
    When only thirty two identical complete words are streamed
    Then no long suffix signal is reported
    When thirty two more identical complete words are streamed
    Then a long suffix signal is reported

  Scenario: Reset clears partial words and overlong words do not create evidence
    Given a detector receiving an unfinished word longer than its carry limit
    When the word ends and fresh distinct prose follows
    Then no loop signal is reported
    When the detector is reset with an unfinished word pending
    And a genuine complete word loop is streamed
    Then the loop is detected at the normal threshold
