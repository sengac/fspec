@BUG-226
@providers
@model-selection
@integration
Feature: Legacy model setters publish changes only after acceptance
  """
  Both direct NAPI setters use the live provider manager under its session
  lock. Provider selection validates before mutating the manager. Display
  labels, threshold overrides, and cached limits are published only after
  successful selection. Unknown provider identifiers genuinely fail the
  profile direct setter, which deliberately bypasses model registry lookup.
  """

  Background: User Story
    As a legacy API session user
    I want rejected switches to preserve all accepted model state
    So that labels and limits still describe the live provider

  Scenario: Rejected ordinary selection preserves model state
    Given a legacy session with accepted identity limits and threshold override
    When the ordinary setter rejects an unknown provider with replacement limits
    Then identity live selection cached limits and override are unchanged

  Scenario: Rejected profile selection preserves model state
    Given a legacy session with accepted identity limits and threshold override
    When the profile setter rejects an unknown provider with replacement limits
    Then identity live selection cached limits and override are unchanged

  Scenario: Successful legacy setters publish identity limits and override together
    Given a legacy session with accepted identity limits and threshold override
    When each legacy setter accepts a replacement model
    Then labels live selection cached limits and override describe the replacement
