@high
@cli
@code-quality
@review
@ai-driven
@workflow
@REV-005
Feature: Conversational Review Prompt Before Done
  """
  Implementation location: src/commands/update-work-unit-status.ts. Add system-reminder emission when transitioning to 'done' status. Check workUnit.type: if story or bug, emit formatted system-reminder using formatAgentOutput(). Template: 'QUALITY CHECK OPPORTUNITY\n\nBefore marking {id} as done, consider running a quality review.\n\nSuggested action:\n1. Ask user: Would you like me to run fspec review {id} to check for bugs, anti-patterns, and FOUNDATION.md alignment?\n2. If yes: Run fspec review {id}, address findings, then mark done\n3. If no: Proceed to mark done\n\nCommand: fspec review {id}'. Use existing getAgentConfig() for agent detection.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. System-reminder must appear when moving work unit to 'done' status (pre-done trigger)
  #   2. System-reminder must prompt AI to ask user about running fspec review
  #   3. Prompt must be conversational and non-blocking (AI asks user, not hard requirement)
  #   4. System-reminder must suggest exact command: 'fspec review <work-unit-id>'
  #   5. Only trigger for story and bug work units (tasks don't need deep review)
  #   6. System-reminder must use agent-aware formatting (getAgentConfig, formatAgentOutput)
  #
  # EXAMPLES:
  #   1. AI runs 'fspec update-work-unit-status AUTH-001 done' and receives system-reminder: 'Before marking complete, consider asking: Would you like me to run fspec review AUTH-001 to check for quality issues?'
  #   2. User says yes, AI runs 'fspec review AUTH-001', finds 2 issues, fixes them, then marks done
  #   3. User says no, AI proceeds to mark work unit as done without review
  #   4. AI moves task work unit TASK-001 to done, NO review prompt appears (tasks exempt)
  #
  # ========================================
  Background: User Story
    As a AI agent completing work units
    I want to proactively suggest running quality review before marking work as done
    So that users catch quality issues early and maintain high code standards

  Scenario: Story work unit transitioning to done shows review prompt
    Given I have a story work unit "AUTH-001" in "validating" status
    When I run "fspec update-work-unit-status AUTH-001 done"
    Then the command should emit a system-reminder
    And the system-reminder should suggest asking user "Would you like me to run fspec review AUTH-001 to check for quality issues?"
    And the system-reminder should include the exact command "fspec review AUTH-001"
    And the system-reminder should be agent-aware formatted
    And the work unit status should be updated to "done"

  Scenario: Bug work unit transitioning to done shows review prompt
    Given I have a bug work unit "BUG-001" in "validating" status
    When I run "fspec update-work-unit-status BUG-001 done"
    Then the command should emit a system-reminder
    And the system-reminder should suggest asking user about running fspec review
    And the system-reminder should include suggested workflow steps
    And the work unit status should be updated to "done"

  Scenario: Task work unit transitioning to done does NOT show review prompt
    Given I have a task work unit "TASK-001" in "validating" status
    When I run "fspec update-work-unit-status TASK-001 done"
    Then the command should NOT emit a review prompt system-reminder
    And the work unit status should be updated to "done"
    And the output should be the standard success message only

  Scenario: AI workflow - user accepts review suggestion
    Given I have a story work unit "API-001" in "validating" status
    And the AI is moving the work unit to done
    When the system-reminder prompts the AI to ask about review
    And the user responds "yes, please run the review"
    Then the AI should run "fspec review API-001"
    And the AI should address any findings from the review
    And the AI should mark the work unit as done after fixes

  Scenario: AI workflow - user declines review suggestion
    Given I have a story work unit "UI-001" in "validating" status
    And the AI is moving the work unit to done
    When the system-reminder prompts the AI to ask about review
    And the user responds "no, just mark it done"
    Then the AI should proceed to mark the work unit as done
    And the AI should skip running fspec review
