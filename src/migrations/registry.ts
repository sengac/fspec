/**
 * Migration registry - central list of all migrations in version order
 */

import { Migration } from './types';
import { compareVersions } from './utils';
import migration001 from './migrations/001-stable-indices';

/**
 * Default version for work-units.json files without a version field
 */
export const DEFAULT_VERSION = '0.6.0';

/**
 * Registry of all available migrations, in version order
 * Add new migrations to this array in ascending version order
 */
const migrations: Migration[] = [
  migration001, // v0.7.0: Stable indices with soft delete
];

/**
 * Get all migrations that need to be applied between two versions
 *
 * @param currentVersion - Current version of work-units.json (undefined = v0.6.0)
 * @param targetVersion - Target version to migrate to
 * @returns Array of migrations to apply, in order
 */
export function getMigrationsToApply(
  currentVersion: string | undefined,
  targetVersion: string
): Migration[] {
  const fromVersion = currentVersion || DEFAULT_VERSION;

  return migrations.filter(migration => {
    const isAfterCurrent = compareVersions(migration.version, fromVersion) > 0;
    const isBeforeOrEqualTarget =
      compareVersions(migration.version, targetVersion) <= 0;
    return isAfterCurrent && isBeforeOrEqualTarget;
  });
}

/**
 * Get all registered migrations
 *
 * @returns Array of all migrations
 */
export function getAllMigrations(): Migration[] {
  return [...migrations];
}

/**
 * Register a migration (for testing or dynamic registration)
 *
 * @param migration - Migration to register
 */
export function registerMigration(migration: Migration): void {
  migrations.push(migration);
  // Sort by version to maintain order
  migrations.sort((a, b) => compareVersions(a.version, b.version));
}

/**
 * Clear all migrations (for testing)
 */
export function clearMigrations(): void {
  migrations.length = 0;
}
