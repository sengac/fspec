/**
 * Feature: spec/features/generate-scenarios-include-architecture-docstring.feature
 *
 * This test file validates that generate-scenarios includes architecture docstrings.
 * Bug fix: BUG-011 - generate-scenarios was missing architecture docstrings.
 *
 * Scenarios tested:
 * - Generated feature file includes architecture docstring
 * - Generated file structure matches create-feature template
 * - Existing tests continue to pass with docstring addition
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, readFile, mkdir } from 'fs/promises';
import { join } from 'path';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import { writeJsonTestFile } from '../../test-helpers/test-file-operations';
import { generateScenarios } from '../generate-scenarios';
import type { WorkUnitsData } from '../../types';

describe('Feature: generate-scenarios include architecture docstring', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('generate-scenarios-docstring');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Generated feature file includes architecture docstring', () => {
    it('should include docstring with architecture notes before example mapping comments', async () => {
      // Given I have a work unit with example mapping data
      // And the work unit has rules and examples
      const workUnitsData: WorkUnitsData = {
        meta: {
          lastId: 1,
          lastUpdated: new Date().toISOString(),
        },
        prefixes: {
          TEST: {
            name: 'Test',
            nextId: 2,
          },
        },
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            prefix: 'TEST',
            title: 'Test Feature',
            description: 'Test description',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            userStory: {
              role: 'developer',
              action: 'test feature',
              benefit: 'ensure quality',
            },
            rules: ['Rule 1', 'Rule 2'],
            examples: ['Example 1', 'Example 2'],
            questions: [],
          },
        },
      };

      await writeJsonTestFile(setup.workUnitsFile, workUnitsData);

      // When I run "fspec generate-scenarios <work-unit-id>"
      const result = await generateScenarios({
        workUnitId: 'TEST-001',
        cwd: setup.testDir,
      });

      // Then a feature file should be created
      expect(result.success).toBe(true);
      expect(result.featureFile).toBeDefined();

      const content = await readFile(result.featureFile, 'utf-8');

      // And the file should contain a docstring with architecture notes placeholder
      expect(content).toContain('"""');
      expect(content).toContain('Architecture notes:');
      expect(content).toContain('TODO: Add key architectural decisions');

      // And the file should contain example mapping comments
      expect(content).toContain('# EXAMPLE MAPPING CONTEXT');

      // And the docstring should come before the example mapping comments
      const docstringIndex = content.indexOf('"""');
      const commentIndex = content.indexOf('# EXAMPLE MAPPING CONTEXT');
      expect(docstringIndex).toBeLessThan(commentIndex);
    });
  });

  describe('Scenario: Generated file structure matches create-feature template', () => {
    it('should match the structure: tags → feature → docstring → comments → background', async () => {
      // Given I run "fspec generate-scenarios" on a work unit
      const workUnitsData: WorkUnitsData = {
        meta: { lastId: 1, lastUpdated: new Date().toISOString() },
        prefixes: { TEST: { name: 'Test', nextId: 2 } },
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            prefix: 'TEST',
            title: 'Test',
            description: 'Test',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            userStory: { role: 'user', action: 'test', benefit: 'quality' },
            rules: ['Rule'],
            examples: ['Example'],
            questions: [],
          },
        },
      };

      await writeJsonTestFile(setup.workUnitsFile, workUnitsData);

      const result = await generateScenarios({
        workUnitId: 'TEST-001',
        cwd: setup.testDir,
      });

      // When I examine the generated feature file
      const content = await readFile(result.featureFile, 'utf-8');
      const lines = content.split('\n').filter(line => line.trim());

      // Then the file structure should be: tags → Feature → docstring → comments → Background
      // Find indices of key sections
      const featureLineIndex = lines.findIndex(line =>
        line.startsWith('Feature:')
      );
      const docstringStartIndex = lines.findIndex(line => line.includes('"""'));
      const docstringEndIndex = lines.findIndex(
        (line, i) => i > docstringStartIndex && line.includes('"""')
      );
      const commentIndex = lines.findIndex(line =>
        line.includes('# EXAMPLE MAPPING CONTEXT')
      );
      const backgroundIndex = lines.findIndex(line =>
        line.includes('Background:')
      );

      // Verify ordering
      expect(featureLineIndex).toBeGreaterThan(-1);
      expect(docstringStartIndex).toBeGreaterThan(featureLineIndex);
      expect(docstringEndIndex).toBeGreaterThan(docstringStartIndex);
      expect(commentIndex).toBeGreaterThan(docstringEndIndex);
      expect(backgroundIndex).toBeGreaterThan(commentIndex);

      // And the docstring should contain "Architecture notes:"
      expect(content).toContain('Architecture notes:');

      // And the docstring should contain TODO placeholders
      expect(content).toContain('TODO:');

      // And the structure should match files created with "fspec create-feature"
      expect(content).toMatch(
        /"""[\s\S]*?Architecture notes:[\s\S]*?TODO:[\s\S]*?"""/
      );
    });
  });

  describe('Scenario: Existing tests continue to pass with docstring addition', () => {
    it('should validate docstring presence and correct ordering', async () => {
      // Given I modify generate-scenarios to include docstrings
      // (This is already done - the feature is implemented)

      // When I run the existing test suite (simulated by running generate-scenarios)
      const workUnitsData: WorkUnitsData = {
        meta: { lastId: 1, lastUpdated: new Date().toISOString() },
        prefixes: { TEST: { name: 'Test', nextId: 2 } },
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            prefix: 'TEST',
            title: 'Regression Test',
            description: 'Test',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            userStory: { role: 'user', action: 'test', benefit: 'quality' },
            rules: ['Rule 1'],
            examples: ['Example 1'],
            questions: [],
          },
        },
      };

      await writeJsonTestFile(setup.workUnitsFile, workUnitsData);

      const result = await generateScenarios({
        workUnitId: 'TEST-001',
        cwd: setup.testDir,
      });

      const content = await readFile(result.featureFile, 'utf-8');

      // Then all generate-scenarios tests should pass
      expect(result.success).toBe(true);

      // And tests should validate docstring presence
      expect(content).toContain('"""');
      expect(content).toContain('Architecture notes:');

      // And tests should validate correct ordering (docstring before comments)
      const docstringIndex = content.indexOf('"""');
      const commentIndex = content.indexOf('# EXAMPLE MAPPING CONTEXT');
      expect(docstringIndex).toBeLessThan(commentIndex);
      expect(docstringIndex).toBeGreaterThan(-1);
      expect(commentIndex).toBeGreaterThan(-1);
    });
  });
});
