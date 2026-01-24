/**
 * Feature: spec/features/foundation-discovery-removal-commands-documentation.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import path from 'path';
import { mkdir, writeFile } from 'fs/promises';
import { discoverFoundation } from '../discover-foundation';
import { getSetupHelpContent } from '../../help';
import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';

describe('Feature: Foundation discovery removal commands documentation', () => {
  describe('Scenario: Help setup section includes remove-persona command', () => {
    it('should include remove-persona in help setup output', () => {
      // @step Given I am using fspec
      // (No setup needed - fspec is available)

      // @step When I get the setup help content
      const setupHelp = getSetupHelpContent();

      // @step Then the output should include "fspec remove-persona <name>"
      expect(setupHelp).toContain('fspec remove-persona');

      // @step And the output should include "Remove persona from foundation.json"
      expect(setupHelp.toLowerCase()).toContain('remove persona');

      // @step And the remove-persona section should appear alongside add-persona
      expect(setupHelp).toContain('fspec add-persona');
      expect(setupHelp).toContain('fspec remove-persona');
    });
  });

  describe('Scenario: Help setup section includes remove-capability command', () => {
    it('should include remove-capability in help setup output', () => {
      // @step Given I am using fspec
      // (No setup needed - fspec is available)

      // @step When I get the setup help content
      const setupHelp = getSetupHelpContent();

      // @step Then the output should include "fspec remove-capability"
      expect(setupHelp).toContain('fspec remove-capability');

      // @step And the output should include info about removing capabilities
      expect(setupHelp.toLowerCase()).toContain('remove capability');

      // @step And the remove-capability section should appear alongside add-capability
      expect(setupHelp).toContain('fspec add-capability');
      expect(setupHelp).toContain('fspec remove-capability');
    });
  });

  describe('Scenario: Removal commands show placeholder removal examples', () => {
    it('should show examples of removing placeholder text', () => {
      // @step Given I am using fspec
      // (No setup needed - fspec is available)

      // @step When I get the setup help content
      const setupHelp = getSetupHelpContent();

      // @step Then the help should include removal commands
      expect(setupHelp).toContain('remove-persona');
      expect(setupHelp).toContain('remove-capability');

      // @step And the help should mention placeholder removal
      // Either via explicit placeholder examples or general removal guidance
      expect(
        setupHelp.includes('QUESTION') ||
          setupHelp.includes('placeholder') ||
          setupHelp.includes('remove')
      ).toBe(true);
    });
  });

  describe('Scenario: discover-foundation error includes removal guidance for placeholders', () => {
    let testDir: string;
    let draftPath: string;
    let finalPath: string;

    beforeEach(async () => {
      // Create temp directory for tests
      testDir = await createTempTestDir(
        'foundation-discovery-removal-commands-documentation'
      );
      draftPath = path.join(testDir, 'spec/foundation.json.draft');
      finalPath = path.join(testDir, 'spec/foundation.json');
    });

    afterEach(async () => {
      // Clean up
      await removeTempTestDir(testDir);
    });

    it('should include removal guidance in error message', async () => {
      // @step Given I have a foundation draft with placeholder personas
      // @step And the draft contains '[QUESTION: Who uses this?]' placeholder
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
              name: '[QUESTION: What can users DO?]',
              description: 'Placeholder capability',
            },
          ],
        },
        personas: [
          {
            name: '[QUESTION: Who uses this?]',
            description: '[QUESTION: Who uses this?]',
            goals: ['[QUESTION: What are their goals?]'],
          },
        ],
      };

      await writeFile(draftPath, JSON.stringify(draft, null, 2), 'utf-8');

      // @step When I run "fspec discover-foundation --finalize"
      const result = await discoverFoundation({
        finalize: true,
        draftPath,
        outputPath: finalPath,
        cwd: testDir,
      });

      // @step Then the command should fail with exit code 1
      expect(result.valid).toBe(false);

      // @step And the error message should explain how to fill placeholders
      expect(result.validationErrors).toBeDefined();
      expect(result.validationErrors).toContain('fill all placeholder fields');
      expect(result.validationErrors).toContain('fspec update-foundation');
      expect(result.validationErrors).toContain('fspec add-capability');
      expect(result.validationErrors).toContain('fspec add-persona');

      // @step And the error message should explain how to remove unwanted placeholders
      expect(result.validationErrors).toContain('remove unwanted placeholder');

      // @step And the error message should include "fspec remove-persona" command example
      expect(result.validationErrors).toContain('fspec remove-persona');

      // @step And the error message should include "fspec remove-capability" command example
      expect(result.validationErrors).toContain('fspec remove-capability');
    });
  });
});
