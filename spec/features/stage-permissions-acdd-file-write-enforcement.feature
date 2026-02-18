@workflow
@codelet
@BLOCK-003
Feature: Stage Permissions - ACDD File Write Enforcement

  """
  Stage Permissions Module: Create codelet/tools/src/stage_permissions/ with StagePermissionsConfig (load/save JSON), FileCategory struct (name + glob patterns), StagePermissions (map stage names to writable categories). Integrates with work unit context from session to determine current stage. Loaded from ~/.fspec/stage-permissions.json (user) or .fspec/stage-permissions.json (project).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Stage-aware file permissions: block writes to certain file categories based on current ACDD stage
  #   2. Project defines file categories: named groups of glob patterns (e.g., 'test': ['src/**/*.test.ts'], 'impl': ['src/**/*.ts', '!**/*.test.ts'])
  #   3. Project defines stage permissions: map each ACDD stage to which file categories are writable
  #   4. Current work unit context determines active stage; system checks file writes against stage permissions
  #   5. Allow all - no ACDD stage enforcement when no work unit is active
  #   6. No override - stage blocks are hard. Must move to correct ACDD stage to write those file types.
  #   7. Stage permissions: backlog=spec, specifying=spec, testing=spec+test, implementing=spec+test+impl, validating=nothing, done=nothing
  #
  # EXAMPLES:
  #   1. Stage block (testing): Work unit in 'testing' → AI tries Write 'src/auth.ts' → Blocked
  #   2. Stage allow (testing): Work unit in 'testing' → AI writes 'src/__tests__/auth.test.ts' → Allowed
  #   3. File category definition: categories: {'spec': ['spec/**/*.feature'], 'test': ['src/**/*.test.ts'], 'impl': ['src/**/*.ts', '!src/**/*.test.ts']}
  #
  # ========================================

  Background: Stage Permissions Configuration
    Given stage permissions are defined for the project

  # ====================
  # TESTING STAGE
  # ====================

  Scenario: Block implementation file write in testing stage
    Given the current work unit is in "testing" stage
    And the project defines "impl" category as "src/**/*.ts" excluding test files
    And "testing" stage only allows writing to "spec" and "test" categories
    When the AI tries to write to "src/auth.ts"
    Then the write should be blocked
    And the AI should receive error "Blocked: Cannot write implementation files in testing stage. Write tests first, then move to implementing stage."

  Scenario: Allow test file write in testing stage
    Given the current work unit is in "testing" stage
    And the project defines "test" category as "src/**/*.test.ts" and "src/**/__tests__/**"
    And "testing" stage allows writing to "spec" and "test" categories
    When the AI tries to write to "src/__tests__/auth.test.ts"
    Then the write should be allowed
    And the file should be written successfully

  # ====================
  # SPECIFYING STAGE
  # ====================

  Scenario: Block code file write in specifying stage
    Given the current work unit is in "specifying" stage
    And "specifying" stage only allows writing to "spec" category
    When the AI tries to write to "src/feature.ts"
    Then the write should be blocked
    And the AI should receive error "Blocked: Cannot write code files in specifying stage. Complete the specification first."

  # ====================
  # BACKLOG STAGE
  # ====================

  Scenario: Allow attachment in backlog stage
    Given the current work unit is in "backlog" stage
    And "backlog" stage allows writing to "spec" category
    When the AI tries to write to "spec/attachments/WORK-001/diagram.png"
    Then the write should be allowed

  # ====================
  # VALIDATING STAGE
  # ====================

  Scenario: Block all writes in validating stage
    Given the current work unit is in "validating" stage
    And "validating" stage allows writing to nothing
    When the AI tries to write to "src/auth.ts"
    Then the write should be blocked
    And the AI should receive error containing "go back to implementing"

  # ====================
  # DONE STAGE
  # ====================

  Scenario: Block all writes in done stage
    Given the current work unit is in "done" stage
    And "done" stage allows writing to nothing
    When the AI tries to write to "spec/features/test.feature"
    Then the write should be blocked
    And the AI should receive error containing "reopen"

  # ====================
  # NO WORK UNIT
  # ====================

  Scenario: Allow all writes when no work unit is active
    Given no work unit is set for the session
    When the AI tries to write to "src/auth.ts"
    Then the write should be allowed
    And no ACDD stage enforcement should occur
