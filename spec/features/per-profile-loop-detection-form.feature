@done
@PROV-145
@rust
@tui
@provider-settings
@high
Feature: Per-profile Loop Detection form fields
  """
  The /provider OpenAI profile create/edit form (rust/fspec-tui/src/views/
  provider_settings/) gains 4 new entries appended after Max Images
  (10th-13th): 'Loop Detection' (boolean toggle, Space/Left/Right flip),
  'Loop Window' (numeric, detector sliding window in words), 'Loop Repeat'
  (numeric, tail n-gram repeat threshold), and 'Loop Retries' (numeric, max
  auto-continue retries after a loop abort). Non-numeric input in any
  numeric field rejects the save with a hint and keeps the form open
  (mirrors PROV-142/144). The stored value (or the effective default when
  the key is absent) prefills each field; saving an empty numeric field
  clears the stored key (absent => RIG-014 default). The on-disk read
  (profiles_config.rs profile_definition_from_value) reads the four camelCase
  keys mirroring the maxImages pattern.
  """

  Background: User Story
    As a user configuring an OpenAI API profile
    I want to tune the RIG-014 streaming loop detector in the /provider profile form
    So that my loop-detection sensitivity and auto-retry budget follow the active profile

  # ========================================
  # TUI form: the new Loop Detection fields
  # ========================================
  Scenario: The four Loop Detection fields appear after Max Images with the correct defaults
    Given an OpenAI profile "work-vllm" exists with no loopDetection keys stored
    When I open the profile edit form in the /provider view
    Then the "Loop Detection" field appears after "Max Images"
    And the "Loop Window", "Loop Repeat", and "Loop Retries" fields appear after "Loop Detection"
    And the "Loop Detection" toggle prefills to Enabled (absent key preserves today's always-on behavior)
    And the "Loop Window" field is prefilled with 160
    And the "Loop Repeat" field is prefilled with 10
    And the "Loop Retries" field is prefilled with 10

  Scenario: Storing loop detection values prefills the fields on re-open
    Given an OpenAI profile "work-vllm" stores loopDetectionEnabled false, loopDetectionWindow 320, loopDetectionMaxRepeats 5, loopDetectionMaxRetries 2
    When I open the profile edit form in the /provider view
    Then the "Loop Detection" toggle prefills to Disabled
    And the "Loop Window" field is prefilled with 320
    And the "Loop Repeat" field is prefilled with 5
    And the "Loop Retries" field is prefilled with 2

  Scenario: The Loop Detection toggle flips with Space and never accepts text
    Given a profile form is open with the "Loop Detection" field focused
    When I press Space
    Then the toggle flips to Disabled
    When I press Space again
    Then the toggle flips back to Enabled
    And printable characters are never appended to the toggle field

  Scenario: Loop Window value saves and round-trips through the form
    Given a profile form is open with the "Loop Window" field focused
    When I clear the field and type 320 and press save
    Then the built profile definition carries loopDetectionWindow 320
    And re-opening the form for the saved profile shows the "Loop Window" field prefilled with 320

  Scenario: Empty numeric loop-detection field saves as absent
    Given an OpenAI profile "work-vllm" stores loopDetectionWindow 320
    When I open the profile edit form and clear the "Loop Window" field
    And I press save
    Then the built profile definition has no loopDetectionWindow value
    And re-opening the form shows the "Loop Window" field prefilled with the default 160

  Scenario: Non-numeric Loop Window input rejects the save
    Given a profile form is open with the "Loop Window" field focused
    When I type "abc" into the "Loop Window" field and press save
    Then the save is rejected with a hint naming the Loop Window field
    And nothing is persisted and the form stays open showing "abc"

  Scenario: Non-numeric Loop Repeat input rejects the save
    Given a profile form is open with the "Loop Repeat" field focused
    When I type "x" into the "Loop Repeat" field and press save
    Then the save is rejected with a hint naming the Loop Repeat field
    And nothing is persisted and the form stays open showing "x"

  Scenario: Non-numeric Loop Retries input rejects the save
    Given a profile form is open with the "Loop Retries" field focused
    When I type "1.5" into the "Loop Retries" field and press save
    Then the save is rejected with a hint naming the Loop Retries field
    And nothing is persisted and the form stays open showing "1.5"
