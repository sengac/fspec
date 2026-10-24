@done
@CMPCT-073
@compaction
@session
@configuration
@phase-1
Feature: Saved threshold override propagation
  """
  Resolve profile configuration through the selected ProviderManager before
  caching limits. Creation and model switching share the runtime configuration
  used by the hook and stream. Trigger thresholds do not change hard capacity.
  Invalid explicit configuration must not be interpreted as absent configuration.
  """

  Background: User Story
    As an agent using saved model profiles
    I want explicit compaction thresholds propagated to every limits consumer
    So that creating or switching sessions cannot silently lose my configuration

  Scenario: Session creation carries saved token threshold to cached limits
    Given a saved model profile with an explicit token threshold
    When a session is created using that profile
    Then its cached threshold matches the active policy without changing capacity

  Scenario: Model switching refreshes saved percentage threshold
    Given a session using a saved token threshold
    When it switches to a profile with a saved percentage threshold
    Then cached limits and active policy use the new threshold not the old one

  Scenario: Invalid saved explicit threshold fails before session creation
    Given a saved model profile with an invalid explicit threshold
    When a session is created using that profile
    Then creation fails clearly instead of caching a guessed threshold

  Scenario: Unresolved saved profile fails before session creation
    Given an explicit model selection naming an unavailable saved profile
    When a session is created using that profile
    Then creation fails clearly instead of treating configuration as absent
