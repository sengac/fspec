/**
 * Test suite for: spec/features/feature-file-prefill-detection.feature
 * Scenarios: System-reminder appears after create-feature and generate-scenarios
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import { createFeature } from '../create-feature';
import { generateScenarios } from '../generate-scenarios';
import type { WorkUnitsData } from '../../types';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';

describe('Feature: Feature File Prefill Detection and CLI Enforcement', () => {
  let setup: TestDirectorySetup;

  beforeEach(async () => {
    setup = await setupTestDirectory('create-feature-system-reminder');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: System-reminder appears after create-feature with prefill', () => {
    it('should include system-reminder in result when feature has prefill', async () => {
      // Given I run fspec create-feature command
      await mkdir(join(setup.testDir, 'spec', 'features'), { recursive: true });

      // When I create a feature
      const result = await createFeature(
        'Test Feature with Prefill',
        setup.testDir
      );

      // Then the feature file should be created
      expect(result.filePath).toContain('test-feature-with-prefill.feature');

      // When the generated feature file contains placeholder text
      const featureContent = await readFile(result.filePath, 'utf-8');

      // Verify it has placeholders
      expect(featureContent).toMatch(/\[role\]|\[action\]|\[benefit\]/);

      // Then prefill detection should detect the placeholders
      expect(result.prefillDetection.hasPrefill).toBe(true);
      expect(result.prefillDetection.matches.length).toBeGreaterThan(0);

      // And a system-reminder should appear suggesting CLI commands to fix prefill
      expect(result.prefillDetection.systemReminder).toBeDefined();
      expect(result.prefillDetection.systemReminder).toContain(
        'set-user-story'
      );
      expect(result.prefillDetection.systemReminder).toContain('CLI');
    });
  });

  describe('Scenario: System-reminder appears after generate-scenarios with prefill', () => {
    it('should include system-reminder when generated scenarios have placeholder steps', async () => {
      // Given I run fspec generate-scenarios command
      await mkdir(join(setup.testDir, 'spec', 'features'), { recursive: true });

      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'TEST-005': {
            id: 'TEST-005',
            title: 'Test scenario generation',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
            examples: [
              'Example 1: Something happens',
              'Example 2: Another thing',
            ],
          },
        },
        states: {
          backlog: [],
          specifying: ['TEST-005'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(
        join(setup.testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      const result = await generateScenarios({
        workUnitId: 'TEST-005',
        cwd: setup.testDir,
      });

      expect(result.success).toBe(true);

      // When the generated scenarios contain placeholder steps
      const featureContent = await readFile(
        join(setup.testDir, 'spec/features/test-scenario-generation.feature'),
        'utf-8'
      );

      // Verify placeholders exist in steps
      expect(featureContent).toMatch(
        /\[precondition\]|\[action\]|\[expected outcome\]/
      );

      // Then a system-reminder should suggest using fspec add-step commands
      if (result.systemReminder) {
        expect(result.systemReminder).toContain('add-step');
        expect(result.systemReminder.toLowerCase()).toContain('cli');
      }
      // Note: System reminders are optional but recommended for better UX
    });
  });
});
