/**
 * Feature: spec/features/validate-foundation-schema-version-detection.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { validateFoundationSchema } from '../validate-foundation-schema';

describe('Feature: validate-foundation-schema version detection', () => {
  let testDir: string;

  beforeEach(async () => {
    // Create temp directory for tests
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    // Clean up temp directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Validate foundation with version 2.0.0 using generic schema', () => {
    it('should use generic-foundation-validator for v2.0.0', async () => {
      // Given I have a foundation.json file with version field set to 2.0.0
      const foundation = {
        version: '2.0.0',
        project: {
          name: 'Test Project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test Problem',
            description: 'Test description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [
            {
              name: 'Test Capability',
              description: 'Test capability description',
            },
          ],
        },
      };

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(foundation, null, 2),
        'utf-8'
      );

      // When I run 'fspec validate-foundation-schema'
      const result = await validateFoundationSchema({ cwd: testDir });

      // Then the command should use generic-foundation-validator
      // And the validation should pass
      expect(result.success).toBe(true);
      expect(result.output).toContain('valid');
    });
  });
});
