@critical
@cli
@workflow-automation
@checkpoint
@git
@bug-fix
@BUG-043
Feature: Auto-checkpoints not working - lazy import fails in bundled dist
  """
  Architecture notes:
  - Root cause: Vite bundler incorrectly optimizes conditional checkpoint logic
  - Source code: 'const isDirty = await isWorkingDirectoryDirty(cwd); if (isDirty && currentStatus !== backlog)'
  - Bundled code: 'await Zn(t) && i !== backlog && (...)' - breaks execution flow
  - Solution: Use explicit boolean assignment to prevent optimizer from breaking logic
  - Implementation: 'const isDirty = await isWorkingDirectoryDirty(cwd); const shouldCreate = isDirty && currentStatus !== backlog; if (shouldCreate) { ... }'
  - Critical: Tests must validate BUNDLED distribution (dist/index.js), not just source code
  - Uses isomorphic-git for checkpoint creation via git stash
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Auto-checkpoints must be created before EVERY state transition (except from backlog)
  #   2. Checkpoint creation logic must survive Vite bundler optimization without breaking execution flow
  #   3. No dynamic imports (await import()) allowed in checkpoint-related code - ONLY static imports at top of file
  #
  # EXAMPLES:
  #   1. User has uncommitted changes, runs 'fspec update-work-unit-status TEST-001 implementing' (from testing state), sees '🤖 Auto-checkpoint: TEST-001-auto-testing created before transition' message
  #   2. User runs 'fspec list-checkpoints TEST-001' after transition, sees the auto-checkpoint in the list
  #   3. User has clean working directory, runs status transition, NO checkpoint created (expected behavior)
  #   4. User transitions FROM backlog state with uncommitted changes, NO checkpoint created (rule: except from backlog)
  #   5. Integration test runs 'npm run build' then executes './dist/index.js update-work-unit-status' and verifies checkpoint is actually created
  #
  # ASSUMPTIONS:
  #   1. Use Option 2: explicit boolean assignment - clearest intent, prevents optimizer issues, maintains readability
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec for ACDD workflow
    I want to have automatic checkpoints created reliably during work unit transitions
    So that I can safely experiment and recover from mistakes without manual checkpoint creation

  Scenario: Auto-checkpoint created on state transition with uncommitted changes
    Given a work unit in 'testing' status with uncommitted file changes in the working directory
    When the user runs 'fspec update-work-unit-status <id> implementing'
    Then the command should display '🤖 Auto-checkpoint: "<id>-auto-testing" created before transition'
    And the checkpoint should be retrievable with 'fspec list-checkpoints <id>'
    And the checkpoint should contain the uncommitted changes

  Scenario: No auto-checkpoint created when transitioning FROM backlog
    Given a work unit in 'backlog' status with uncommitted file changes in the working directory
    When the user runs 'fspec update-work-unit-status <id> specifying'
    Then the command should NOT display any checkpoint creation message
    And no checkpoints should exist for the work unit

  Scenario: No auto-checkpoint created with clean working directory
    Given a work unit in 'testing' status with a clean working directory
    When the user runs 'fspec update-work-unit-status <id> implementing'
    Then the command should NOT display any checkpoint creation message
    And no checkpoints should exist for the work unit

  Scenario: Static imports enforced in update-work-unit-status.ts
    Given the source file 'src/commands/update-work-unit-status.ts'
    When a static analysis test scans the file for dynamic imports
    Then the test should NOT find any 'await import(' patterns
    And the test should verify git-checkpoint is imported statically at top of file
