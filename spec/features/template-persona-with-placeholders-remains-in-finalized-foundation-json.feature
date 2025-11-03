@cli
@discovery-command
@validation
@foundation
@critical
@DISC-002
Feature: Template persona with placeholders remains in finalized foundation.json

  """
  Root cause in discover-foundation.ts:287-383. The finalize process computes allFieldsComplete (line 297) but never checks it before writing foundation.json (line 352). Fix: Add check after line 297 to reject finalization if allFieldsComplete is false.
  scanDraftForNextField() at lines 31-87 correctly detects placeholders by checking if stringified field values contain '[QUESTION:' or '[DETECTED:'. For personas array, it stringifies the entire array, so template personas with placeholders are correctly flagged.
  Schema validation (validateGenericFoundationObject) only checks structure, not content. A persona with name='[QUESTION: Who?]' passes validation because it's a valid string with minLength>1. This is why we need the allFieldsComplete check.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The discover-foundation --finalize command MUST check if allFieldsComplete is false before writing foundation.json
  #   2. If allFieldsComplete is false (placeholder fields remain), finalization MUST fail with a clear error message listing which fields still have placeholders
  #   3. Template personas contain [QUESTION:] markers in name, description, or goals fields
  #
  # EXAMPLES:
  #   1. User runs discover-foundation (creates draft with template persona containing [QUESTION:]), then runs add-persona 'AI Agent' 'description' --goal 'goal', then runs add-capability 'Capability' 'desc', then runs discover-foundation --finalize. Currently: finalization succeeds with template persona still in file. Expected: finalization should fail with error about unfilled personas field.
  #   2. Draft contains personas: [{name: '[QUESTION: Who?]', description: '[QUESTION: Who?]', goals: ['[QUESTION: What?]']}, {name: 'Real Person', description: 'A real user', goals: ['Achieve things']}]. scanDraftForNextField detects placeholder in personas array, sets allFieldsComplete=false. Finalize should check this flag and reject with error.
  #
  # ========================================

  Background: User Story
    As a developer using fspec for project foundation setup
    I want to finalize the foundation draft with discover-foundation --finalize
    So that I receive clear error messages when placeholder fields remain unfilled

  Scenario: Reject finalization when template persona with placeholders remains in draft
    Given I have a foundation.json.draft with all basic fields filled
    And the draft has a template persona with "[QUESTION: Who uses this?]" placeholders
    And I have added real personas using add-persona command
    And I have added capabilities using add-capability command
    When I run "fspec discover-foundation --finalize"
    Then the command should fail with exit code 1
    And the output should contain "Cannot finalize: draft still has unfilled placeholder fields"
    And the output should indicate which field contains placeholders
    And the foundation.json file should NOT be created
    And the foundation.json.draft file should remain unchanged

  Scenario: Detect placeholders in personas array during finalization
    Given I have a foundation.json.draft with mixed personas
    And the personas array contains a template persona with "[QUESTION:]" markers
    And the personas array contains a valid persona without placeholders
    When the scanDraftForNextField function processes the draft
    Then allFieldsComplete should be set to false
    And nextField should indicate "personas"
    When I attempt to finalize the draft
    Then finalization should fail with a clear error message
    And the error should list the specific fields with placeholders