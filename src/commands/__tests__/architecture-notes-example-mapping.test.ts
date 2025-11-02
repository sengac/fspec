/**
 * Feature: spec/features/architecture-notes-in-example-mapping.feature
 *
 * This test file validates that architecture notes can be captured during Example Mapping
 * and are properly integrated into the generate-scenarios workflow.
 *
 * Scenarios tested:
 * - Add architecture note during Example Mapping
 * - Generate scenarios populates docstring with architecture notes
 * - View architecture notes in work unit
 * - Remove architecture note from work unit
 * - Generated docstring organizes notes by category
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, readFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { addArchitectureNote } from '../add-architecture-note';
import { removeArchitectureNote } from '../remove-architecture-note';
import { generateScenarios } from '../generate-scenarios';
import type { WorkUnitsData } from '../../types';

describe('Feature: Architecture notes in Example Mapping', () => {
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    await mkdir(join(tmpDir, 'spec', 'features'), { recursive: true });
    await writeFile(join(tmpDir, 'spec', 'features', '.gitkeep'), '');
  });

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true });
  });

  describe('Scenario: Add architecture note during Example Mapping', () => {
    it('should add architecture note to work unit', async () => {
      // Given I have a work unit WORK-001 in specifying status
      const workUnitsData: WorkUnitsData = {
        meta: {
          lastId: 1,
          lastUpdated: new Date().toISOString(),
        },
        prefixes: {
          WORK: {
            name: 'Work',
            nextId: 2,
          },
        },
        workUnits: {
          'WORK-001': {
            id: 'WORK-001',
            prefix: 'WORK',
            title: 'Test Feature',
            description: 'Test',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            rules: [
              { id: 0, text: 'Rule 1', deleted: false, createdAt: new Date().toISOString() },
            ],
            examples: [
              { id: 0, text: 'Example 1', deleted: false, createdAt: new Date().toISOString() },
            ],
            questions: [],
            nextRuleId: 1,
            nextExampleId: 1,
            nextQuestionId: 0,
          },
        },
      };

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec add-architecture-note WORK-001 'Uses @cucumber/gherkin parser'"
      await addArchitectureNote({
        workUnitId: 'WORK-001',
        note: 'Uses @cucumber/gherkin parser',
        cwd: tmpDir,
      });

      // Then the architecture note should be added to the work unit
      const updatedData = JSON.parse(
        await readFile(join(tmpDir, 'spec', 'work-units.json'), 'utf-8')
      );

      // And when I run "fspec show-work-unit WORK-001"
      // Then I should see an "Architecture Notes:" section
      expect(updatedData.workUnits['WORK-001'].architectureNotes).toBeDefined();

      // And the section should contain "Uses @cucumber/gherkin parser"
      const activeNotes = updatedData.workUnits['WORK-001'].architectureNotes.filter(
        (n: any) => !n.deleted
      );
      expect(activeNotes).toHaveLength(1);
      expect(activeNotes[0].text).toBe('Uses @cucumber/gherkin parser');
    });
  });

  describe('Scenario: Generate scenarios populates docstring with architecture notes', () => {
    it('should populate docstring with captured architecture notes', async () => {
      // Given I have a work unit with architecture notes captured
      const workUnitsData: WorkUnitsData = {
        meta: { lastId: 1, lastUpdated: new Date().toISOString() },
        prefixes: { WORK: { name: 'Work', nextId: 2 } },
        workUnits: {
          'WORK-001': {
            id: 'WORK-001',
            prefix: 'WORK',
            title: 'Test Feature',
            description: 'Test',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            rules: [
              { id: 0, text: 'Rule', deleted: false, createdAt: new Date().toISOString() },
            ],
            examples: [
              { id: 0, text: 'Example', deleted: false, createdAt: new Date().toISOString() },
            ],
            questions: [],
            nextRuleId: 1,
            nextExampleId: 1,
            nextQuestionId: 0,
            // And the work unit has the note "Uses @cucumber/gherkin parser"
            // And the work unit has the note "Must complete validation within 2 seconds"
            architectureNotes: [
              { id: 0, text: 'Uses @cucumber/gherkin parser', deleted: false, createdAt: new Date().toISOString() },
              { id: 1, text: 'Must complete validation within 2 seconds', deleted: false, createdAt: new Date().toISOString() },
            ],
            nextNoteId: 2,
          },
        },
      };

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec generate-scenarios WORK-001"
      const result = await generateScenarios({
        workUnitId: 'WORK-001',
        cwd: tmpDir,
      });

      const content = await readFile(result.featureFile, 'utf-8');

      // Then the generated feature file should have a docstring
      expect(content).toContain('"""');

      // And the docstring should contain "Uses @cucumber/gherkin parser"
      expect(content).toContain('Uses @cucumber/gherkin parser');

      // And the docstring should contain "Must complete validation within 2 seconds"
      expect(content).toContain('Must complete validation within 2 seconds');

      // And the docstring should NOT contain placeholder text
      expect(content).not.toContain('TODO: Add key architectural decisions');
    });
  });

  describe('Scenario: View architecture notes in work unit', () => {
    it('should display architecture notes in order with indices', async () => {
      // Given I have added architecture notes to WORK-001
      const workUnitsData: WorkUnitsData = {
        meta: { lastId: 1, lastUpdated: new Date().toISOString() },
        prefixes: { WORK: { name: 'Work', nextId: 2 } },
        workUnits: {
          'WORK-001': {
            id: 'WORK-001',
            prefix: 'WORK',
            title: 'Test Feature',
            description: 'Test',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            rules: ['Rule 1'],
            examples: ['Example 1'],
            questions: [],
            architectureNotes: [
              { id: 0, text: 'Uses @cucumber/gherkin parser', deleted: false, createdAt: new Date().toISOString() },
              { id: 1, text: 'Must complete within 2 seconds', deleted: false, createdAt: new Date().toISOString() },
              { id: 2, text: 'Validates all Gherkin keywords', deleted: false, createdAt: new Date().toISOString() },
            ],
            nextNoteId: 3,
          },
        },
      };

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec show-work-unit WORK-001"
      const { showWorkUnit } = await import('../show-work-unit');
      const result = await showWorkUnit({
        workUnitId: 'WORK-001',
        cwd: tmpDir,
      });

      // Then I should see an "Architecture Notes:" section
      expect(result.architectureNotes).toBeDefined();

      // And the section should list all captured notes with indices
      expect(result.architectureNotes).toHaveLength(3);

      // And the notes should be displayed in the order they were added with IDs
      expect(result.architectureNotes).toEqual([
        '[0] Uses @cucumber/gherkin parser',
        '[1] Must complete within 2 seconds',
        '[2] Validates all Gherkin keywords',
      ]);
    });
  });

  describe('Scenario: Remove architecture note from work unit', () => {
    it('should remove architecture note at specified index', async () => {
      // Given I have a work unit with 3 architecture notes
      const workUnitsData: WorkUnitsData = {
        meta: { lastId: 1, lastUpdated: new Date().toISOString() },
        prefixes: { WORK: { name: 'Work', nextId: 2 } },
        workUnits: {
          'WORK-001': {
            id: 'WORK-001',
            prefix: 'WORK',
            title: 'Test',
            description: 'Test',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            rules: [
              { id: 0, text: 'Rule', deleted: false, createdAt: new Date().toISOString() },
            ],
            examples: [
              { id: 0, text: 'Example', deleted: false, createdAt: new Date().toISOString() },
            ],
            questions: [],
            nextRuleId: 1,
            nextExampleId: 1,
            nextQuestionId: 0,
            architectureNotes: [
              { id: 0, text: 'Note 1', deleted: false, createdAt: new Date().toISOString() },
              { id: 1, text: 'Note 2', deleted: false, createdAt: new Date().toISOString() },
              { id: 2, text: 'Note 3', deleted: false, createdAt: new Date().toISOString() },
            ],
            nextNoteId: 3,
          },
        },
      };

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec remove-architecture-note WORK-001 1"
      await removeArchitectureNote({
        workUnitId: 'WORK-001',
        index: 1,
        cwd: tmpDir,
      });

      // Then the architecture note at index 1 should be removed
      const updatedData = JSON.parse(
        await readFile(join(tmpDir, 'spec', 'work-units.json'), 'utf-8')
      );

      // And when I run "fspec show-work-unit WORK-001"
      // Then I should see 2 remaining architecture notes (non-deleted)
      const activeNotes = updatedData.workUnits['WORK-001'].architectureNotes.filter(
        (n: any) => !n.deleted
      );
      expect(activeNotes.length).toBe(2);
      expect(activeNotes.map((n: any) => n.text)).toEqual([
        'Note 1',
        'Note 3',
      ]);
    });
  });

  describe('Scenario: Generated docstring organizes notes by category', () => {
    it('should group architecture notes by detected prefix', async () => {
      // Given I have architecture notes with natural prefixes
      const workUnitsData: WorkUnitsData = {
        meta: { lastId: 1, lastUpdated: new Date().toISOString() },
        prefixes: { WORK: { name: 'Work', nextId: 2 } },
        workUnits: {
          'WORK-001': {
            id: 'WORK-001',
            prefix: 'WORK',
            title: 'Test Feature',
            description: 'Test',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            rules: [
              { id: 0, text: 'Rule', deleted: false, createdAt: new Date().toISOString() },
            ],
            examples: [
              { id: 0, text: 'Example', deleted: false, createdAt: new Date().toISOString() },
            ],
            questions: [],
            nextRuleId: 1,
            nextExampleId: 1,
            nextQuestionId: 0,
            // And I have note "Dependency: @cucumber/gherkin parser"
            // And I have note "Performance: Must complete within 2 seconds"
            // And I have note "Refactoring: Share validation logic with formatter"
            architectureNotes: [
              { id: 0, text: 'Dependency: @cucumber/gherkin parser', deleted: false, createdAt: new Date().toISOString() },
              { id: 1, text: 'Performance: Must complete within 2 seconds', deleted: false, createdAt: new Date().toISOString() },
              { id: 2, text: 'Refactoring: Share validation logic with formatter', deleted: false, createdAt: new Date().toISOString() },
            ],
            nextNoteId: 3,
          },
        },
      };

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec generate-scenarios WORK-001"
      const result = await generateScenarios({
        workUnitId: 'WORK-001',
        cwd: tmpDir,
      });

      const content = await readFile(result.featureFile, 'utf-8');

      // Then the docstring should group notes by detected prefix
      expect(content).toContain('"""');

      // And dependency notes should appear first
      expect(content).toContain('Dependency:');
      expect(content).toContain('- @cucumber/gherkin parser');

      // And performance notes should appear in their own section
      expect(content).toContain('Performance:');
      expect(content).toContain('- Must complete within 2 seconds');
    });
  });
});
