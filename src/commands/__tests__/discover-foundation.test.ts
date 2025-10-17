/**
 * Feature: spec/features/implement-discover-foundation-command.feature
 *
 * This test file validates the acceptance criteria for discover-foundation command.
 * Tests map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import {
  discoverFoundation,
  analyzeCodebase,
  emitDiscoveryReminder,
} from '../discover-foundation';

describe('Feature: Implement discover-foundation Command', () => {
  describe('Scenario: Emit system-reminder after code analysis detects personas', () => {
    it('should emit system-reminder with detected personas list', async () => {
      // Given I run discover-foundation command
      // And code analysis detects 3 personas: End User, Admin, API Consumer
      const discoveryResult = analyzeCodebase();
      expect(discoveryResult.personas).toHaveLength(3);

      // When code analysis completes
      const systemReminder = emitDiscoveryReminder(discoveryResult);

      // Then command should emit system-reminder with detected personas
      expect(systemReminder).toContain('<system-reminder>');
      expect(systemReminder).toContain('</system-reminder>');

      // And system-reminder should list all 3 detected personas
      expect(systemReminder).toContain('End User');
      expect(systemReminder).toContain('Admin');
      expect(systemReminder).toContain('API Consumer');

      // And system-reminder should guide AI to review in questionnaire
      expect(systemReminder).toContain('Review in questionnaire');
    });
  });

  describe('Scenario: Generate validated foundation.json after questionnaire', () => {
    it('should create validated foundation.json with questionnaire answers', async () => {
      // Given I complete the questionnaire with all required answers
      // (Simulated by creating a draft file manually)
      const { writeFile, mkdir } = await import('fs/promises');
      const draftPath = 'spec/foundation.json.draft';
      const editedDraft = {
        version: '2.0.0',
        project: {
          name: 'fspec',
          vision: 'CLI tool for managing Gherkin specs with ACDD',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Specification Management',
            description: 'AI agents need structured workflow for specifications',
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

      await mkdir('spec', { recursive: true });
      await writeFile(draftPath, JSON.stringify(editedDraft, null, 2), 'utf-8');

      // When discover-foundation finishes (in finalize mode)
      const result = await discoverFoundation({ finalize: true, draftPath });

      // Then foundation.json should be created
      expect(result.foundation).toBeDefined();

      // And foundation.json should pass schema validation
      expect(result.valid).toBe(true);

      // And foundation.json should contain questionnaire answers
      expect(result.foundation.project.vision).toBeDefined();
      expect(result.foundation.problemSpace.primaryProblem).toBeDefined();
      expect(result.foundation.solutionSpace.capabilities).toBeDefined();
      expect(result.foundation.personas).toBeDefined();
    });
  });

  describe('Scenario: Create draft foundation with detected values and question placeholders', () => {
    it('should create foundation.json.draft with detected values and placeholders', async () => {
      // Given I have an existing fspec codebase with commander.js commands
      const discoveryResult = analyzeCodebase();

      // When I run 'fspec discover-foundation'
      const result = await discoverFoundation();

      // Then a file 'spec/foundation.json.draft' should be created
      expect(result.draftPath).toBe('spec/foundation.json.draft');
      expect(result.draftCreated).toBe(true);

      // And the draft should contain '[DETECTED: web-app]' for project type
      expect(result.draftContent).toContain('[DETECTED: web-app]');

      // And the draft should contain '[QUESTION: What is the core vision?]' placeholders
      expect(result.draftContent).toContain('[QUESTION:');
      expect(result.draftContent).toContain('core vision');
    });
  });

  describe('Scenario: Finalize foundation after AI edits draft', () => {
    it('should validate draft and create final foundation.json', async () => {
      // Given I have edited spec/foundation.json.draft with answers
      const { writeFile, mkdir } = await import('fs/promises');
      const draftPath = 'spec/foundation.json.draft';
      const editedDraft = {
        version: '2.0.0',
        project: {
          name: 'fspec',
          vision: 'CLI tool for managing Gherkin specs with ACDD',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Specification Management',
            description: 'AI agents need structured workflow for specifications',
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

      // Create spec directory and write draft file
      await mkdir('spec', { recursive: true });
      await writeFile(draftPath, JSON.stringify(editedDraft, null, 2), 'utf-8');

      // When I run 'fspec discover-foundation --finalize'
      const result = await discoverFoundation({
        finalize: true,
        draftPath,
      });

      // Then the command should validate the draft file
      expect(result.validated).toBe(true);

      // And create 'spec/foundation.json' with validated content
      expect(result.finalPath).toBe('spec/foundation.json');
      expect(result.finalCreated).toBe(true);

      // And delete 'spec/foundation.json.draft'
      expect(result.draftDeleted).toBe(true);
    });
  });
});
