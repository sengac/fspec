/**
 * Feature: spec/features/discover-foundation-finalize-doesn-t-show-validation-errors.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { discoverFoundation } from '../discover-foundation';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';

describe('Feature: discover-foundation --finalize validation error feedback', () => {
  let testDir: string;
  let draftPath: string;

  beforeEach(async () => {
    // Create temp directory for tests
    testDir = join(process.cwd(), `test-temp-${Date.now()}`);
    draftPath = join(testDir, 'spec/foundation.json.draft');
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    // Clean up
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Show detailed error when personas array has placeholder data', () => {
    it('should show validation errors with missing personas fields and fix commands', async () => {
      // Given I have a foundation.json.draft with all basic fields filled
      // And the personas array is empty (no personas at all)
      const draft = {
        version: '2.0.0',
        project: {
          name: 'TestProject',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test problem',
            description: 'Test problem description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [],
        },
        personas: [],
      };

      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // When I run `fspec discover-foundation --finalize`
      const result = await discoverFoundation({
        finalize: true,
        draftPath,
        outputPath: join(testDir, 'spec/foundation.json'),
      });

      // Then the command should exit with code 1 (result.valid should be false)
      expect(result.valid).toBe(false);

      // And the output should contain "Foundation validation failed"
      // And the output should contain "Missing required"
      // And the output should show command to fix: "fspec add-persona"
      expect(result.validationErrors).toBeDefined();
      expect(result.validationErrors).toContain('Missing required');
      expect(result.validationErrors).toContain('fspec add-persona');
    });
  });

  describe('Scenario: Show detailed error when capabilities array is empty', () => {
    it('should show validation errors with missing capabilities and fix commands', async () => {
      // Given I have a foundation.json.draft with all basic fields filled
      // And the solutionSpace.capabilities array is empty
      const draft = {
        version: '2.0.0',
        project: {
          name: 'TestProject',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test problem',
            description: 'Test problem description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [], // Empty capabilities
        },
        personas: [],
      };

      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // When I run `fspec discover-foundation --finalize`
      const result = await discoverFoundation({
        finalize: true,
        draftPath,
        outputPath: join(testDir, 'spec/foundation.json'),
      });

      // Then the command should exit with code 1
      expect(result.valid).toBe(false);

      // And the output should contain "Foundation validation failed"
      // And the output should contain "Missing required"
      // And the output should show command to fix: "fspec add-capability"
      expect(result.validationErrors).toBeDefined();
      expect(result.validationErrors).toContain('Missing required');
      expect(result.validationErrors).toContain('fspec add-capability');
    });
  });

  describe('Scenario: Show all missing fields in one error message', () => {
    it('should list all validation errors and all fix commands', async () => {
      // Given I have a foundation.json.draft with multiple missing fields
      // And the personas array is empty
      // And the capabilities array is empty
      const draft = {
        version: '2.0.0',
        project: {
          name: 'TestProject',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test problem',
            description: 'Test problem description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [], // Empty
        },
        personas: [],
      };

      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // When I run `fspec discover-foundation --finalize`
      const result = await discoverFoundation({
        finalize: true,
        draftPath,
        outputPath: join(testDir, 'spec/foundation.json'),
      });

      // Then the command should exit with code 1
      expect(result.valid).toBe(false);

      // And the output should contain "Foundation validation failed"
      // And the output should list all missing fields
      // And the output should show all relevant fix commands
      expect(result.validationErrors).toBeDefined();
      expect(result.validationErrors).toContain('Missing required');
      expect(result.validationErrors).toContain('fspec add-capability');
      expect(result.validationErrors).toContain('fspec add-persona');
      expect(result.validationErrors).toContain('fspec update-foundation');
    });
  });
});
