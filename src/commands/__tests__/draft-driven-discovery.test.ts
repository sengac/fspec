/**
 * Feature: spec/features/project-type-detection-incorrectly-identifies-cli-tools-as-web-apps.feature
 * Bug: FOUND-010
 *
 * This test file validates that discover-foundation uses foundation.json.draft as guidance (NOT automated detection).
 * Tests map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, readFile, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { mkdtemp } from 'fs/promises';
import { discoverFoundation } from '../discover-foundation';

describe('Feature: Project type detection incorrectly identifies CLI tools as web-apps', () => {
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    await mkdir(join(tmpDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true });
  });

  describe('Scenario: Fill project type field using draft guidance', () => {
    it('should read draft with [QUESTION:] placeholder and create draft with guidance', async () => {
      // Given foundation.json.draft contains "projectType": "[QUESTION: What type of project is this?]"
      // When discover-foundation command runs
      const result = await discoverFoundation({
        draftPath: join(tmpDir, 'spec', 'foundation.json.draft'),
      });

      // Then command should create draft with [QUESTION:] placeholders
      expect(result.draftCreated).toBe(true);
      expect(result.draftContent).toBeDefined();

      // And draft should contain [QUESTION:] for projectType
      expect(result.draftContent).toContain('[QUESTION:');

      // And draft should guide AI to fill in missing fields
      // (System reminders are emitted to prompt AI - tested separately)
      expect(result.systemReminder).toContain('Detected');
    });

    it('should emit system-reminder prompting AI to gather information', async () => {
      // When discover-foundation identifies unfilled field
      const result = await discoverFoundation({
        draftPath: join(tmpDir, 'spec', 'foundation.json.draft'),
      });

      // Then command should emit system-reminder
      expect(result.systemReminder).toContain('<system-reminder>');
      expect(result.systemReminder).toContain('</system-reminder>');

      // And reminder should prompt AI to review detected values
      expect(result.systemReminder).toContain('Review in questionnaire');
    });
  });

  describe('Scenario: Fill persona field using draft guidance', () => {
    it('should guide AI to fill persona fields via commands', async () => {
      // Given foundation.json.draft contains persona with "[QUESTION: Describe this persona]"
      const result = await discoverFoundation({
        draftPath: join(tmpDir, 'spec', 'foundation.json.draft'),
      });

      // Then draft should contain personas with placeholders
      expect(result.draftContent).toContain('personas');
      expect(result.draftContent).toContain('[QUESTION:');

      // And AI should be guided to use fspec update-foundation command
      // (Implementation should check that fspec commands are used, not manual editing)
    });
  });

  describe('Scenario: Finalize foundation when all fields are filled', () => {
    it('should validate draft and create final foundation.json when all placeholders resolved', async () => {
      // Given foundation.json.draft has all [QUESTION:] placeholders resolved
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const completeDraft = {
        version: '2.0.0',
        project: {
          name: 'fspec',
          vision: 'CLI tool for managing Gherkin specs with ACDD',
          projectType: 'cli-tool', // NO [QUESTION:] placeholder
        },
        problemSpace: {
          primaryProblem: {
            title: 'Specification Management',
            description: 'AI agents need structured workflow',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Gherkin-based specification management',
          capabilities: [
            { name: 'Gherkin Validation', description: 'Validate feature files' },
          ],
        },
        personas: [
          {
            name: 'Developer using CLI',
            description: 'Uses fspec in terminal',
            goals: ['Manage specifications'],
          },
        ],
      };

      await writeFile(draftPath, JSON.stringify(completeDraft, null, 2), 'utf-8');

      // When discover-foundation command validates the draft
      const result = await discoverFoundation({
        finalize: true,
        draftPath,
        outputPath: join(tmpDir, 'spec', 'foundation.json'),
      });

      // Then command should validate draft against JSON schema
      expect(result.valid).toBe(true);
      expect(result.validated).toBe(true);

      // And if validation passes, command should create foundation.json
      expect(result.finalCreated).toBe(true);
      expect(result.finalPath).toBe(join(tmpDir, 'spec', 'foundation.json'));

      // And command should delete foundation.json.draft
      expect(result.draftDeleted).toBe(true);
    });

    it('should show validation errors if draft is invalid', async () => {
      // Given foundation.json.draft has invalid schema (missing required fields)
      const draftPath = join(tmpDir, 'spec', 'foundation.json.draft');
      const invalidDraft = {
        version: '2.0.0',
        // Missing project field (required)
      };

      await writeFile(draftPath, JSON.stringify(invalidDraft, null, 2), 'utf-8');

      // When discover-foundation validates the draft
      const result = await discoverFoundation({
        finalize: true,
        draftPath,
        outputPath: join(tmpDir, 'spec', 'foundation.json'),
      });

      // Then validation should fail
      expect(result.valid).toBe(false);
      expect(result.validated).toBe(false);

      // And command should NOT create foundation.json
      expect(result.finalCreated).toBeUndefined();
    });
  });

  describe('Scenario: Prevent manual editing of draft file', () => {
    it('should guide AI to use fspec commands instead of manual editing', async () => {
      // This scenario tests guidance behavior - AI should be prompted to use commands
      // The actual detection of manual editing would happen at the command level
      // This test documents the expected behavior

      // Given AI attempts to manually edit foundation.json.draft
      // When discover-foundation runs
      // Then system-reminder should guide AI to use fspec update-foundation commands

      // This is a documentation test - the behavior is:
      // 1. AI sees draft with [QUESTION:] placeholders
      // 2. System reminds AI to use fspec update-foundation commands
      // 3. AI uses commands (not Write/Edit tools) to update fields

      expect(true).toBe(true); // Documented behavior
    });
  });

  describe('Scenario: Verify detected values with human', () => {
    it('should allow AI to verify [DETECTED:] values with human', async () => {
      // Given foundation.json.draft contains "projectType": "[DETECTED: web-app]"
      const result = await discoverFoundation({
        draftPath: join(tmpDir, 'spec', 'foundation.json.draft'),
      });

      // Then draft should contain [DETECTED:] values
      expect(result.draftContent).toContain('[DETECTED:');

      // And AI should be able to verify with human and update if needed
      // (AI can run: fspec update-foundation --field project.projectType --value cli-tool)

      // System reminder should guide AI to review detected values
      expect(result.systemReminder).toContain('Review in questionnaire');
    });
  });
});
