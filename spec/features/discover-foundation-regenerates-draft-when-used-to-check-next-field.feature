@done
@discovery
@foundation
@FOUND-019
Feature: discover-foundation regenerates draft when used to check next field
  """
  Do NOT change discover-foundation behavior - regeneration is intentional for initial draft creation
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. discover-foundation without flags ALWAYS regenerates draft from scratch
  #   2. update-foundation commands automatically emit system-reminder for next field
  #   3. System-reminders must clarify the workflow: discover once → update many → finalize once
  #
  # EXAMPLES:
  #   1. AI runs 'fspec discover-foundation' after filling 3 fields → draft regenerated, all progress lost
  #   2. AI runs 'fspec update-foundation projectName fspec' → system-reminder says 'Field 2/8: project.vision' with correct command
  #   3. System-reminder after update-foundation should say 'Continue with: fspec update-foundation...' NOT 'Run: fspec discover-foundation'
  #
  # ========================================
  Background: User Story
    As a AI agent filling foundation draft
    I want to understand which command to use next
    So that I don't accidentally regenerate the draft and lose my progress

  Scenario: AI accidentally runs discover-foundation and loses progress
    Given a foundation draft exists with 3 fields already filled
    And the draft contains filled values for projectName, projectVision, and projectType
    When AI runs "fspec discover-foundation" without flags
    Then the draft is regenerated from scratch
    And all 3 previously filled fields are replaced with [QUESTION:] placeholders
    And all progress is lost

  Scenario: update-foundation emits next field reminder
    Given a foundation draft exists
    When AI runs "fspec update-foundation projectName 'fspec'"
    Then a system-reminder is emitted
    And the system-reminder contains "Field 2/8: project.vision"
    And the system-reminder contains the correct next command "fspec update-foundation projectVision"

  Scenario: System-reminder clarifies workflow to prevent confusion
    Given a foundation draft exists
    When AI runs any "fspec update-foundation" command successfully
    Then the system-reminder must NOT suggest running "fspec discover-foundation"
    And the system-reminder must explain the workflow: "discover once → update many → finalize once"
    And the system-reminder must show the next update-foundation or add-persona/add-capability command
