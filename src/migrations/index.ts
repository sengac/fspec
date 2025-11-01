/**
 * Migration runner - orchestrates the migration process
 */

import { readFile, writeFile, copyFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import { WorkUnitsData, MigrationHistoryEntry } from './types';
import { getMigrationsToApply, DEFAULT_VERSION } from './registry';

/**
 * Ensure work-units.json is at the latest version
 * Automatically runs migrations if needed
 *
 * @param cwd - Project root directory
 * @param targetVersion - Target version to migrate to
 * @returns Updated work-units data
 */
export async function ensureLatestVersion(
  cwd: string,
  targetVersion: string
): Promise<WorkUnitsData> {
  const workUnitsPath = join(cwd, 'spec', 'work-units.json');

  // Read current data
  const fileContent = await readFile(workUnitsPath, 'utf-8');
  let data: WorkUnitsData = JSON.parse(fileContent);

  const currentVersion = data.version || DEFAULT_VERSION;

  // Get migrations to apply
  const migrationsToApply = getMigrationsToApply(currentVersion, targetVersion);

  if (migrationsToApply.length === 0) {
    // No migrations needed - already at target version
    return data;
  }

  // Run each migration in sequence
  for (const migration of migrationsToApply) {
    console.log(
      chalk.yellow(
        `⚠ Migrating work-units.json from v${currentVersion} to ${migration.version}...`
      )
    );

    // Create backup before migration
    const timestamp = Date.now();
    const backupPath = `spec/work-units.json.backup-${migration.version}-${timestamp}`;
    const absoluteBackupPath = join(cwd, backupPath);

    await copyFile(workUnitsPath, absoluteBackupPath);
    console.log(chalk.dim(`Backup created: ${backupPath}`));

    try {
      // Run migration
      console.log(
        chalk.blue(`Applying: ${migration.name} (${migration.version})`)
      );
      data = await Promise.resolve(migration.up(data));

      // Update version field
      data.version = migration.version;

      // Initialize migrationHistory if it doesn't exist
      if (!data.migrationHistory) {
        data.migrationHistory = [];
      }

      // Add history entry
      const historyEntry: MigrationHistoryEntry = {
        version: migration.version,
        applied: new Date().toISOString(),
        backupPath: backupPath,
      };
      data.migrationHistory.push(historyEntry);

      // Save migrated data
      await writeFile(workUnitsPath, JSON.stringify(data, null, 2), 'utf-8');

      console.log(
        chalk.green(
          `✓ Migration complete: v${currentVersion} → ${migration.version}`
        )
      );
    } catch (error) {
      // Migration failed - throw error with backup path
      const errorMessage = `Migration to ${migration.version} failed: ${
        error instanceof Error ? error.message : String(error)
      }\nBackup file: ${backupPath}\nRestore manually if needed.`;
      throw new Error(errorMessage);
    }
  }

  return data;
}

/**
 * Rollback the last migration using its down() function
 *
 * @param cwd - Project root directory
 * @returns Updated work-units data
 */
export async function rollbackMigration(cwd: string): Promise<WorkUnitsData> {
  const workUnitsPath = join(cwd, 'spec', 'work-units.json');

  // Read current data
  const fileContent = await readFile(workUnitsPath, 'utf-8');
  let data: WorkUnitsData = JSON.parse(fileContent);

  if (!data.migrationHistory || data.migrationHistory.length === 0) {
    throw new Error('No migrations to rollback');
  }

  // Get last migration from history
  const lastMigration = data.migrationHistory[data.migrationHistory.length - 1];
  const migrationVersion = lastMigration.version;

  // Find the migration definition
  const { getAllMigrations } = await import('./registry');
  const migrations = getAllMigrations();
  const migration = migrations.find(m => m.version === migrationVersion);

  if (!migration) {
    throw new Error(`Migration ${migrationVersion} not found in registry`);
  }

  if (!migration.down) {
    throw new Error(
      `Migration v${migrationVersion} does not support rollback (no down() function)`
    );
  }

  // Create backup before rollback
  const timestamp = Date.now();
  const backupPath = `spec/work-units.json.backup-rollback-${migrationVersion}-${timestamp}`;
  const absoluteBackupPath = join(cwd, backupPath);

  await copyFile(workUnitsPath, absoluteBackupPath);
  console.log(chalk.dim(`Backup created: ${backupPath}`));

  // Run down() migration
  console.log(
    chalk.blue(`Rolling back: ${migration.name} (${migration.version})`)
  );
  data = await Promise.resolve(migration.down(data));

  // Remove last migration from history
  data.migrationHistory.pop();

  // Update version to previous migration version or remove field if no migrations left
  if (data.migrationHistory.length > 0) {
    data.version =
      data.migrationHistory[data.migrationHistory.length - 1].version;
  } else {
    delete data.version;
    delete data.migrationHistory;
  }

  // Save rolled-back data
  await writeFile(workUnitsPath, JSON.stringify(data, null, 2), 'utf-8');

  const previousVersion =
    data.version || DEFAULT_VERSION.replace(/\.\d+$/, '.0'); // Approximate previous version
  console.log(
    chalk.green(
      `✓ Rolled back migration: v${migrationVersion} → v${previousVersion}`
    )
  );

  return data;
}

/**
 * Get migration status for current project
 *
 * @param cwd - Project root directory
 * @returns Migration status information
 */
export async function getMigrationStatus(cwd: string) {
  const workUnitsPath = join(cwd, 'spec', 'work-units.json');

  const fileContent = await readFile(workUnitsPath, 'utf-8');
  const data: WorkUnitsData = JSON.parse(fileContent);

  const currentVersion = data.version || DEFAULT_VERSION;
  const history = data.migrationHistory || [];

  const { getAllMigrations } = await import('./registry');
  const availableMigrations = getAllMigrations();

  return {
    currentVersion,
    history,
    availableMigrations,
  };
}
