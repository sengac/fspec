/**
 * Feature: spec/features/auto-replace-placeholder-personas-and-capabilities-when-adding-first-item.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, readFile, rm } from 'fs/promises';
import path from 'path';
import { addPersona } from '../add-persona';
import { addCapability } from '../add-capability';
import type { GenericFoundation } from '../../types/foundation';

import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';
describe('Feature: Auto-replace placeholder personas and capabilities when adding first item', () => {
  let testDir: string;
  let foundationPath: string;
  let draftPath: string;

  beforeEach(async () => {
    // Create temp directory for tests
    testDir = await createTempTestDir(
      'auto-replace-placeholder-personas-and-capabilities-when-adding-first-item'
    );
    foundationPath = path.join(testDir, 'spec/foundation.json');
    draftPath = path.join(testDir, 'spec/foundation.json.draft');
    await mkdir(path.join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    // Clean up
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Add first persona removes placeholder persona', () => {
    it('should remove placeholder persona and add new real persona', async () => {
      // @step Given a foundation file with placeholder persona "[QUESTION: Who uses this?]"
      const foundation: GenericFoundation = {
        version: '2.0.0',
        project: {
          name: 'Test Project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test problem',
            description: 'Test description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [],
        },
        personas: [
          {
            name: '[QUESTION: Who uses this?]',
            description: '[QUESTION: Who uses this?]',
            goals: ['[QUESTION: What are their goals?]'],
          },
        ],
      };
      await writeFile(foundationPath, JSON.stringify(foundation, null, 2));

      // Mock console.log to capture output
      const logs: string[] = [];
      const originalLog = console.log;
      console.log = (...args: unknown[]) => {
        logs.push(args.map(arg => String(arg)).join(' '));
      };

      // @step When I run "fspec add-persona \"Developer\" \"Writes code\""
      await addPersona(testDir, 'Developer', 'Writes code', ['Build software']);

      // Restore console.log
      console.log = originalLog;

      // @step Then the placeholder persona should be removed
      const updatedContent = await readFile(foundationPath, 'utf-8');
      const updated: GenericFoundation = JSON.parse(updatedContent);
      const hasPlaceholder = updated.personas.some(
        p =>
          p.name.includes('[QUESTION:') || p.description.includes('[QUESTION:')
      );
      expect(hasPlaceholder).toBe(false);

      // @step And the foundation should contain persona "Developer" with description "Writes code"
      const developer = updated.personas.find(p => p.name === 'Developer');
      expect(developer).toBeDefined();
      expect(developer?.description).toBe('Writes code');
      expect(developer?.goals).toContain('Build software');

      // @step And the output should show "Removed 1 placeholder persona(s)"
      const output = logs.join('\n');
      expect(output).toContain('Removed 1 placeholder persona(s)');
    });
  });

  describe('Scenario: Add first capability removes placeholder capability', () => {
    it('should remove placeholder capability and add new real capability', async () => {
      // @step Given a foundation file with placeholder capability "[QUESTION: What can users DO?]"
      const foundation: GenericFoundation = {
        version: '2.0.0',
        project: {
          name: 'Test Project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test problem',
            description: 'Test description',
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
        personas: [],
      };
      await writeFile(foundationPath, JSON.stringify(foundation, null, 2));

      // Mock console.log to capture output
      const logs: string[] = [];
      const originalLog = console.log;
      console.log = (...args: unknown[]) => {
        logs.push(args.map(arg => String(arg)).join(' '));
      };

      // @step When I run "fspec add-capability \"Spec Validation\" \"Validates Gherkin\""
      await addCapability(testDir, 'Spec Validation', 'Validates Gherkin');

      // Restore console.log
      console.log = originalLog;

      // @step Then the placeholder capability should be removed
      const updatedContent = await readFile(foundationPath, 'utf-8');
      const updated: GenericFoundation = JSON.parse(updatedContent);
      const hasPlaceholder = updated.solutionSpace.capabilities.some(
        c =>
          c.name.includes('[QUESTION:') || c.description.includes('[QUESTION:')
      );
      expect(hasPlaceholder).toBe(false);

      // @step And the foundation should contain capability "Spec Validation" with description "Validates Gherkin"
      const capability = updated.solutionSpace.capabilities.find(
        c => c.name === 'Spec Validation'
      );
      expect(capability).toBeDefined();
      expect(capability?.description).toBe('Validates Gherkin');

      // @step And the output should show "Removed 1 placeholder capability(ies)"
      const output = logs.join('\n');
      expect(output).toContain('Removed 1 placeholder capability(ies)');
    });
  });

  describe('Scenario: Add subsequent persona does not remove placeholders', () => {
    it('should not remove placeholders when adding subsequent persona', async () => {
      // @step Given a foundation file with real persona "Developer"
      const foundation: GenericFoundation = {
        version: '2.0.0',
        project: {
          name: 'Test Project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test problem',
            description: 'Test description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [],
        },
        personas: [
          {
            name: 'Developer',
            description: 'Writes code',
            goals: ['Build software'],
          },
        ],
      };
      await writeFile(foundationPath, JSON.stringify(foundation, null, 2));

      // Mock console.log to capture output
      const logs: string[] = [];
      const originalLog = console.log;
      console.log = (...args: unknown[]) => {
        logs.push(args.map(arg => String(arg)).join(' '));
      };

      // @step When I run "fspec add-persona \"QA Engineer\" \"Tests features\""
      await addPersona(testDir, 'QA Engineer', 'Tests features', [
        'Ensure quality',
      ]);

      // Restore console.log
      console.log = originalLog;

      // @step Then no placeholders should be removed
      // (No placeholder removal message in output)

      // @step And the foundation should contain both personas "Developer" and "QA Engineer"
      const updatedContent = await readFile(foundationPath, 'utf-8');
      const updated: GenericFoundation = JSON.parse(updatedContent);
      expect(updated.personas).toHaveLength(2);
      expect(updated.personas.find(p => p.name === 'Developer')).toBeDefined();
      expect(
        updated.personas.find(p => p.name === 'QA Engineer')
      ).toBeDefined();

      // @step And the output should NOT show placeholder removal message
      const output = logs.join('\n');
      expect(output).not.toContain('Removed');
      expect(output).not.toContain('placeholder');
    });
  });

  describe('Scenario: Multiple placeholder personas are all removed', () => {
    it('should remove all placeholder personas when adding first real persona', async () => {
      // @step Given a foundation file with 3 placeholder personas
      const foundation: GenericFoundation = {
        version: '2.0.0',
        project: {
          name: 'Test Project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test problem',
            description: 'Test description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [],
        },
        personas: [
          {
            name: '[QUESTION: Who uses this?]',
            description: '[QUESTION: Who uses this?]',
            goals: ['[QUESTION: Goals?]'],
          },
          {
            name: '[DETECTED: User]',
            description: 'Placeholder',
            goals: ['Placeholder goal'],
          },
          {
            name: 'Placeholder Name',
            description: '[QUESTION: What do they do?]',
            goals: ['Goal'],
          },
        ],
      };
      await writeFile(foundationPath, JSON.stringify(foundation, null, 2));

      // Mock console.log to capture output
      const logs: string[] = [];
      const originalLog = console.log;
      console.log = (...args: unknown[]) => {
        logs.push(args.map(arg => String(arg)).join(' '));
      };

      // @step When I run "fspec add-persona \"Developer\" \"First real persona\""
      await addPersona(testDir, 'Developer', 'First real persona', [
        'Build features',
      ]);

      // Restore console.log
      console.log = originalLog;

      // @step Then all 3 placeholder personas should be removed
      const updatedContent = await readFile(foundationPath, 'utf-8');
      const updated: GenericFoundation = JSON.parse(updatedContent);
      const hasAnyPlaceholder = updated.personas.some(
        p =>
          p.name.includes('[QUESTION:') ||
          p.name.includes('[DETECTED:') ||
          p.description.includes('[QUESTION:') ||
          p.description.includes('[DETECTED:')
      );
      expect(hasAnyPlaceholder).toBe(false);

      // @step And the foundation should contain only persona "Developer"
      expect(updated.personas).toHaveLength(1);
      expect(updated.personas[0].name).toBe('Developer');
      expect(updated.personas[0].description).toBe('First real persona');

      // @step And the output should show "Removed 3 placeholder persona(s)"
      const output = logs.join('\n');
      expect(output).toContain('Removed 3 placeholder persona(s)');
    });
  });

  describe('Scenario: Mixed real and placeholder personas - no removal on subsequent add', () => {
    it('should not remove placeholders when foundation has real personas already', async () => {
      // @step Given a foundation file with real persona "Developer" and placeholder persona "[QUESTION: ...]"
      const foundation: GenericFoundation = {
        version: '2.0.0',
        project: {
          name: 'Test Project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test problem',
            description: 'Test description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [],
        },
        personas: [
          {
            name: 'Developer',
            description: 'Writes code',
            goals: ['Build software'],
          },
          {
            name: '[QUESTION: Who else uses this?]',
            description: 'Placeholder',
            goals: ['Placeholder goal'],
          },
        ],
      };
      await writeFile(foundationPath, JSON.stringify(foundation, null, 2));

      // Mock console.log to capture output
      const logs: string[] = [];
      const originalLog = console.log;
      console.log = (...args: unknown[]) => {
        logs.push(args.map(arg => String(arg)).join(' '));
      };

      // @step When I run "fspec add-persona \"QA Engineer\" \"Tests features\""
      await addPersona(testDir, 'QA Engineer', 'Tests features', [
        'Ensure quality',
      ]);

      // Restore console.log
      console.log = originalLog;

      // @step Then no placeholders should be removed
      const updatedContent = await readFile(foundationPath, 'utf-8');
      const updated: GenericFoundation = JSON.parse(updatedContent);
      const placeholderStillExists = updated.personas.some(p =>
        p.name.includes('[QUESTION:')
      );
      expect(placeholderStillExists).toBe(true);

      // @step And the foundation should contain personas "Developer", "QA Engineer", and the placeholder
      expect(updated.personas).toHaveLength(3);
      expect(updated.personas.find(p => p.name === 'Developer')).toBeDefined();
      expect(
        updated.personas.find(p => p.name === 'QA Engineer')
      ).toBeDefined();
      expect(
        updated.personas.find(p => p.name.includes('[QUESTION:'))
      ).toBeDefined();

      // @step And the output should NOT show placeholder removal message
      const output = logs.join('\n');
      expect(output).not.toContain('Removed');
      expect(output).not.toContain('placeholder');
    });
  });

  describe('Scenario: Auto-remove works with draft file', () => {
    it('should remove placeholder from draft file', async () => {
      // @step Given a draft file "foundation.json.draft" with placeholder persona "[QUESTION: Who uses this?]"
      const foundation: GenericFoundation = {
        version: '2.0.0',
        project: {
          name: 'Test Project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test problem',
            description: 'Test description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [],
        },
        personas: [
          {
            name: '[QUESTION: Who uses this?]',
            description: '[QUESTION: Who uses this?]',
            goals: ['[QUESTION: Goals?]'],
          },
        ],
      };
      await writeFile(draftPath, JSON.stringify(foundation, null, 2));

      // Mock console.log to capture output
      const logs: string[] = [];
      const originalLog = console.log;
      console.log = (...args: unknown[]) => {
        logs.push(args.map(arg => String(arg)).join(' '));
      };

      // @step When I run "fspec add-persona \"Developer\" \"Writes code\" --draft-path foundation.json.draft"
      // Note: The command uses draft file by default when it exists
      await addPersona(testDir, 'Developer', 'Writes code', ['Build software']);

      // Restore console.log
      console.log = originalLog;

      // @step Then the placeholder persona should be removed from the draft
      const updatedContent = await readFile(draftPath, 'utf-8');
      const updated: GenericFoundation = JSON.parse(updatedContent);
      const hasPlaceholder = updated.personas.some(p =>
        p.name.includes('[QUESTION:')
      );
      expect(hasPlaceholder).toBe(false);

      // @step And the draft should contain persona "Developer"
      const developer = updated.personas.find(p => p.name === 'Developer');
      expect(developer).toBeDefined();

      // @step And the output should show "Removed 1 placeholder persona(s)"
      const output = logs.join('\n');
      expect(output).toContain('Removed 1 placeholder persona(s)');
    });
  });

  describe('Scenario: Auto-remove works with capability in draft file', () => {
    it('should remove placeholder capability from draft file', async () => {
      // @step Given a draft file "foundation.json.draft" with placeholder capability "[DETECTED: cli-tool]"
      const foundation: GenericFoundation = {
        version: '2.0.0',
        project: {
          name: 'Test Project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test problem',
            description: 'Test description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [
            {
              name: '[DETECTED: CLI execution]',
              description: 'Placeholder capability',
            },
          ],
        },
        personas: [],
      };
      await writeFile(draftPath, JSON.stringify(foundation, null, 2));

      // Mock console.log to capture output
      const logs: string[] = [];
      const originalLog = console.log;
      console.log = (...args: unknown[]) => {
        logs.push(args.map(arg => String(arg)).join(' '));
      };

      // @step When I run "fspec add-capability \"Command Execution\" \"Runs CLI commands\" --draft-path foundation.json.draft"
      // Note: The command uses draft file by default when it exists
      await addCapability(testDir, 'Command Execution', 'Runs CLI commands');

      // Restore console.log
      console.log = originalLog;

      // @step Then the placeholder capability should be removed from the draft
      const updatedContent = await readFile(draftPath, 'utf-8');
      const updated: GenericFoundation = JSON.parse(updatedContent);
      const hasPlaceholder = updated.solutionSpace.capabilities.some(c =>
        c.name.includes('[DETECTED:')
      );
      expect(hasPlaceholder).toBe(false);

      // @step And the draft should contain capability "Command Execution"
      const capability = updated.solutionSpace.capabilities.find(
        c => c.name === 'Command Execution'
      );
      expect(capability).toBeDefined();

      // @step And the output should show "Removed 1 placeholder capability(ies)"
      const output = logs.join('\n');
      expect(output).toContain('Removed 1 placeholder capability(ies)');
    });
  });
});
