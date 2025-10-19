/**
 * Test suite for: spec/features/automatic-json-file-initialization.feature
 * Feature: Automatic JSON File Initialization
 *
 * Tests the ensure* utility functions that auto-create JSON files with proper structure.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, readFile, access } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import {
  ensureWorkUnitsFile,
  ensurePrefixesFile,
  ensureEpicsFile,
} from '../ensure-files';

describe('Feature: Automatic JSON File Initialization', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: ensureWorkUnitsFile creates file with proper structure', () => {
    it('should create work-units.json with all Kanban states when missing', async () => {
      // Given I have a fresh project with spec/ directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And spec/work-units.json does not exist
      // (file doesn't exist yet)

      // When ensureWorkUnitsFile is called
      const result = await ensureWorkUnitsFile(testDir);

      // Then spec/work-units.json should be created
      const filePath = join(testDir, 'spec/work-units.json');
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
      await mkdir(join(testDir, 'spec'), { recursive: true });
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
      await mkdir(join(testDir, 'spec'), { recursive: true });
      const filePath = join(testDir, 'spec/work-units.json');
      await mkdir(join(testDir, 'spec'), { recursive: true });
      const fs = await import('fs/promises');
      await fs.writeFile(filePath, JSON.stringify(existingData, null, 2));

      // When ensureWorkUnitsFile is called
      const result = await ensureWorkUnitsFile(testDir);

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
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // When ensurePrefixesFile is called
      const result = await ensurePrefixesFile(testDir);

      // Then spec/prefixes.json should be created
      const filePath = join(testDir, 'spec/prefixes.json');
      await access(filePath);

      // And the file should contain empty prefixes object
      expect(result.prefixes).toBeDefined();
      expect(result.prefixes).toEqual({});
    });
  });

  describe('Scenario: ensureEpicsFile creates file with proper structure', () => {
    it('should create epics.json with empty epics object when missing', async () => {
      // Given I have a fresh project with spec/ directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // When ensureEpicsFile is called
      const result = await ensureEpicsFile(testDir);

      // Then spec/epics.json should be created
      const filePath = join(testDir, 'spec/epics.json');
      await access(filePath);

      // And the file should contain empty epics object
      expect(result.epics).toBeDefined();
      expect(result.epics).toEqual({});
    });
  });

  describe('Scenario: Ensure utilities validate JSON structure', () => {
    it('should throw a helpful error when JSON is corrupted', async () => {
      // Given I have corrupted spec/work-units.json
      await mkdir(join(testDir, 'spec'), { recursive: true });
      const filePath = join(testDir, 'spec/work-units.json');
      const fs = await import('fs/promises');

      // Write corrupted JSON (trailing comma, missing closing brace)
      await fs.writeFile(filePath, '{"workUnits": {},');

      // When ensureWorkUnitsFile is called
      // Then it should throw a helpful error
      await expect(ensureWorkUnitsFile(testDir)).rejects.toThrow();

      // And should indicate the file is invalid JSON
      try {
        await ensureWorkUnitsFile(testDir);
      } catch (error: any) {
        expect(error.message).toContain('work-units.json');
        expect(error.message.toLowerCase()).toMatch(/json|parse|invalid/);
      }
    });

    it('should provide helpful error message with file path', async () => {
      // Given I have corrupted spec/work-units.json
      await mkdir(join(testDir, 'spec'), { recursive: true });
      const filePath = join(testDir, 'spec/work-units.json');
      const fs = await import('fs/promises');

      // Write invalid JSON
      await fs.writeFile(filePath, 'not valid json at all');

      // When ensureWorkUnitsFile is called
      // Then error should mention the file path
      await expect(ensureWorkUnitsFile(testDir)).rejects.toThrow(
        /work-units\.json/
      );
    });
  });
});
