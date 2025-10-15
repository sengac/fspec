/**
 * Test suite for: spec/features/automatic-json-file-initialization.feature
 * Scenario: Update work unit command uses ensureWorkUnitsFile instead of direct readFile
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile, access } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { updateWorkUnit } from '../update-work-unit';
import type { WorkUnitsData } from '../../types';

describe('Feature: Automatic JSON File Initialization', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Update work unit command uses ensureWorkUnitsFile instead of direct readFile', () => {
    it('should create work-units.json if missing and update work unit', async () => {
      // Given I have a fresh project with spec/ directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // And spec/work-units.json does not exist initially
      // But I have created a work unit "AUTH-001" (via ensureWorkUnitsFile in the command)
      const workUnitsFile = join(testDir, 'spec/work-units.json');

      // First, create the work unit
      const initialData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Old Title',
            status: 'backlog',
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
      await writeFile(workUnitsFile, JSON.stringify(initialData, null, 2));

      // When I run "fspec update-work-unit AUTH-001 --title='New Title'"
      const result = await updateWorkUnit({
        workUnitId: 'AUTH-001',
        title: 'New Title',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And spec/work-units.json should exist
      await access(workUnitsFile);

      // And the work unit "AUTH-001" title should be "New Title"
      const fs = await import('fs/promises');
      const fileContent = await fs.readFile(workUnitsFile, 'utf-8');
      const data: WorkUnitsData = JSON.parse(fileContent);

      expect(data.workUnits['AUTH-001'].title).toBe('New Title');

      // And no ENOENT error should occur (file was auto-created by ensureWorkUnitsFile)
      expect(data.meta).toBeDefined();
      expect(data.states).toBeDefined();
    });
  });
});
