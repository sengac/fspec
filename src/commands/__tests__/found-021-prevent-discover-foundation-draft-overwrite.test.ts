/**
 * Feature: spec/features/discover-foundation-overwrites-draft-without-warning.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Tests ensure discover-foundation command does not overwrite existing draft without --force flag.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { discoverFoundation } from '../discover-foundation';
import { mkdir, writeFile, rm, access, readFile } from 'fs/promises';
import { join } from 'path';

describe('Feature: discover-foundation overwrites draft without warning', () => {
  let testDir: string;
  let draftPath: string;
  let finalPath: string;

  beforeEach(async () => {
    // Create temp directory for tests
    testDir = join(process.cwd(), `test-temp-${Date.now()}`);
    draftPath = join(testDir, 'spec/foundation.json.draft');
    finalPath = join(testDir, 'spec/foundation.json');
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    // Clean up
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Prevent overwriting existing draft without --force flag', () => {
    it('should fail with exit code 1 and preserve existing draft', async () => {
      // @step Given a foundation.json.draft file exists
      const existingDraft = {
        version: '2.0.0',
        project: {
          name: 'MyProject',
          vision: 'My vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test Problem',
            description: 'Test Description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test Solution',
          capabilities: [],
        },
        personas: [],
      };
      await writeFile(
        draftPath,
        JSON.stringify(existingDraft, null, 2),
        'utf-8'
      );

      // @step When I run "fspec discover-foundation" without the --force flag
      const result = await discoverFoundation({
        draftPath,
        cwd: testDir,
      });

      // @step Then the command should fail with exit code 1
      expect(result.valid).toBe(false);

      // @step And the output should contain a system-reminder error message
      expect(result.systemReminder).toBeDefined();
      expect(result.systemReminder).toContain('<system-reminder>');
      expect(result.systemReminder).toContain('</system-reminder>');

      // @step And the error message should suggest using the --force flag
      expect(result.systemReminder).toContain('--force');

      // @step And the existing draft file should remain unchanged
      const draftContent = await readFile(draftPath, 'utf-8');
      const draftData = JSON.parse(draftContent);
      expect(draftData.project.name).toBe('MyProject');
      expect(draftData.project.vision).toBe('My vision');
    });
  });

  describe('Scenario: Allow overwriting draft with --force flag', () => {
    it('should succeed and overwrite the existing draft with warning', async () => {
      // @step Given a foundation.json.draft file exists
      const existingDraft = {
        version: '2.0.0',
        project: {
          name: 'OldProject',
          vision: 'Old vision',
          projectType: 'web-app',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Old Problem',
            description: 'Old Description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Old Solution',
          capabilities: [],
        },
        personas: [],
      };
      await writeFile(
        draftPath,
        JSON.stringify(existingDraft, null, 2),
        'utf-8'
      );

      // @step When I run "fspec discover-foundation --force"
      const result = await discoverFoundation({
        draftPath,
        cwd: testDir,
        force: true,
      });

      // @step Then the command should succeed
      expect(result.valid).toBe(true);
      expect(result.draftCreated).toBe(true);

      // @step And the draft file should be overwritten with a new template
      const draftContent = await readFile(draftPath, 'utf-8');
      const draftData = JSON.parse(draftContent);
      expect(draftData.project.name).toContain('[QUESTION:');
      expect(draftData.project.vision).toContain('[QUESTION:');

      // @step And the output should show a warning that the draft was overwritten
      expect(result.systemReminder).toBeDefined();
      expect(result.systemReminder).toContain('overwritten');
    });
  });

  describe('Scenario: Prevent draft creation when foundation.json exists', () => {
    it('should fail when foundation.json exists without --force flag', async () => {
      // @step Given a foundation.json file already exists
      const existingFoundation = {
        version: '2.0.0',
        project: {
          name: 'ExistingProject',
          vision: 'Existing vision',
          projectType: 'library',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Existing Problem',
            description: 'Existing Description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Existing Solution',
          capabilities: [],
        },
        personas: [],
      };
      await writeFile(
        finalPath,
        JSON.stringify(existingFoundation, null, 2),
        'utf-8'
      );

      // @step And no foundation.json.draft file exists
      // (draft doesn't exist yet)

      // @step When I run "fspec discover-foundation" without the --force flag
      const result = await discoverFoundation({
        draftPath,
        outputPath: 'spec/foundation.json', // Relative path, will be joined with cwd
        cwd: testDir,
      });

      // @step Then the command should fail with exit code 1
      expect(result.valid).toBe(false);

      // @step And the output should contain a system-reminder error message
      expect(result.systemReminder).toBeDefined();
      expect(result.systemReminder).toContain('<system-reminder>');

      // @step And the error message should explain that foundation already exists
      expect(result.systemReminder).toContain('foundation.json already exists');

      // @step And the error message should suggest using --force to regenerate
      expect(result.systemReminder).toContain('--force');
      expect(result.systemReminder).toContain('REGENERATE');
    });
  });

  describe('Scenario: Create draft when neither draft nor foundation exists', () => {
    it('should create draft successfully when no files exist', async () => {
      // @step Given no foundation.json.draft file exists
      // (draft doesn't exist in fresh test dir)

      // @step And no foundation.json file exists
      // (foundation doesn't exist in fresh test dir)

      // @step When I run "fspec discover-foundation"
      const result = await discoverFoundation({
        draftPath,
        cwd: testDir,
      });

      // @step Then the command should succeed
      expect(result.valid).toBe(true);
      expect(result.draftCreated).toBe(true);

      // @step And a foundation.json.draft file should be created
      await expect(access(draftPath)).resolves.not.toThrow();

      // @step And the output should show field-by-field guidance
      expect(result.systemReminder).toBeDefined();
      expect(result.systemReminder).toContain('Field 1/');
    });
  });

  describe('Scenario: Finalize flow unchanged when draft exists', () => {
    it('should execute normal finalization flow with --finalize flag', async () => {
      // @step Given a foundation.json.draft file exists with complete fields
      const completeDraft = {
        version: '2.0.0',
        project: {
          name: 'CompleteProject',
          vision: 'Complete vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Complete Problem',
            description: 'Complete Description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Complete Solution',
          capabilities: [
            {
              name: 'TestCapability',
              description: 'Test Description',
            },
          ],
        },
        personas: [
          {
            name: 'TestPersona',
            description: 'Test Persona Description',
            goals: ['Test Goal'],
          },
        ],
      };
      await writeFile(
        draftPath,
        JSON.stringify(completeDraft, null, 2),
        'utf-8'
      );

      // @step When I run "fspec discover-foundation --finalize"
      const result = await discoverFoundation({
        finalize: true,
        draftPath,
        outputPath: finalPath,
        cwd: testDir,
      });

      // @step Then the normal finalization flow should execute
      expect(result.valid).toBe(true);
      expect(result.validated).toBe(true);

      // @step And the draft should be validated and converted to foundation.json
      expect(result.finalCreated).toBe(true);
      await expect(access(finalPath)).resolves.not.toThrow();

      // @step And the draft file should be deleted after successful finalization
      expect(result.draftDeleted).toBe(true);
      await expect(access(draftPath)).rejects.toThrow();
    });
  });
});
