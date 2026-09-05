@done
@PROV-145
@rust
@agent-loop
@session
@high
Feature: Per-profile loop detection detector construction
  """
  architecture:
  - The per-turn streaming loop detector honors the active profile's stored
  loop-detection values. At the start of every turn,
  BackgroundOutput::with_provider (rust/agent-loop/src/background_output.rs)
  receives a per-turn LoopDetectionWiring (enabled flag + window +
  maxRepeats + maxRetries) resolved from the session's current
  provider/model selection. The wiring's Default (absent values) keeps
  today's behavior: detector ON with the POC-validated RIG-014 defaults
  (window 160, repeat threshold 10, retry cap 10).
  - with_provider builds the StreamLoopDetector from the wiring's
  detector_config() (window + max_repeats substituted, all other thresholds
  keep their POC-validated values); a wiring with enabled=false makes
  feed_loop_detectors a no-op so no corrective note is ever staged.
  - The loop-abort auto-continue retry cap replaces the hard-coded const
  RIG014_MAX_LOOP_ABORT_RETRIES (10) with the wiring's max_retries (read
  per turn from the profile resolution); the counter keeps its
  per-user-turn reset. Because the detector is rebuilt from the current
  selection every turn, a mid-session model switch takes effect on the
  next turn. The NAPI twin (rust/napi) keeps its detector-less
  BackgroundOutput (documented drift, follow-up card).
  """

  Background: User Story
    As a user configuring an OpenAI API profile
    I want the runtime loop detector to follow my profile's settings
    So that a loose profile aborts late while a strict profile aborts early, without code changes

  # ========================================
  # PER-TURN DETECTOR CONSTRUCTION (agent-loop layer)
  # ========================================
  Scenario: A stored loopDetectionEnabled false disables the detector for the session's turns
    Given a per-turn loop-detection wiring built from the stored value loopDetectionEnabled false
    When the session's streaming loop detector path feeds 100 words of a degenerating repeating loop
    Then no loop-detector abort fires for that turn
    And the stream is never cancelled by the loop detector and no corrective note is staged

  Scenario: A wiring built from absent values keeps the detector enabled
    Given a per-turn loop-detection wiring built from all absent values
    When the session's streaming loop detector path feeds a 10-word normal prefix followed by "alpha beta gamma" repeated 10 times
    Then the loop detector fires and a corrective note is staged on the session (today's behavior preserved)

  Scenario: A stored window and repeat threshold loosen the detector
    Given a per-turn loop-detection wiring built from the stored values loopDetectionWindow 30 and loopDetectionMaxRepeats 12
    When the session's streaming loop detector path feeds a 10-word normal prefix followed by "alpha beta gamma" repeated 10 times
    Then no loop signal fires (the 30-word window holds fewer repeats than the stored threshold 12, and the small window disables the long-suffix, periodicity, and diversity signals)
    And the same stream fed through the default wiring DOES fire the n-gram repeat signal

  Scenario: A stored lower repeat threshold aborts earlier
    Given a per-turn loop-detection wiring built from the stored value loopDetectionMaxRepeats 5
    When the session's streaming loop detector path feeds a 10-word normal prefix followed by "alpha beta gamma" repeated 10 times
    Then the loop detector fires after the 5th repetition of the tail
    And the default wiring feeds 15 more words before it fires (repeat threshold 10)

  # ========================================
  # RETRY CAP (agent loop)
  # ========================================
  Scenario: The per-turn retry cap resolves from the stored loopDetectionMaxRetries
    Given a per-turn loop-detection wiring built from the stored value loopDetectionMaxRetries 2
    When the agent loop reads the loop-abort auto-continue retry cap for the turn
    Then the cap is 2
    And a wiring built from an absent value reads the cap 10 (the RIG-014 default)

  Scenario: The agent loop reads the per-turn retry cap from the profile resolution
    Given the agent loop constructs the per-turn BackgroundOutput from the session's provider/model
    When the turn's loop-detection wiring is resolved
    Then the loop-abort retry cap comes from that resolution (the hard-coded const RIG014_MAX_LOOP_ABORT_RETRIES is gone)
    And the retry counter still resets on genuine user input (per-user-turn semantics preserved)
