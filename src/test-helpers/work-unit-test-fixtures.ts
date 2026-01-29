/**
 * Test fixtures and utilities for work unit testing.
 *
 * Provides common setup patterns for work unit tests following SOLID/DRY principles.
 */

import { join } from 'path';
import type { WorkUnitsData, PrefixesData, EpicsData } from '../types';
import {
  createJsonTestFile,
  ensureTestDirectory,
  createTestFiles,
  readJsonTestFile,
} from './test-file-operations';

/**
 * Create initial work units file structure in a test directory.
 */
export async function createWorkUnitsFixture(
  testDir: string,
  initialData?: Partial<WorkUnitsData>
): Promise<string> {
  const specDir = join(testDir, 'spec');

  // Create work units file with default structure
  const defaultData: WorkUnitsData = {
    workUnits: {},
    states: {
      backlog: [],
      specifying: [],
      testing: [],
      implementing: [],
      validating: [],
      done: [],
      blocked: [],
    },
    ...initialData,
  };

  return await createJsonTestFile(specDir, 'work-units.json', defaultData);
}

/**
 * Create prefixes file for testing.
 */
export async function createPrefixesFixture(
  testDir: string,
  prefixes: Record<string, { description: string }> = {}
): Promise<string> {
  const specDir = join(testDir, 'spec');
  const data: PrefixesData = { prefixes };
  return await createJsonTestFile(specDir, 'prefixes.json', data);
}

/**
 * Create epics file for testing.
 */
export async function createEpicsFixture(
  testDir: string,
  epics: Record<string, { id: string; title: string; workUnits: string[] }> = {}
): Promise<string> {
  const specDir = join(testDir, 'spec');
  const data: EpicsData = { epics };
  return await createJsonTestFile(specDir, 'epics.json', data);
}

/**
 * Create complete work unit test environment with all necessary files.
 */
export async function createWorkUnitTestEnvironment(testDir: string): Promise<{
  workUnitsFile: string;
  prefixesFile: string;
  epicsFile: string;
  specDir: string;
  featuresDir: string;
}> {
  const specDir = join(testDir, 'spec');
  const featuresDir = join(specDir, 'features');

  // Create directory structure
  await ensureTestDirectory(featuresDir);

  // Create all necessary files using the shared utilities
  const files = await createTestFiles(testDir, {
    'spec/work-units.json': {
      data: {
        workUnits: {},
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      },
    },
    'spec/prefixes.json': {
      data: { prefixes: {} },
    },
    'spec/epics.json': {
      data: { epics: {} },
    },
  });

  return {
    workUnitsFile: files['spec/work-units.json'],
    prefixesFile: files['spec/prefixes.json'],
    epicsFile: files['spec/epics.json'],
    specDir,
    featuresDir,
  };
}

/**
 * Register a test prefix for use in tests.
 */
export async function registerTestPrefix(
  testDir: string,
  prefix: string,
  description: string = `${prefix} features`
): Promise<void> {
  // Read existing prefixes first
  const prefixesFile = join(testDir, 'spec', 'prefixes.json');
  let existingPrefixes: Record<string, { description: string }> = {};

  try {
    const existingData = await readJsonTestFile<{
      prefixes: Record<string, { description: string }>;
    }>(prefixesFile);
    existingPrefixes = existingData.prefixes || {};
  } catch {
    // File doesn't exist yet, start with empty
  }

  // Add the new prefix
  existingPrefixes[prefix] = { description };

  // Write back all prefixes
  await createPrefixesFixture(testDir, existingPrefixes);
}
