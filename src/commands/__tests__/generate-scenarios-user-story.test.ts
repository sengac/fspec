/**
 * Test suite for: spec/features/feature-file-prefill-detection.feature
 * Scenario: User story from Example Mapping generates complete Background
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile, readFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { generateScenarios } from '../generate-scenarios';
import type { WorkUnitsData } from '../../types';

describe('Feature: Feature File Prefill Detection and CLI Enforcement', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: User story from Example Mapping generates complete Background', () => {
    it('should generate Background with complete user story when Example Mapping has all fields', async () => {
      // Given a work unit has complete user story fields in Example Mapping
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'TEST-003': {
            id: 'TEST-003',
            title: 'Test feature with user story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
            userStory: {
              role: 'developer',
              action: 'generate scenarios from examples',
              benefit: 'complete feature files are created automatically',
            },
            examples: [
              'Example 1: User logs in successfully',
              'Example 2: User sees error for invalid password',
            ],
          },
        },
        states: {
          backlog: [],
          specifying: ['TEST-003'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run fspec generate-scenarios
      const result = await generateScenarios({
        workUnitId: 'TEST-003',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      // Then the Background section should contain the complete user story without placeholders
      const featureContent = await readFile(
        join(testDir, 'spec/features/test-feature-with-user-story.feature'),
        'utf-8'
      );

      // Verify complete user story fields are present in Background
      expect(featureContent).toContain('As a developer');
      expect(featureContent).toContain('I want to generate scenarios from examples');
      expect(featureContent).toContain(
        'So that complete feature files are created automatically'
      );

      // Verify NO placeholders in Background section (scenarios may still have them)
      const backgroundSection = featureContent.match(
        /Background:[\s\S]*?(?=\n  Scenario:)/
      )?.[0];
      expect(backgroundSection).toBeDefined();
      expect(backgroundSection).not.toContain('[role]');
      expect(backgroundSection).not.toContain('[action]');
      expect(backgroundSection).not.toContain('[benefit]');
    });

    it('should generate placeholders when user story fields are missing', async () => {
      // Given a work unit WITHOUT user story fields
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'TEST-004': {
            id: 'TEST-004',
            title: 'Test feature without user story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
            examples: ['Example 1: Something happens'],
          },
        },
        states: {
          backlog: [],
          specifying: ['TEST-004'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run fspec generate-scenarios
      const result = await generateScenarios({
        workUnitId: 'TEST-004',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      // Then placeholders should be present (needs user story via set-user-story)
      const featureContent = await readFile(
        join(testDir, 'spec/features/test-feature-without-user-story.feature'),
        'utf-8'
      );

      // Should have placeholders
      expect(featureContent).toMatch(/\[role\]|\[action\]|\[benefit\]/);
    });
  });
});
