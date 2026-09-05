@done
@PROV-145
@rust
@session
@high
Feature: Per-profile loop detection resolution
  """
  architecture:
  - resolve_profile_loop_detection (rust/sessions/src/model_resolution.rs,
  next to resolve_profile_max_images) reads the active profile's stored
  loopDetection* keys from fspec-config.json for a profile model selection
  (composite "openai:<profile>/<model>") and returns the flat stored values
  (enabled Option<bool>, window / maxRepeats / maxRetries Option<u32>).
  - It returns flat values, not the agent-loop's LoopDetectorConfig (a crate
  cycle forbids codelet-sessions from naming that type — the agent-loop
  layer assembles the effective values with the canonical defaults).
  - Non-profile selections (cloud / custom / codex) and profile lookups that
  find no stored profile resolve to all None => all RIG-014 defaults
  (today's behavior). The resolution follows the model on every call, so a
  mid-session model switch re-resolves on the next turn.
  """

  Background: User Story
    As a user configuring an OpenAI API profile
    I want the runtime loop detector to follow my profile's settings
    So that a loose profile aborts late while a strict profile aborts early, without code changes

  Scenario: The resolver resolves the stored loop-detection values
    Given a session against a profile storing loopDetectionEnabled false, loopDetectionWindow 320, loopDetectionMaxRepeats 5, loopDetectionMaxRetries 2
    When the per-turn loop-detection resolution runs
    Then it resolves to enabled false, window 320, maxRepeats 5, maxRetries 2

  Scenario: The resolver resolves absent keys to none
    Given a session against a profile with no loop-detection keys stored
    When the per-turn loop-detection resolution runs
    Then every value resolves to absent (the defaults apply downstream: enabled, 160, 10, 10)

  Scenario: A non-profile model resolves to all absent
    Given a session against a cloud model that has no profile behind it
    When the per-turn loop-detection resolution runs
    Then every value resolves to absent (the RIG-014 defaults apply uniformly)

  Scenario: A mid-session switch re-resolves to the new profile's values
    Given a session against a profile storing loopDetectionMaxRetries 3
    When the session switches mid-session to a profile storing loopDetectionMaxRetries 1
    Then the resolution follows the new profile (maxRetries 1, not the previous 3)
