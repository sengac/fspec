/**
 * Feature: spec/features/scenario-title-cleaning.feature
 * Scenario: Clean scenario titles by removing common prefixes
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtempSync, rmSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';
import { mkdir, writeFile, readFile } from 'fs/promises';
import type { WorkUnitsData } from '../../types';

describe('Feature: Scenario Title Cleaning', () => {
  let tempDir: string;
  let specDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    // Given I have a project with spec directory
    tempDir = mkdtempSync(join(tmpdir(), 'fspec-test-'));
    specDir = join(tempDir, 'spec');
    await mkdir(specDir, { recursive: true });
    workUnitsFile = join(specDir, 'work-units.json');
  });

  afterEach(() => {
    rmSync(tempDir, { recursive: true, force: true });
  });

  describe('Scenario: Remove common prefixes from scenario titles', () => {
    it('should clean REPRODUCTION: prefix from scenario title', async () => {
      // Given a work unit with example "REPRODUCTION: file has 42 scenarios"
      const workUnitsData: WorkUnitsData = {
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            title: 'Test Feature',
            description: 'Test',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            userStory: {
              role: 'developer',
              action: 'test feature',
              benefit: 'verify functionality',
            },
            examples: [
              'REPRODUCTION: work-unit-dependency-management.feature has 42 scenarios but coverage has 39',
            ],
          },
        },
        epics: {},
        prefixes: {},
      };

      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // When I run generate-scenarios
      const { generateScenarios } = await import('../generate-scenarios.js');
      const result = await generateScenarios({
        workUnitId: 'TEST-001',
        cwd: tempDir,
      });

      // Then the generated scenario title should NOT include "REPRODUCTION:"
      const featureContent = await readFile(result.featureFile, 'utf-8');

      // Should have clean title
      expect(featureContent).toContain(
        'Scenario: work-unit-dependency-management.feature has 42 scenarios but coverage has 39'
      );

      // Should NOT have prefix in title
      expect(featureContent).not.toMatch(/Scenario: REPRODUCTION:/);

      // Should preserve original as comment
      expect(featureContent).toContain(
        '# Example: REPRODUCTION: work-unit-dependency-management.feature has 42 scenarios but coverage has 39'
      );
    });

    it('should clean all common prefixes from scenario titles', async () => {
      // Given work unit with examples using various prefixes
      const workUnitsData: WorkUnitsData = {
        workUnits: {
          'TEST-002': {
            id: 'TEST-002',
            title: 'Test Prefixes',
            description: 'Test all prefixes',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            userStory: {
              role: 'developer',
              action: 'clean titles',
              benefit: 'readable scenarios',
            },
            examples: [
              'MISSING: Scenario X (line 501-518) NOT in coverage file',
              'ERROR WHEN LINKING: fspec link-coverage fails with Scenario not found',
              'COMMAND RESULT: fspec generate-coverage runs without error',
              'FILE: spec/features/work-unit.feature - grep shows 42 scenarios',
              'EXACT LINE: Feature file line 501: @COV-046 tag missing from coverage',
              'COMMAND TO REPRODUCE: 1) Add scenario 2) Run command 3) Check file',
            ],
          },
        },
        epics: {},
        prefixes: {},
      };

      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // When I run generate-scenarios
      const { generateScenarios } = await import('../generate-scenarios.js');
      const result = await generateScenarios({
        workUnitId: 'TEST-002',
        cwd: tempDir,
      });

      const featureContent = await readFile(result.featureFile, 'utf-8');

      // Then all scenario titles should be cleaned
      expect(featureContent).toContain('Scenario: Scenario X (line 501-518) NOT in coverage file');
      expect(featureContent).toContain('Scenario: fspec link-coverage fails with Scenario not found');
      expect(featureContent).toContain('Scenario: fspec generate-coverage runs without error');
      expect(featureContent).toContain('Scenario: spec/features/work-unit.feature - grep shows 42 scenarios');
      expect(featureContent).toContain('Scenario: Feature file line 501: @COV-046 tag missing from coverage');
      expect(featureContent).toContain('Scenario: 1) Add scenario 2) Run command 3) Check file');

      // And should NOT contain any prefixes in titles
      expect(featureContent).not.toMatch(/Scenario: MISSING:/);
      expect(featureContent).not.toMatch(/Scenario: ERROR WHEN LINKING:/);
      expect(featureContent).not.toMatch(/Scenario: COMMAND RESULT:/);
      expect(featureContent).not.toMatch(/Scenario: FILE:/);
      expect(featureContent).not.toMatch(/Scenario: EXACT LINE:/);
      expect(featureContent).not.toMatch(/Scenario: COMMAND TO REPRODUCE:/);

      // But should preserve all original examples as comments
      expect(featureContent).toContain('# Example: MISSING:');
      expect(featureContent).toContain('# Example: ERROR WHEN LINKING:');
      expect(featureContent).toContain('# Example: COMMAND RESULT:');
      expect(featureContent).toContain('# Example: FILE:');
      expect(featureContent).toContain('# Example: EXACT LINE:');
      expect(featureContent).toContain('# Example: COMMAND TO REPRODUCE:');
    });
  });
});
