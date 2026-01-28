@feature-management
@tool-integration
@integration
@CODE-006
Feature: System Reminder Preservation

  """
  Include captured system reminders in FspecTool response JSON for LLM workflow orchestration
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. System reminders must be captured from fspec TypeScript command execution
  #   2. Captured system reminders must be included in FspecTool response for LLM workflow guidance
  #   3. System reminder format must follow '<system-reminder>content</system-reminder>' convention
  #   4. We should capture console.error() output during command execution since both patterns output to stderr - this is the comprehensive approach
  #   5. Capture both patterns to maintain compatibility with existing commands - parse result.systemReminder AND raw <system-reminder> tags
  #
  # EXAMPLES:
  #   1. When creating work unit, capture reminder: 'Work unit EXAMPLE-001 has no estimate' from console.error output
  #   2. Agent receives system reminder in FspecTool response: 'Run: fspec update-work-unit-estimate CODE-006 <points>'
  #   3. Multiple system reminders are captured and combined in tool response: estimate reminder + example mapping reminder
  #   4. Agent receives system reminders: 'Run: fspec update-work-unit-estimate CODE-006 <points>' for workflow guidance
  #
  # QUESTIONS (ANSWERED):
  #   Q: How should we capture system reminders from real fspec TypeScript command execution? Should we intercept console.error() output during command execution, or do the fspec commands return system reminders in their response objects?
  #   A: We should capture console.error() output during command execution since both patterns output to stderr - this is the comprehensive approach
  #
  #   Q: Looking at the existing fspec commands, I can see system reminders are output via console.error() in two patterns: (1) Formatted in result.systemReminder property, and (2) Raw <system-reminder> tags directly to console.error. Should we capture both patterns or standardize on one approach?
  #   A: Capture both patterns to maintain compatibility with existing commands - parse result.systemReminder AND raw <system-reminder> tags
  #
  # ========================================

  Background: User Story
    As a AI agent using FspecTool
    I want to capture system reminders from fspec command execution
    So that I receive workflow guidance and next-step instructions for effective project management

  Scenario: Capture system reminder from console.error during command execution
    Given a fspec command outputs system reminders to console.error during execution
    When the TypeScript callback executes the command within fspecCallback
    Then the console.error output should be captured
    And the system reminder should be parsed from the captured stderr
    And the system reminder should be included in the FspecTool response

  Scenario: Parse result.systemReminder property from command response  
    Given a fspec command returns a result with systemReminder property
    When the TypeScript callback processes the command result
    Then the result.systemReminder content should be extracted
    And the system reminder should be included in the FspecTool response for LLM guidance

  Scenario: Parse raw <system-reminder> tags from console.error output
    Given a fspec command outputs raw <system-reminder> tags to console.error
    When the TypeScript callback captures the stderr output during execution
    Then the <system-reminder> tags should be parsed and extracted
    And the system reminder content should be included in the FspecTool response

  Scenario: Combine multiple system reminders in tool response
    Given a fspec command outputs both result.systemReminder and raw <system-reminder> tags
    When the TypeScript callback processes the command execution
    Then both system reminder patterns should be captured
    And the system reminders should be combined in the FspecTool response
    And the LLM should receive all workflow guidance in a single response
