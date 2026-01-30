/**
 * Test suite for: spec/features/feature-file-prefill-detection.feature
 * Scenario: Workflow blocking prevents status change with prefill
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { updateWorkUnitStatus } from '../update-work-unit-status';
import type { WorkUnitsData } from '../../types';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';
import {
  writeJsonTestFile,
  ensureTestDirectory,
  createTestFile,
} from '../../test-helpers/test-file-operations';

describe('Feature: Feature File Prefill Detection and CLI Enforcement', () => {
  let setup: TestDirectorySetup;

  beforeEach(async () => {
    setup = await setupTestDirectory('prefill-workflow-blocking');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Workflow blocking prevents status change with prefill', () => {
    it('should fail when trying to move to testing with prefill in linked feature', async () => {
      // Given a linked feature file contains prefill placeholders
      await ensureTestDirectory(join(setup.testDir, 'spec', 'features'));

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

      await createTestFile(
        join(setup.testDir, 'spec/features'),
        'test-feature.feature',
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
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
            linkedFeatures: ['test-feature'],
            rules: [
              'Feature file must not contain prefill placeholders before testing',
            ],
            examples: ['All [placeholder] values replaced with actual content'],
            architectureNotes: [
              'Implementation: Scan feature files for bracket-enclosed placeholders',
            ],
            attachments: ['spec/attachments/TEST-001/ast-research.json'],
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

      await writeJsonTestFile(
        join(setup.testDir, 'spec/work-units.json'),
        workUnitsData
      );

      // When I try to update work unit status to testing
      // Then the command should fail with an error listing the prefill placeholders
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'TEST-001',
          status: 'testing',
          cwd: setup.testDir,
        })
      ).rejects.toThrow(/prefill/i);

      // Verify error message mentions the placeholders
      try {
        await updateWorkUnitStatus({
          workUnitId: 'TEST-001',
          status: 'testing',
          cwd: setup.testDir,
        });
      } catch (error: any) {
        expect(error.message).toMatch(/\[role\]|\[action\]|\[benefit\]/);
        expect(error.message.toLowerCase()).toContain('prefill');
      }
    });

    it('should succeed when feature file has no prefill', async () => {
      // Given a linked feature file WITHOUT prefill placeholders
      await ensureTestDirectory(join(setup.testDir, 'spec', 'features'));

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

      await createTestFile(
        join(setup.testDir, 'spec/features'),
        'complete-feature.feature',
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
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
            linkedFeatures: ['complete-feature'],
            rules: [
              'Complete feature files can move to testing without prefill blocking',
            ],
            examples: [
              'Feature with fully specified scenarios passes prefill check',
            ],
            architectureNotes: [
              'Implementation: Prefill validator allows transition when no placeholders found',
            ],
            attachments: ['spec/attachments/TEST-002/ast-research.json'],
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

      await writeJsonTestFile(
        join(setup.testDir, 'spec/work-units.json'),
        workUnitsData
      );

      // When I try to update work unit status to testing
      // Then the command should succeed (no prefill to block)
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-002',
        status: 'testing',
        cwd: setup.testDir,
      });

      expect(result.success).toBe(true);
    });
  });
});
