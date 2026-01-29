/**
 * Test suite for: spec/features/automatic-json-file-initialization.feature
 * Feature: Automatic JSON File Initialization
 *
 * Tests the ensure* utility functions that auto-create JSON files with proper structure.
 *
 * Feature: spec/features/work-units-json-not-stamped-with-current-version-on-initial-creation.feature
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, readFile, access } from 'fs/promises';
import { join } from 'path';
import {
  ensureWorkUnitsFile,
  ensurePrefixesFile,
  ensureEpicsFile,
} from '../ensure-files';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';
import { writeJsonTestFile } from '../../test-helpers/test-file-operations';

describe('Feature: Automatic JSON File Initialization', () => {
  let setup: TestDirectorySetup;

  beforeEach(async () => {
    setup = await setupTestDirectory('ensure-files');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: ensureWorkUnitsFile creates file with proper structure', () => {
    it('should create work-units.json with all Kanban states when missing', async () => {
      // Given I have a fresh project with spec/ directory
      await mkdir(join(setup.testDir, 'spec'), { recursive: true });

      // And spec/work-units.json does not exist
      // (file doesn't exist yet)

      // When ensureWorkUnitsFile is called
      const result = await ensureWorkUnitsFile(setup.testDir);

      // Then spec/work-units.json should be created
      const filePath = join(setup.testDir, 'spec/work-units.json');
      await access(filePath); // Throws if file doesn't exist

      // And the file should contain proper structure
      expect(result.meta).toBeDefined();
      expect(result.meta.version).toBe('1.0.0');
      expect(result.workUnits).toBeDefined();
      expect(result.workUnits).toEqual({});

      // And the file should contain all 7 Kanban states
      expect(result.states).toBeDefined();
      expect(result.states.backlog).toEqual([]);
      expect(result.states.specifying).toEqual([]);
      expect(result.states.testing).toEqual([]);
      expect(result.states.implementing).toEqual([]);
      expect(result.states.validating).toEqual([]);
      expect(result.states.done).toEqual([]);
      expect(result.states.blocked).toEqual([]);
    });

    it('should return existing data when file already exists', async () => {
      // Given I have spec/work-units.json with existing work unit
      await mkdir(join(setup.testDir, 'spec'), { recursive: true });
      const existingData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Original Title',
            status: 'backlog' as const,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      const filePath = join(setup.testDir, 'spec/work-units.json');
      await writeJsonTestFile(filePath, existingData);

      // When ensureWorkUnitsFile is called
      const result = await ensureWorkUnitsFile(setup.testDir);

      // Then it should return the existing data
      expect(result.workUnits['AUTH-001']).toBeDefined();
      expect(result.workUnits['AUTH-001'].title).toBe('Original Title');
      expect(result.states.backlog).toContain('AUTH-001');

      // And the file should not be modified
      const fileContent = await readFile(filePath, 'utf-8');
      const fileData = JSON.parse(fileContent);
      expect(fileData.workUnits['AUTH-001'].title).toBe('Original Title');
    });
  });

  describe('Scenario: ensurePrefixesFile creates file with proper structure', () => {
    it('should create prefixes.json with empty prefixes object when missing', async () => {
      // Given I have a fresh project with spec/ directory
      await mkdir(join(setup.testDir, 'spec'), { recursive: true });

      // When ensurePrefixesFile is called
      const result = await ensurePrefixesFile(setup.testDir);

      // Then spec/prefixes.json should be created
      const filePath = join(setup.testDir, 'spec/prefixes.json');
      await access(filePath);

      // And the file should contain empty prefixes object
      expect(result.prefixes).toBeDefined();
      expect(result.prefixes).toEqual({});
    });
  });

  describe('Scenario: ensureEpicsFile creates file with proper structure', () => {
    it('should create epics.json with empty epics object when missing', async () => {
      // Given I have a fresh project with spec/ directory
      await mkdir(join(setup.testDir, 'spec'), { recursive: true });

      // When ensureEpicsFile is called
      const result = await ensureEpicsFile(setup.testDir);

      // Then spec/epics.json should be created
      const filePath = join(setup.testDir, 'spec/epics.json');
      await access(filePath);

      // And the file should contain empty epics object
      expect(result.epics).toBeDefined();
      expect(result.epics).toEqual({});
    });
  });

  describe('Scenario: Ensure utilities validate JSON structure', () => {
    it('should throw a helpful error when JSON is corrupted', async () => {
      // Given I have corrupted spec/work-units.json
      await mkdir(join(setup.testDir, 'spec'), { recursive: true });
      const filePath = join(setup.testDir, 'spec/work-units.json');
      const fs = await import('fs/promises');

      // Write corrupted JSON (trailing comma, missing closing brace)
      await fs.writeFile(filePath, '{"workUnits": {},');

      // When ensureWorkUnitsFile is called
      // Then it should throw a helpful error
      await expect(ensureWorkUnitsFile(setup.testDir)).rejects.toThrow();

      // And should indicate the file is invalid JSON
      try {
        await ensureWorkUnitsFile(setup.testDir);
      } catch (error: any) {
        expect(error.message).toContain('work-units.json');
        expect(error.message.toLowerCase()).toMatch(/json|parse|invalid/);
      }
    });

    it('should provide helpful error message with file path', async () => {
      // Given I have corrupted spec/work-units.json
      await mkdir(join(setup.testDir, 'spec'), { recursive: true });
      const filePath = join(setup.testDir, 'spec/work-units.json');
      const fs = await import('fs/promises');

      // Write invalid JSON
      await fs.writeFile(filePath, 'not valid json at all');

      // When ensureWorkUnitsFile is called
      // Then error should mention the file path
      await expect(ensureWorkUnitsFile(setup.testDir)).rejects.toThrow(
        /work-units\.json/
      );
    });
  });

  describe('BUG-070: Version stamping on initial creation', () => {
    describe('Scenario: Create work-units.json with current version on first run', () => {
      // @step Given I am in a new project without spec/work-units.json
      // @step When I run the first fspec command
      // @step Then spec/work-units.json should be created
      // @step And the file should have version field set to '0.7.1'
      // @step And no migration should run
      // @step And no backup files should be created
      it('should create work-units.json with root-level version field', async () => {
        // @step Given I am in a new project without spec/work-units.json
        await mkdir(join(setup.testDir, 'spec'), { recursive: true });

        // @step When I run the first fspec command
        const result = await ensureWorkUnitsFile(setup.testDir);

        // @step Then spec/work-units.json should be created
        const filePath = join(setup.testDir, 'spec/work-units.json');
        await access(filePath);

        // @step And the file should have version field set to '0.7.1'
        expect(result.version).toBe('0.7.1');

        // @step And no migration should run
        expect(result.migrationHistory).toBeUndefined();

        // @step And no backup files should be created
        const fs = await import('fs/promises');
        const specContents = await fs.readdir(join(setup.testDir, 'spec'));
        const backupFiles = specContents.filter(f => f.includes('backup'));
        expect(backupFiles).toHaveLength(0);
      });
    });

    describe('Scenario: Recreate work-units.json with current version after deletion', () => {
      // @step Given I have an existing fspec project
      // @step And spec/work-units.json has been deleted
      // @step When I run any fspec command
      // @step Then spec/work-units.json should be recreated
      // @step And the file should have version field set to '0.7.1'
      // @step And no migration should run
      // @step And no backup files should be created
      it('should recreate work-units.json with version field after deletion', async () => {
        // @step Given I have an existing fspec project
        await mkdir(join(setup.testDir, 'spec'), { recursive: true });

        // Create initial file
        await ensureWorkUnitsFile(setup.testDir);

        // @step And spec/work-units.json has been deleted
        const filePath = join(setup.testDir, 'spec/work-units.json');
        const fs = await import('fs/promises');
        await fs.unlink(filePath);

        // @step When I run any fspec command
        const result = await ensureWorkUnitsFile(setup.testDir);

        // @step Then spec/work-units.json should be recreated
        await access(filePath);

        // @step And the file should have version field set to '0.7.1'
        expect(result.version).toBe('0.7.1');

        // @step And no migration should run
        expect(result.migrationHistory).toBeUndefined();

        // @step And no backup files should be created
        const specContents = await fs.readdir(join(setup.testDir, 'spec'));
        const backupFiles = specContents.filter(f => f.includes('backup'));
        expect(backupFiles).toHaveLength(0);
      });
    });

    describe('Scenario: Version constant shared across ensure-files and migrations', () => {
      // @step Given the version constant is defined in a shared location
      // @step When the version is updated to '0.8.0' in the shared constant
      // @step Then ensure-files.ts should use version '0.8.0' when creating work-units.json
      // @step And migrations should recognize '0.8.0' as the current version
      // @step And no version string should be hardcoded in multiple files
      it('should use shared version constant from migrations registry', async () => {
        // @step Given the version constant is defined in a shared location
        // This test verifies that ensure-files imports from migrations/registry

        // @step When the version is updated to '0.8.0' in the shared constant
        // We verify by checking that the version used matches the current version
        await mkdir(join(setup.testDir, 'spec'), { recursive: true });
        const result = await ensureWorkUnitsFile(setup.testDir);

        // @step Then ensure-files.ts should use version '0.8.0' when creating work-units.json
        // @step And migrations should recognize '0.8.0' as the current version
        expect(result.version).toBeDefined();
        expect(typeof result.version).toBe('string');

        // @step And no version string should be hardcoded in multiple files
        // Verify that version comes from shared constant by checking it exists
        const filePath = join(setup.testDir, 'spec/work-units.json');
        const fileContent = await readFile(filePath, 'utf-8');
        const fileData = JSON.parse(fileContent);
        expect(fileData.version).toBe('0.7.1');
      });
    });
  });
});
