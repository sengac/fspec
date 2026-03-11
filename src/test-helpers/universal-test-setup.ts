/**
 * Universal test setup utilities for all fspec filesystem tests.
 *
 * This module provides a unified interface for test setup that follows SOLID/DRY principles
 * and eliminates code duplication across all test files.
 */

import { join } from 'path';
import { createTempTestDir, removeTempTestDir } from './temp-directory';
import {
  createWorkUnitTestEnvironment,
  registerTestPrefix,
} from './work-unit-test-fixtures';
import { createTestFiles } from './test-file-operations';

/**
 * Standard test directory setup with automatic cleanup.
 * Use this for any test that needs a temporary directory.
 */
export interface TestDirectorySetup {
  testDir: string;
  specDir: string;
  workUnitsFile: string;
  cleanup: () => Promise<void>;
}

export async function setupTestDirectory(
  testName: string
): Promise<TestDirectorySetup> {
  const testDir = await createTempTestDir(testName);
  const specDir = join(testDir, 'spec');
  const workUnitsFile = join(specDir, 'work-units.json');

  // Create basic structure
  await createTestFiles(testDir, {
    'spec/work-units.json': {
      data: {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
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
  });

  return {
    testDir,
    specDir,
    workUnitsFile,
    cleanup: async () => await removeTempTestDir(testDir),
  };
}

/**
 * Full work unit test environment with all required files and directories.
 * Use this for tests that need work units, prefixes, epics, etc.
 */
export interface WorkUnitTestSetup extends TestDirectorySetup {
  workUnitsFile: string;
  prefixesFile: string;
  epicsFile: string;
  specDir: string;
  featuresDir: string;
}

export async function setupWorkUnitTest(
  testName: string
): Promise<WorkUnitTestSetup> {
  const testDir = await createTempTestDir(testName);
  const environment = await createWorkUnitTestEnvironment(testDir);

  return {
    testDir,
    ...environment,
    cleanup: async () => await removeTempTestDir(testDir),
  };
}

/**
 * Quick setup for foundation-based tests.
 */
export interface FoundationTestSetup extends TestDirectorySetup {
  foundationFile: string;
  specDir: string;
}

export async function setupFoundationTest(
  testName: string
): Promise<FoundationTestSetup> {
  const testDir = await createTempTestDir(testName);

  const files = await createTestFiles(testDir, {
    'spec/foundation.json': {
      data: {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        project: {
          name: 'Test Project',
          description: 'Test project for testing',
        },
        personas: [],
        capabilities: [],
        rules: [],
        examples: [],
        questions: [],
        architectureNotes: [],
      },
    },
  });

  return {
    testDir,
    foundationFile: files['spec/foundation.json'],
    specDir: `${testDir}/spec`,
    cleanup: async () => await removeTempTestDir(testDir),
  };
}

/**
 * Setup with both foundation and work units.
 */
export interface FullTestSetup extends WorkUnitTestSetup {
  foundationFile: string;
}

export async function setupFullTest(testName: string): Promise<FullTestSetup> {
  const workUnitSetup = await setupWorkUnitTest(testName);

  const files = await createTestFiles(workUnitSetup.testDir, {
    'spec/foundation.json': {
      data: {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        project: {
          name: 'Test Project',
          description: 'Test project for testing',
        },
        personas: [],
        capabilities: [],
        rules: [],
        examples: [],
        questions: [],
        architectureNotes: [],
      },
    },
  });

  return {
    ...workUnitSetup,
    foundationFile: files['spec/foundation.json'],
  };
}

/**
 * Register commonly used test prefixes.
 */
export async function registerCommonTestPrefixes(
  testDir: string
): Promise<void> {
  await registerTestPrefix(testDir, 'TEST', 'Test prefix for testing');
  await registerTestPrefix(testDir, 'AUTH', 'Authentication features');
  await registerTestPrefix(testDir, 'DASH', 'Dashboard features');
  await registerTestPrefix(testDir, 'API', 'API features');
}

/**
 * Git repository test setup with all necessary configurations.
 * Use this for tests that need git operations.
 */
export interface GitTestSetup extends TestDirectorySetup {
  initGit: () => Promise<void>;
}

export async function setupGitTest(testName: string): Promise<GitTestSetup> {
  const baseSetup = await setupTestDirectory(testName);

  const initGit = async () => {
    const { gitInit, gitSetConfig, gitAdd, gitCommit } = await import(
      '@sengac/codelet-napi'
    );

    // Initialize git repository
    gitInit(baseSetup.testDir, 'main');

    // Configure git
    gitSetConfig(baseSetup.testDir, 'user.name', 'Test User');
    gitSetConfig(baseSetup.testDir, 'user.email', 'test@example.com');

    // Create initial commit so HEAD exists
    const { writeFile } = await import('fs/promises');
    const { join } = await import('path');
    await writeFile(join(baseSetup.testDir, 'README.md'), '# Test Project');
    gitAdd(baseSetup.testDir, 'README.md');
    gitCommit(
      baseSetup.testDir,
      'Initial commit',
      'Test User',
      'test@example.com'
    );
  };

  return {
    ...baseSetup,
    initGit,
  };
}
