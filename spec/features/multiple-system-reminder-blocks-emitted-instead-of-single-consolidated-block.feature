@REMIND-017
Feature: Multiple system-reminder blocks emitted instead of single consolidated block
  """
  Apply the VAL-004 strip-join-rewrap consolidation pattern to all commands that collect system reminders. Affected: show-work-unit, add-tag-to-feature, generate-scenarios, globalSessionStreamManager TUI path.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All commands must consolidate multiple reminders into a single <system-reminder> block using the strip-join-rewrap pattern from VAL-004
  #   2. Commands returning systemReminders: string[] must consolidate before returning, not delegate to CLI emission loop
  #   3. The TUI path in globalSessionStreamManager must join unwrapped reminders into one block, not re-wrap each individually
  #   4. No output from any fspec command should ever contain consecutive </system-reminder><system-reminder> patterns
  #
  # EXAMPLES:
  #   1. show-work-unit for a work unit with no estimate AND in done state for >24h emits ONE block containing both reminders separated by blank line
  #   2. add-tag-to-feature with 3 unregistered tags emits ONE block listing all three, not three separate blocks
  #   3. generate-scenarios with prefill AND post-generation reminders emits ONE block containing both
  #   4. TUI agent session receives single systemReminder string from fspec tool call, not multiple blocks joined with newlines
  #
  # ========================================
  Background: User Story
    As a AI agent
    I want to receive a single consolidated system-reminder block from fspec commands
    So that parse reminders reliably without handling multiple XML blocks

  Scenario: show-work-unit consolidates multiple reminders into single block
    Given a work unit with no estimate that has been in done state for over 24 hours
    When I run show-work-unit for that work unit
    Then the output contains exactly one system-reminder opening tag
    Then the block contains both the missing estimate and long duration reminders separated by a blank line

  Scenario: add-tag-to-feature consolidates unregistered tag reminders into single block
    Given a feature file and three unregistered tags
    When I run add-tag-to-feature with validate-registry enabled
    Then the output contains exactly one system-reminder opening tag
    Then the block contains all three unregistered tag warnings

  Scenario: generate-scenarios consolidates reminders into single block
    Given a work unit with example mapping data that will trigger both generation and prefill reminders
    When I run generate-scenarios for that work unit
    Then the output contains exactly one system-reminder opening tag
    Then the block contains both the generation guidance and prefill detection content

  Scenario: TUI path consolidates reminders from fspec tool call into single block
    Given a fspec tool call result containing multiple unwrapped system reminders
    When the globalSessionStreamManager processes the result
    Then the systemReminder string sent to the session contains exactly one system-reminder opening tag
    Then all reminder content is within that single block separated by blank lines
