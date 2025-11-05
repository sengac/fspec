@done
@critical
@file-ops
@migration
@infrastructure
@version-management
@MIG-001
Feature: Build Migration System
  """
  File structure: src/migrations/ with index.ts (runner), registry.ts (version tracking), types.ts (interfaces), migrations/ subfolder for individual migration files, __tests__/ for tests
  Migration interface: {version: string, name: string, description: string, up: (data) => data, down?: (data) => data}. Each migration has unique version number.
  Registry pattern: migrations array in registry.ts exports all migrations in version order. getMigrationsToApply(currentVersion, targetVersion) returns migrations to run.
  Auto-migration integration: ensureWorkUnitsFile() in src/utils/ensure-files.ts calls ensureLatestVersion(cwd, CURRENT_VERSION) before returning data
  Backup naming: work-units.json.backup-{version}-{timestamp} (e.g., work-units.json.backup-0.7.0-1738329600000). Stored in same directory as work-units.json.
  Version comparison: Use semver-compatible compareVersions(v1, v2) function. Returns -1 (v1 < v2), 0 (equal), or 1 (v1 > v2).
  Migration history structure in work-units.json: migrationHistory: [{version: '0.7.0', applied: '2025-01-31T12:00:00Z', backupPath: 'spec/work-units.json.backup-0.7.0-1738329600000'}]
  Error handling: Migration failure throws error with migration version, does NOT automatically restore backup. User can manually restore from backup file if needed.
  Console output: Use chalk.yellow for warnings, chalk.blue for migration names, chalk.green for success, chalk.red for errors, chalk.dim for backup paths. Show progress for each migration.
  Transaction safety: Each migration runs atomically - backup created first, then migration up() called, then data saved. If up() throws, data is not saved.
  Version field location: Add 'version' field at root level of work-units.json (same level as 'workUnits' and 'states'). Default to undefined for old files (treated as v0.6.0).
  Manual command structure: 'fspec migrate' with options: --version X.Y.Z (migrate to specific version), --status (show migration info), --rollback (undo last migration if down() exists)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Migrations must execute in version order (0.6.0 → 0.7.0 → 0.8.0)
  #   2. Each migration must create a timestamped backup before modifying work-units.json
  #   3. Migration history must be tracked in work-units.json with version, timestamp, and backup path
  #   4. Failed migrations must not corrupt work-units.json (atomic operations)
  #   5. Migrations must be idempotent (running twice produces same result as running once)
  #   6. Version field must be added to work-units.json root level
  #   7. Auto-migration must run transparently on first command after upgrade
  #   8. Manual migration command must be available: fspec migrate --version X.Y.Z
  #   9. Migration runner must detect current version from work-units.json (or default to v0.6.0 if missing)
  #   10. Console output must show migration progress (backup path, migration name, success/failure)
  #   11. Migration registry must be extensible (easy to add future migrations)
  #   12. Backup files must use format: work-units.json.backup-{version}-{timestamp}
  #   13. Create src/commands/migrate-help.ts with CommandHelpConfig for 'fspec migrate' command (auto-loaded by help-registry.ts)
  #   14. Update src/commands/bootstrap.ts and src/utils/slashCommandTemplate.ts to document the migration system and version tracking
  #   15. Add migration system section to bootstrap output explaining version field, auto-migration, and manual migration command
  #   16. Manual restore - migration failure throws error with backup path, user can restore manually if needed. No automatic rollback.
  #   17. Fail immediately with clear error message if work-units.json is corrupted or invalid JSON. Do not attempt automatic repair.
  #   18. Show progress output only (backup path, migration name, success). No warning prompts - auto-migration is expected behavior after upgrade.
  #   19. Yes, support 'fspec migrate --rollback' to undo last migration if down() function exists. Error if down() not implemented.
  #
  # EXAMPLES:
  #   1. User upgrades fspec from 0.6.0 to 0.7.0, runs 'fspec show-work-unit AUTH-001', migration runs automatically with backup, work-units.json updated with version field
  #   2. User runs command when already at v0.7.0, no migrations run (idempotent check), command proceeds normally
  #   3. Migration fails due to invalid JSON structure, error message shown, work-units.json unchanged (backup not restored automatically)
  #   4. User with no version field in work-units.json (v0.6.0 format), migration adds version: '0.7.0' and migrationHistory array
  #   5. User runs manual migration: 'fspec migrate --version 0.7.0', explicit migration with progress output, backup created at spec/work-units.json.backup-0.7.0-1738329600000
  #   6. Multiple migrations needed (v0.6.0 → v0.8.0), both v0.7.0 and v0.8.0 migrations run in sequence, two backups created, final version is 0.8.0
  #   7. Migration history tracking: after migration, work-units.json contains migrationHistory: [{version: '0.7.0', applied: '2025-01-31T12:00:00Z', backupPath: '...'}]
  #   8. User checks migration status: 'fspec migrate --status', output shows current version, migration history, and available migrations
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should migration failure trigger automatic rollback from backup, or should user manually restore?
  #   A: true
  #
  #   Q: Should we support downgrade migrations (down() function), or only forward migrations?
  #   A: true
  #
  #   Q: How many backup files should be kept? Should old backups be auto-deleted after N days?
  #   A: true
  #
  #   Q: If work-units.json is corrupted/invalid, should migration fail immediately or attempt repair?
  #   A: true
  #
  #   Q: Should migration warnings be shown to user (e.g., 'This will modify your work-units.json'), or run silently with just progress output?
  #   A: true
  #
  #   Q: Should 'fspec migrate --rollback' be supported to undo last migration using the down() function?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. Support optional down() function for rollback, but not required for all migrations. Document as 'probably not needed' for most cases.
  #   2. Keep all backup files indefinitely. No auto-deletion. Users can manually delete old backups from spec/ directory if disk space is concern.
  #
  # ========================================
  Background: User Story
    As a fspec developer
    I want to safely evolve the work-units.json schema across versions
    So that future schema changes can be deployed without data loss or manual migration steps

  Scenario: Automatic migration on first command after upgrade
    Given I have fspec v0.6.0 installed with work-units.json
    And work-units.json has NO version field
    When I upgrade fspec to v0.7.0
    And I run "fspec show-work-unit AUTH-001"
    Then the migration system should detect current version as v0.6.0
    And migration to v0.7.0 should run automatically
    And a backup file "spec/work-units.json.backup-0.7.0-{timestamp}" should be created
    And work-units.json should have version field set to "0.7.0"
    And work-units.json should have migrationHistory array with one entry
    And the command "fspec show-work-unit AUTH-001" should complete successfully

  Scenario: No migration when already at target version (idempotent)
    Given I have fspec v0.7.0 installed
    And work-units.json has version field "0.7.0"
    When I run "fspec show-work-unit AUTH-001"
    Then no migrations should run
    And no backup file should be created
    And the command should complete normally without migration output

  Scenario: Migration failure does not corrupt work-units.json
    Given I have fspec v0.6.0 installed with work-units.json
    And work-units.json contains invalid JSON structure that will cause migration to fail
    When I upgrade fspec to v0.7.0
    And I run "fspec show-work-unit AUTH-001"
    Then migration should fail with error message
    And error output should contain the migration version "0.7.0"
    And work-units.json should remain unchanged (original content preserved)
    And backup file should have been created before migration attempt
    And error message should include path to backup file for manual restore
    And command should exit with non-zero code

  Scenario: Add version field to legacy work-units.json
    Given I have work-units.json in v0.6.0 format
    And work-units.json has NO version field
    And work-units.json has NO migrationHistory field
    When migration system runs for v0.7.0
    Then work-units.json should have version field "0.7.0" at root level
    And work-units.json should have migrationHistory array at root level
    And migrationHistory should contain one entry with version "0.7.0"
    And migrationHistory entry should have "applied" timestamp
    And migrationHistory entry should have "backupPath" field

  Scenario: Manual migration with explicit version
    Given I have fspec v0.6.0 installed with work-units.json
    When I run "fspec migrate --version 0.7.0"
    Then migration progress output should be displayed
    And output should show "⚠ Migrating work-units.json from v0.6.0 to 0.7.0..."
    And output should show backup path "spec/work-units.json.backup-0.7.0-{timestamp}"
    And output should show "Applying: stable-indices (0.7.0)"
    And output should show "✓ Convert string arrays to objects with stable IDs"
    And output should show "✓ Migration complete: v0.6.0 → 0.7.0"
    And work-units.json should be updated to v0.7.0 format
    And command should exit with code 0

  Scenario: Sequential migrations for multiple version jumps
    Given I have work-units.json at version "0.6.0"
    And fspec v0.8.0 is installed (requires migrations for 0.7.0 and 0.8.0)
    When migration system runs
    Then v0.7.0 migration should run first
    And backup "spec/work-units.json.backup-0.7.0-{timestamp}" should be created
    And v0.8.0 migration should run second
    And backup "spec/work-units.json.backup-0.8.0-{timestamp}" should be created
    And work-units.json should have version "0.8.0"
    And migrationHistory should have 2 entries (0.7.0 and 0.8.0)
    And console output should show both migrations

  Scenario: Migration history tracking
    Given I have work-units.json at version "0.6.0"
    When migration to v0.7.0 completes successfully
    Then work-units.json should contain migrationHistory array
    And migrationHistory should have structure:
      """
      [{
        "version": "0.7.0",
        "applied": "2025-01-31T12:00:00.000Z",
        "backupPath": "spec/work-units.json.backup-0.7.0-1738329600000"
      }]
      """
    And the "applied" timestamp should be in ISO 8601 format
    And the "backupPath" should match the actual backup file created

  Scenario: Check migration status
    Given I have work-units.json at version "0.7.0"
    And migrationHistory has one entry for v0.7.0
    When I run "fspec migrate --status"
    Then output should show current version "0.7.0"
    And output should show migration history with version "0.7.0"
    And output should show applied timestamp
    And output should show backup path
    And if future migrations exist, output should list available migrations
    And command should exit with code 0

  Scenario: Rollback migration with down() function
    Given I have work-units.json at version "0.7.0"
    And migrationHistory has one entry for v0.7.0
    And the v0.7.0 migration has a down() function implemented
    When I run "fspec migrate --rollback"
    Then migration system should execute down() function for v0.7.0
    And work-units.json should be reverted to v0.6.0 format
    And version field should be set to "0.6.0"
    And migrationHistory entry for v0.7.0 should be removed
    And backup file should be created before rollback
    And output should show "✓ Rolled back migration: v0.7.0 → v0.6.0"
    And command should exit with code 0

  Scenario: Rollback fails when down() function not implemented
    Given I have work-units.json at version "0.7.0"
    And migrationHistory has one entry for v0.7.0
    And the v0.7.0 migration has NO down() function
    When I run "fspec migrate --rollback"
    Then command should fail with error message
    And error output should contain "Migration v0.7.0 does not support rollback (no down() function)"
    And work-units.json should remain at version "0.7.0"
    And command should exit with non-zero code
