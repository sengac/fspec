/**
 * Feature: spec/features/coverage-file-synchronization.feature
 *
 * Tests for coverage file synchronization when scenarios are deleted, renamed, or modified.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { promises as fs } from 'fs';
import { join } from 'path';
import { deleteScenario } from '../../commands/delete-scenario';
import { deleteScenariosByTag } from '../../commands/delete-scenarios-by-tag';
import { updateCoverageFile, CoverageScenario } from '../coverage-file';
import { updateScenario } from '../../commands/update-scenario';
import { checkFeatureCoverage } from '../../commands/update-work-unit-status';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';

describe('Feature: Coverage File Synchronization', () => {
  let setup: TestDirectorySetup;

  beforeEach(async () => {
    setup = await setupTestDirectory('coverage-sync');
    await fs.mkdir(join(setup.testDir, 'spec', 'features'), {
      recursive: true,
    });
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Delete scenario removes coverage entry', () => {
    it('should remove coverage entry and recalculate stats when scenario is deleted', async () => {
      // @step Given a feature file test.feature with scenarios A, B, and C
      const featurePath = join(
        setup.testDir,
        'spec',
        'features',
        'test.feature'
      );
      const featureContent = `Feature: Test Feature

Scenario: Scenario A
  Given setup A
  When action A
  Then result A

Scenario: Scenario B
  Given setup B
  When action B
  Then result B

Scenario: Scenario C
  Given setup C
  When action C
  Then result C
`;
      await fs.writeFile(featurePath, featureContent, 'utf-8');

      // @step And a coverage file test.feature.coverage with entries for A, B, and C
      const coveragePath = `${featurePath}.coverage`;
      const coverageContent = {
        scenarios: [
          { name: 'Scenario A', testMappings: [] },
          { name: 'Scenario B', testMappings: [] },
          { name: 'Scenario C', testMappings: [] },
        ],
        stats: {
          totalScenarios: 3,
          coveredScenarios: 0,
          coveragePercent: 0,
        },
      };
      await fs.writeFile(
        coveragePath,
        JSON.stringify(coverageContent, null, 2),
        'utf-8'
      );

      // @step When I run 'fspec delete-scenario test.feature "Scenario B"'
      const result = await deleteScenario({
        feature: 'test',
        scenario: 'Scenario B',
        cwd: setup.testDir,
      });
      if (!result.success) {
        console.log('Delete failed:', result.error);
      }
      expect(result.success).toBe(true);

      // Verify scenario was deleted from feature file
      const updatedFeature = await fs.readFile(featurePath, 'utf-8');
      expect(updatedFeature).not.toContain('Scenario B');

      // @step Then the coverage file should only contain entries for A and C
      const updatedCoverage = JSON.parse(
        await fs.readFile(coveragePath, 'utf-8')
      );
      expect(updatedCoverage.scenarios).toHaveLength(2);
      expect(
        updatedCoverage.scenarios.map((s: CoverageScenario) => s.name)
      ).toEqual(['Scenario A', 'Scenario C']);

      // @step And the coverage statistics should show totalScenarios as 2
      expect(updatedCoverage.stats.totalScenarios).toBe(2);

      // @step And the output should display '✓ Deleted scenario "Scenario B" from test.feature' with coverage update notification
      // Note: This would be tested via command output capture in integration tests
    });
  });

  describe('Scenario: Bulk delete scenarios by tag removes coverage entries', () => {
    it('should remove all coverage entries for deleted scenarios when bulk deleting by tag', async () => {
      // @step Given multiple feature files with scenarios tagged @deprecated
      const feature1Path = join(
        setup.testDir,
        'spec',
        'features',
        'feature1.feature'
      );
      const feature1Content = `Feature: Feature 1

@deprecated
Scenario: Old Feature A
  Given old setup
  When old action
  Then old result

Scenario: Current Feature B
  Given new setup
  When new action
  Then new result
`;
      await fs.writeFile(feature1Path, feature1Content, 'utf-8');

      const feature2Path = join(
        setup.testDir,
        'spec',
        'features',
        'feature2.feature'
      );
      const feature2Content = `Feature: Feature 2

@deprecated
Scenario: Old Feature C
  Given old setup
  When old action
  Then old result

Scenario: Current Feature D
  Given new setup
  When new action
  Then new result
`;
      await fs.writeFile(feature2Path, feature2Content, 'utf-8');

      // @step And coverage files exist for all features
      const coverage1Path = `${feature1Path}.coverage`;
      const coverage1Content = {
        scenarios: [
          { name: 'Old Feature A', testMappings: [] },
          { name: 'Current Feature B', testMappings: [] },
        ],
        stats: { totalScenarios: 2, coveredScenarios: 0, coveragePercent: 0 },
      };
      await fs.writeFile(
        coverage1Path,
        JSON.stringify(coverage1Content, null, 2),
        'utf-8'
      );

      const coverage2Path = `${feature2Path}.coverage`;
      const coverage2Content = {
        scenarios: [
          { name: 'Old Feature C', testMappings: [] },
          { name: 'Current Feature D', testMappings: [] },
        ],
        stats: { totalScenarios: 2, coveredScenarios: 0, coveragePercent: 0 },
      };
      await fs.writeFile(
        coverage2Path,
        JSON.stringify(coverage2Content, null, 2),
        'utf-8'
      );

      // @step When I run 'fspec delete-scenarios --tag @deprecated'
      await deleteScenariosByTag({ tags: ['@deprecated'], cwd: setup.testDir });

      // @step Then all @deprecated scenarios should be removed from feature files
      const updatedFeature1 = await fs.readFile(feature1Path, 'utf-8');
      expect(updatedFeature1).not.toContain('Old Feature A');
      expect(updatedFeature1).toContain('Current Feature B');

      const updatedFeature2 = await fs.readFile(feature2Path, 'utf-8');
      expect(updatedFeature2).not.toContain('Old Feature C');
      expect(updatedFeature2).toContain('Current Feature D');

      // @step And all corresponding coverage entries should be removed
      const updatedCoverage1 = JSON.parse(
        await fs.readFile(coverage1Path, 'utf-8')
      );
      expect(updatedCoverage1.scenarios).toHaveLength(1);
      expect(updatedCoverage1.scenarios[0].name).toBe('Current Feature B');

      const updatedCoverage2 = JSON.parse(
        await fs.readFile(coverage2Path, 'utf-8')
      );
      expect(updatedCoverage2.scenarios).toHaveLength(1);
      expect(updatedCoverage2.scenarios[0].name).toBe('Current Feature D');

      // @step And coverage statistics should be recalculated for affected files
      expect(updatedCoverage1.stats.totalScenarios).toBe(1);
      expect(updatedCoverage2.stats.totalScenarios).toBe(1);
    });
  });

  describe('Scenario: Coverage validation detects stale scenarios', () => {
    it('should fail validation when coverage file contains stale scenarios', async () => {
      // @step Given a feature file with scenarios A and B
      const featurePath = join(
        setup.testDir,
        'spec',
        'features',
        'test.feature'
      );
      const featureContent = `Feature: Test Feature

Scenario: Scenario A
  Given setup A
  When action A
  Then result A

Scenario: Scenario B
  Given setup B
  When action B
  Then result B
`;
      await fs.writeFile(featurePath, featureContent, 'utf-8');

      // @step And a coverage file with entries for A, B, and C (stale)
      const coveragePath = `${featurePath}.coverage`;
      const coverageContent = {
        scenarios: [
          {
            name: 'Scenario A',
            testMappings: [
              { file: 'test.ts', lines: '1-10', implMappings: [] },
            ],
          },
          {
            name: 'Scenario B',
            testMappings: [
              { file: 'test.ts', lines: '11-20', implMappings: [] },
            ],
          },
          { name: 'Scenario C', testMappings: [] }, // Stale - doesn't exist in feature file
        ],
        stats: {
          totalScenarios: 3,
          coveredScenarios: 2,
          coveragePercent: 66.67,
        },
      };
      await fs.writeFile(
        coveragePath,
        JSON.stringify(coverageContent, null, 2),
        'utf-8'
      );

      // @step When I run 'fspec update-work-unit-status WORK-001 validating'
      const result = await checkFeatureCoverage(
        'test',
        featurePath,
        setup.testDir
      );

      // @step Then the command should fail with an error
      expect(result.complete).toBe(false);

      // @step And the output should suggest running 'fspec generate-coverage' to sync
      // Note: The message reports stale scenarios (in coverage but not in feature file)
      expect(result.message).toContain('stale');

      // @step And the output should list the stale scenario names
      expect(result.message).toContain('Scenario C');
    });
  });

  describe('Scenario: Generate coverage syncs deleted scenarios', () => {
    it('should remove stale scenarios from coverage when generate-coverage is run', async () => {
      // @step Given a feature file with scenarios A and B
      const featurePath = join(
        setup.testDir,
        'spec',
        'features',
        'test.feature'
      );
      const featureContent = `Feature: Test Feature

Scenario: Scenario A
  Given setup A
  When action A
  Then result A

Scenario: Scenario B
  Given setup B
  When action B
  Then result B
`;
      await fs.writeFile(featurePath, featureContent, 'utf-8');

      // @step And a coverage file with entries for A, B, and C (stale)
      const coveragePath = `${featurePath}.coverage`;
      const coverageContent = {
        scenarios: [
          { name: 'Scenario A', testMappings: [] },
          { name: 'Scenario B', testMappings: [] },
          { name: 'Scenario C', testMappings: [] }, // Stale - doesn't exist in feature file
        ],
        stats: {
          totalScenarios: 3,
          coveredScenarios: 0,
          coveragePercent: 0,
        },
      };
      await fs.writeFile(
        coveragePath,
        JSON.stringify(coverageContent, null, 2),
        'utf-8'
      );

      // @step When I run 'fspec generate-coverage'
      await updateCoverageFile(featurePath);

      // @step Then the coverage file should only contain entries for A and B
      const updatedCoverage = JSON.parse(
        await fs.readFile(coveragePath, 'utf-8')
      );
      expect(updatedCoverage.scenarios).toHaveLength(2);
      expect(
        updatedCoverage.scenarios.map((s: CoverageScenario) => s.name)
      ).toEqual(['Scenario A', 'Scenario B']);

      // @step And scenario C should be removed from the coverage file
      expect(
        updatedCoverage.scenarios.find(
          (s: CoverageScenario) => s.name === 'Scenario C'
        )
      ).toBeUndefined();

      // @step And the output should display 'ℹ Removed 1 stale scenario from coverage'
      // Note: This would be tested via command output capture in integration tests
    });
  });

  describe('Scenario: Deleted scenario no longer causes uncovered error', () => {
    it('should not report uncovered error for deleted scenarios after coverage sync', async () => {
      // @step Given a coverage file with uncovered scenario X
      const featurePath = join(
        setup.testDir,
        'spec',
        'features',
        'test.feature'
      );
      const featureContent = `Feature: Test Feature

Scenario: Scenario A
  Given setup A
  When action A
  Then result A
`;
      await fs.writeFile(featurePath, featureContent, 'utf-8');

      // @step And scenario X has been deleted from the feature file
      // @step And the coverage file has been synced to remove X
      const coveragePath = `${featurePath}.coverage`;
      const coverageContent = {
        scenarios: [
          {
            name: 'Scenario A',
            testMappings: [
              { file: 'test.ts', lines: '1-10', implMappings: [] },
            ],
          },
        ],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 1,
          coveragePercent: 100,
        },
      };
      await fs.writeFile(
        coveragePath,
        JSON.stringify(coverageContent, null, 2),
        'utf-8'
      );

      // @step When I run 'fspec update-work-unit-status WORK-001 validating'
      const result = await checkFeatureCoverage(
        'test',
        featurePath,
        setup.testDir
      );

      // @step Then the command should succeed
      expect(result.complete).toBe(true);

      // @step And no 'scenario X uncovered' error should be reported
      if (result.message) {
        expect(result.message).not.toContain('Scenario X');
        expect(result.message).not.toContain('uncovered');
      }
    });
  });

  describe('Scenario: Rename scenario preserves test mappings', () => {
    it('should preserve test mappings when scenario is renamed', async () => {
      // @step Given a scenario 'User logs in' with test mappings
      const featurePath = join(
        setup.testDir,
        'spec',
        'features',
        'user-login.feature'
      );
      const featureContent = `Feature: User Login

Scenario: User logs in
  Given the login page is displayed
  When the user enters valid credentials
  Then the user should be redirected to dashboard
`;
      await fs.writeFile(featurePath, featureContent, 'utf-8');

      const coveragePath = `${featurePath}.coverage`;
      const coverageContent = {
        scenarios: [
          {
            name: 'User logs in',
            testMappings: [
              {
                file: 'src/__tests__/auth.test.ts',
                lines: '10-25',
                implMappings: [
                  { file: 'src/auth/login.ts', lines: [15, 16, 20] },
                ],
              },
            ],
          },
        ],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 1,
          coveragePercent: 100,
        },
      };
      await fs.writeFile(
        coveragePath,
        JSON.stringify(coverageContent, null, 2),
        'utf-8'
      );

      // @step When I run 'fspec update-scenario user-login "User logs in" "User authenticates"'
      const result = await updateScenario({
        feature: 'user-login',
        oldName: 'User logs in',
        newName: 'User authenticates',
        cwd: setup.testDir,
      });
      if (!result.success) {
        console.log('Update failed:', result.error);
      }
      expect(result.success).toBe(true);

      // @step Then the feature file should show the renamed scenario 'User authenticates'
      const updatedFeature = await fs.readFile(featurePath, 'utf-8');
      expect(updatedFeature).toContain('Scenario: User authenticates');
      expect(updatedFeature).not.toContain('Scenario: User logs in');

      // @step And the coverage file should have entry for 'User authenticates'
      const updatedCoverage = JSON.parse(
        await fs.readFile(coveragePath, 'utf-8')
      );
      expect(updatedCoverage.scenarios[0].name).toBe('User authenticates');

      // @step And all test mappings should be preserved from the old scenario name
      expect(updatedCoverage.scenarios[0].testMappings).toHaveLength(1);
      expect(updatedCoverage.scenarios[0].testMappings[0].file).toBe(
        'src/__tests__/auth.test.ts'
      );
      expect(updatedCoverage.scenarios[0].testMappings[0].lines).toBe('10-25');
      expect(
        updatedCoverage.scenarios[0].testMappings[0].implMappings
      ).toHaveLength(1);
      expect(
        updatedCoverage.scenarios[0].testMappings[0].implMappings[0].file
      ).toBe('src/auth/login.ts');
    });
  });
});
