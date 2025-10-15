@done
@workflow-automation
@hooks
@phase1
@HOOK-004
Feature: Hook discovery and event naming

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # USER STORY:
  #   As a fspec system
  #   I want to discover and execute hooks for lifecycle events
  #   So that hooks can be triggered at the right moments during command execution
  #
  # BUSINESS RULES:
  #   1. Event names follow pre-/post- convention: pre-<command-name>, post-<command-name>
  #   2. Special lifecycle hooks: pre-start (before any command), pre-exit (before returning output)
  #   3. Hook discovery searches config.hooks for matching event name
  #   4. If no hooks match event name, return empty array (no error)
  #   5. Event names are case-sensitive (pre-implementing != pre-Implementing)
  #   6. Command name extraction: 'fspec update-work-unit-status' -> 'update-work-unit-status'
  #
  # EXAMPLES:
  #   1. Discover hooks for 'post-implementing' event - returns array of matching hooks
  #   2. Discover hooks for 'pre-start' special lifecycle event - returns global pre-start hooks
  #   3. Discover hooks for non-existent event 'pre-xyz' - returns empty array (no error)
  #   4. Event name for command 'update-work-unit-status' generates 'pre-update-work-unit-status' and 'post-update-work-unit-status'
  #   5. Case-sensitive matching: 'post-implementing' finds hooks, 'post-Implementing' does not
  #   6. Multiple hooks for same event - returns all matching hooks in config order
  #
  # ========================================
  Background: User Story
    As a fspec system
    I want to discover and execute hooks for lifecycle events
    So that hooks can be triggered at the right moments during command execution

  Scenario: Discover hooks for specific event
    Given I have a hook configuration with hooks for "post-implementing"
    When I discover hooks for event "post-implementing"
    Then I should receive an array of matching hooks
    And the hooks should be returned in config order

  Scenario: Discover special lifecycle hooks
    Given I have a hook configuration with "pre-start" lifecycle hooks
    When I discover hooks for event "pre-start"
    Then I should receive the pre-start hooks
    And these hooks should execute before any command logic

  Scenario: Discover hooks for non-existent event
    Given I have a hook configuration
    When I discover hooks for event "pre-xyz"
    Then I should receive an empty array
    And no error should be thrown

  Scenario: Generate event names from command name
    Given the command name is "update-work-unit-status"
    When I generate event names for this command
    Then the pre-event name should be "pre-update-work-unit-status"
    And the post-event name should be "post-update-work-unit-status"

  Scenario: Event names are case-sensitive
    Given I have a hook configuration with hooks for "post-implementing"
    When I discover hooks for event "post-Implementing"
    Then I should receive an empty array
    And the hooks for "post-implementing" should not match

  Scenario: Multiple hooks for same event
    Given I have a hook configuration with multiple hooks for "post-testing"
    And the hooks are named "unit-tests", "integration-tests", "e2e-tests"
    When I discover hooks for event "post-testing"
    Then I should receive all three hooks
    And the hooks should be in the order they appear in config
