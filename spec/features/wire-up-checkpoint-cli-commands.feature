@checkpoint
@done
@cli
@bug-fix
@command-registration
@GIT-005
Feature: Wire up checkpoint CLI commands
  """
  Architecture notes:
  - Uses Commander.js registration pattern matching other fspec commands
  - Each checkpoint command file must export registerXCommand(program) function
  - Registration happens in src/index.ts by importing and calling register functions
  - Follows existing patterns: checkpoint, list-checkpoints, restore-checkpoint, cleanup-checkpoints
  - No changes to underlying checkpoint logic, only CLI wiring
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All checkpoint command files must export a registerXCommand function that takes a Commander program instance
  #   2. All checkpoint commands must be imported and registered in src/index.ts
  #   3. Command registration must follow existing patterns (checkpoint, list-checkpoints, restore-checkpoint, cleanup-checkpoints)
  #
  # EXAMPLES:
  #   1. Running './dist/index.js checkpoint AUTH-001 baseline' creates a checkpoint named 'baseline' for work unit AUTH-001
  #   2. Running './dist/index.js list-checkpoints AUTH-001' shows all checkpoints for AUTH-001 with emoji indicators
  #   3. Running './dist/index.js restore-checkpoint AUTH-001 baseline' restores the baseline checkpoint for AUTH-001
  #   4. Running './dist/index.js cleanup-checkpoints AUTH-001 --keep-last 5' deletes old checkpoints, keeping only the 5 most recent
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to use checkpoint commands from the CLI
    So that I can create, list, restore, and cleanup checkpoints for safe experimentation

  Scenario: Register checkpoint command
    Given the checkpoint.ts file exports a registerCheckpointCommand function
    And the registerCheckpointCommand function is imported in src/index.ts
    And registerCheckpointCommand(program) is called in src/index.ts
    When I run './dist/index.js checkpoint AUTH-001 baseline'
    Then the command should create a checkpoint named 'baseline' for work unit AUTH-001
    And the command should exit with code 0

  Scenario: Register list-checkpoints command
    Given the list-checkpoints.ts file exports a registerListCheckpointsCommand function
    And the registerListCheckpointsCommand function is imported in src/index.ts
    And registerListCheckpointsCommand(program) is called in src/index.ts
    When I run './dist/index.js list-checkpoints AUTH-001'
    Then the command should list all checkpoints for AUTH-001 with emoji indicators
    And the command should exit with code 0

  Scenario: Register restore-checkpoint command
    Given the restore-checkpoint.ts file exports a registerRestoreCheckpointCommand function
    And the registerRestoreCheckpointCommand function is imported in src/index.ts
    And registerRestoreCheckpointCommand(program) is called in src/index.ts
    When I run './dist/index.js restore-checkpoint AUTH-001 baseline'
    Then the command should restore the baseline checkpoint for AUTH-001
    And the command should exit with code 0

  Scenario: Register cleanup-checkpoints command
    Given the cleanup-checkpoints.ts file exports a registerCleanupCheckpointsCommand function
    And the registerCleanupCheckpointsCommand function is imported in src/index.ts
    And registerCleanupCheckpointsCommand(program) is called in src/index.ts
    When I run './dist/index.js cleanup-checkpoints AUTH-001 --keep-last 5'
    Then the command should delete old checkpoints keeping only the 5 most recent
    And the command should exit with code 0
