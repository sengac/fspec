/**
 * Test suite for: spec/features/feature-file-prefill-detection.feature
 * Scenario: Workflow blocking prevents status change with prefill
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { updateWorkUnitStatus } from '../update-work-unit-status';
import type { WorkUnitsData } from '../../types';

describe('Feature: Feature File Prefill Detection and CLI Enforcement', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Workflow blocking prevents status change with prefill', () => {
    it('should fail when trying to move to testing with prefill in linked feature', async () => {
      // Given a linked feature file contains prefill placeholders
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

      // Create feature file with prefill
      const featureContent = `@TEST-001
Feature: Test Feature

  Background: User Story
    As a [role]
    I want to [action]
    So that [benefit]

  @TEST-001
  Scenario: Test scenario
    Given [precondition]
    When [action]
    Then [expected outcome]
`;

      await writeFile(
        join(testDir, 'spec/features/test-feature.feature'),
        featureContent
      );

      // Create work unit linked to this feature
      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            title: 'Test work unit',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
            linkedFeatures: ['test-feature'],
          },
        },
        states: {
          backlog: [],
          specifying: ['TEST-001'],
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

      // When I try to update work unit status to testing
      // Then the command should fail with an error listing the prefill placeholders
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'TEST-001',
          status: 'testing',
          cwd: testDir,
        })
      ).rejects.toThrow(/prefill/i);

      // Verify error message mentions the placeholders
      try {
        await updateWorkUnitStatus({
          workUnitId: 'TEST-001',
          status: 'testing',
          cwd: testDir,
        });
      } catch (error: any) {
        expect(error.message).toMatch(/\[role\]|\[action\]|\[benefit\]/);
        expect(error.message.toLowerCase()).toContain('prefill');
      }
    });

    it('should succeed when feature file has no prefill', async () => {
      // Given a linked feature file WITHOUT prefill placeholders
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

      // Create feature file WITHOUT prefill
      const featureContent = `@TEST-002
Feature: Complete Feature

  Background: User Story
    As a developer
    I want to test the workflow
    So that I can verify prefill detection works

  @TEST-002
  Scenario: Complete scenario
    Given I have a complete feature file
    When I move to testing
    Then it should succeed
`;

      await writeFile(
        join(testDir, 'spec/features/complete-feature.feature'),
        featureContent
      );

      // Create work unit linked to this feature
      const workUnitsData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'TEST-002': {
            id: 'TEST-002',
            title: 'Complete work unit',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
            linkedFeatures: ['complete-feature'],
          },
        },
        states: {
          backlog: [],
          specifying: ['TEST-002'],
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

      // When I try to update work unit status to testing
      // Then the command should succeed (no prefill to block)
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-002',
        status: 'testing',
        cwd: testDir,
      });

      expect(result.success).toBe(true);
    });
  });
});
