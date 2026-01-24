/**
 * Feature: spec/features/template-persona-with-placeholders-remains-in-finalized-foundation-json.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { discoverFoundation } from '../discover-foundation';
import { mkdir, writeFile, rm, access } from 'fs/promises';
import { join } from 'path';

import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';
describe('Feature: Template persona with placeholders remains in finalized foundation.json', () => {
  let testDir: string;
  let draftPath: string;
  let finalPath: string;

  beforeEach(async () => {
    // Create temp directory for tests
    testDir = await createTempTestDir(
      'discover-foundation-placeholder-detection'
    );
    draftPath = join(testDir, 'spec/foundation.json.draft');
    finalPath = join(testDir, 'spec/foundation.json');
  });

  afterEach(async () => {
    // Clean up
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Reject finalization when template persona with placeholders remains in draft', () => {
    it('should fail with exit code 1 and NOT create foundation.json', async () => {
      // Given I have a foundation.json.draft with all basic fields filled
      // And the draft has a template persona with "[QUESTION: Who uses this?]" placeholders
      // And I have added real personas using add-persona command
      // And I have added capabilities using add-capability command
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
          capabilities: [
            {
              name: 'Capability',
              description: 'A real capability',
            },
          ],
        },
        personas: [
          // Template persona with placeholders (should cause failure)
          {
            name: '[QUESTION: Who uses this?]',
            description: '[QUESTION: Who uses this?]',
            goals: ['[QUESTION: What are their goals?]'],
          },
          // Real persona added by user
          {
            name: 'AI Developer Agent',
            description: 'An AI agent that writes code',
            goals: ['Automate development tasks'],
          },
        ],
      };

      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // When I run "fspec discover-foundation --finalize"
      const result = await discoverFoundation({
        finalize: true,
        draftPath,
        outputPath: finalPath,
      });

      // Then the command should fail with exit code 1
      expect(result.valid).toBe(false);

      // And the output should contain "Cannot finalize: draft still has unfilled placeholder fields"
      expect(result.validationErrors).toBeDefined();
      expect(result.validationErrors).toContain('Cannot finalize');
      expect(result.validationErrors).toContain('unfilled placeholder fields');

      // And the output should indicate which field contains placeholders
      expect(result.validationErrors).toContain('personas');

      // And the foundation.json file should NOT be created
      await expect(access(finalPath)).rejects.toThrow();

      // And the foundation.json.draft file should remain unchanged
      await expect(access(draftPath)).resolves.not.toThrow();
    });
  });

  describe('Scenario: Detect placeholders in personas array during finalization', () => {
    it('should detect placeholders and set allFieldsComplete to false', async () => {
      // Given I have a foundation.json.draft with mixed personas
      // And the personas array contains a template persona with "[QUESTION:]" markers
      // And the personas array contains a valid persona without placeholders
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
          capabilities: [
            {
              name: 'Test Capability',
              description: 'A test capability',
            },
          ],
        },
        personas: [
          // Template persona with [QUESTION:] markers
          {
            name: '[QUESTION: Who?]',
            description: '[QUESTION: Who?]',
            goals: ['[QUESTION: What?]'],
          },
          // Valid persona without placeholders
          {
            name: 'Real Person',
            description: 'A real user',
            goals: ['Achieve things'],
          },
        ],
      };

      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // @step When the scanDraftForNextField function processes the draft
      // (This happens internally during finalize call)
      // @step Then allFieldsComplete should be set to false
      // @step And nextField should indicate "personas"
      // When I attempt to finalize the draft
      const result = await discoverFoundation({
        finalize: true,
        draftPath,
        outputPath: finalPath,
      });

      // Then finalization should fail with a clear error message
      expect(result.valid).toBe(false);

      // And the error should list the specific fields with placeholders
      expect(result.validationErrors).toBeDefined();
      expect(result.validationErrors).toContain('personas');
      expect(result.validationErrors).toContain('[QUESTION:');
    });
  });
});
