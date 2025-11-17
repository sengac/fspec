@critical
@cli
@foundation-management
@bug-fix
@FOUND-021
Feature: discover-foundation overwrites draft without warning

  """
  Architecture notes:
  - Modify discover-foundation.ts discoverFoundation() function (lines 475-544)
  - Add file existence checks using fs.promises.access() before creating draft
  - Add --force flag to Commander.js command registration
  - Emit system-reminder wrapped error messages for AI visibility
  - Preserve existing --finalize behavior (no changes to finalization logic)
  - Check both foundation.json.draft AND foundation.json existence

  Critical implementation requirements:
  - MUST check file existence BEFORE writeFile() call on line 510
  - Error messages MUST use wrapInSystemReminder() utility
  - --force flag parsing MUST be added to action handler (line 570-632)
  - Exit with code 1 when draft/foundation exists without --force
  - Maintain backward compatibility with existing finalize workflow
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. discover-foundation without --finalize flag MUST check if foundation.json.draft exists before creating a new draft
  #   2. If draft exists, command MUST emit error message with --force flag suggestion
  #   3. The --force flag MUST allow overwriting existing draft (explicit opt-in)
  #   4. If foundation.json already exists, command MUST also warn and require --force flag
  #   5. Error message MUST be wrapped in system-reminder tags for AI visibility
  #
  # EXAMPLES:
  #   1. User runs 'fspec discover-foundation' when draft exists → Error: Draft already exists, use --force to overwrite
  #   2. User runs 'fspec discover-foundation --force' when draft exists → Draft is overwritten, warning shown
  #   3. User runs 'fspec discover-foundation' when foundation.json exists → Error: Foundation already exists, use --force to regenerate draft
  #   4. User runs 'fspec discover-foundation' when neither draft nor foundation.json exist → Draft created successfully
  #   5. User runs 'fspec discover-foundation --finalize' when draft exists → Normal finalization flow (no change to existing behavior)
  #
  # ========================================

  Background: User Story
    As a AI agent or developer using fspec
    I want to run discover-foundation command safely without losing draft progress
    So that I can recover from accidentally running the command twice without losing all my work

  Scenario: Prevent overwriting existing draft without --force flag
    Given a foundation.json.draft file exists
    When I run "fspec discover-foundation" without the --force flag
    Then the command should fail with exit code 1
    And the output should contain a system-reminder error message
    And the error message should suggest using the --force flag
    And the existing draft file should remain unchanged

  Scenario: Allow overwriting draft with --force flag
    Given a foundation.json.draft file exists
    When I run "fspec discover-foundation --force"
    Then the command should succeed
    And the draft file should be overwritten with a new template
    And the output should show a warning that the draft was overwritten

  Scenario: Prevent draft creation when foundation.json exists
    Given a foundation.json file already exists
    And no foundation.json.draft file exists
    When I run "fspec discover-foundation" without the --force flag
    Then the command should fail with exit code 1
    And the output should contain a system-reminder error message
    And the error message should explain that foundation already exists
    And the error message should suggest using --force to regenerate

  Scenario: Create draft when neither draft nor foundation exists
    Given no foundation.json.draft file exists
    And no foundation.json file exists
    When I run "fspec discover-foundation"
    Then the command should succeed
    And a foundation.json.draft file should be created
    And the output should show field-by-field guidance

  Scenario: Finalize flow unchanged when draft exists
    Given a foundation.json.draft file exists with complete fields
    When I run "fspec discover-foundation --finalize"
    Then the normal finalization flow should execute
    And the draft should be validated and converted to foundation.json
    And the draft file should be deleted after successful finalization
