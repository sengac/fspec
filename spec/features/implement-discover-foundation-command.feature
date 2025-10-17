@orchestration
@discovery
@cli
@phase1
@FOUND-004
Feature: Implement discover-foundation Command

  """
  System-reminder integration: Wrap questionnaire output in <system-reminder> tags when running in AI context (detectable via environment)
  System-reminder triggers: (1) After code analysis completes - show detected personas/capabilities (2) During questionnaire - provide examples of good answers (3) After foundation.json generated - next steps guidance
  System-reminder content: 'Code analysis detected 3 personas [list]. Review and confirm during questionnaire. Focus on WHY/WHAT, not HOW. See CLAUDE.md for boundary guidance.'
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Must emit system-reminders to guide AI through discovery process (persona review, WHY/WHAT boundary, next steps)
  #
  # EXAMPLES:
  #   1. After code analysis: <system-reminder>Detected 3 user personas from routes: End User, Admin, API Consumer. Review in questionnaire. Use 'fspec show-work-unit FOUND-002' for details.</system-reminder>
  #
  # ========================================

  Background: User Story
    As a developer using fspec with AI to bootstrap foundation documents
    I want to run discover-foundation command to orchestrate analysis and questionnaire
    So that I get a complete validated foundation.json without manual work

  Scenario: Emit system-reminder after code analysis detects personas
    Given I run discover-foundation command
    And code analysis detects 3 personas: End User, Admin, API Consumer
    When code analysis completes
    Then command should emit system-reminder with detected personas
    And system-reminder should list all 3 detected personas
    And system-reminder should guide AI to review in questionnaire


  Scenario: Generate validated foundation.json after questionnaire
    Given I complete the questionnaire with all required answers
    When discover-foundation finishes
    Then foundation.json should be created
    And foundation.json should pass schema validation
    And foundation.json should contain questionnaire answers

