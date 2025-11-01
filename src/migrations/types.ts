/**
 * Type definitions for the migration system
 */

/**
 * Represents the data structure of work-units.json
 */
export interface WorkUnitsData {
  version?: string;
  migrationHistory?: MigrationHistoryEntry[];
  meta: {
    version: string;
    lastUpdated: string;
  };
  workUnits: Record<string, unknown>;
  states: {
    backlog: string[];
    specifying: string[];
    testing: string[];
    implementing: string[];
    validating: string[];
    done: string[];
    blocked: string[];
  };
  [key: string]: unknown;
}

/**
 * Entry in the migration history array
 */
export interface MigrationHistoryEntry {
  version: string;
  applied: string; // ISO 8601 timestamp
  backupPath: string;
}

/**
 * Migration definition interface
 */
export interface Migration {
  version: string;
  name: string;
  description: string;
  up: (data: WorkUnitsData) => WorkUnitsData | Promise<WorkUnitsData>;
  down?: (data: WorkUnitsData) => WorkUnitsData | Promise<WorkUnitsData>;
}

/**
 * Migration status information
 */
export interface MigrationStatus {
  currentVersion: string;
  history: MigrationHistoryEntry[];
  availableMigrations?: Migration[];
}
