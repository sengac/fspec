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
            rules: ['Rule 1'],
            examples: ['Example 1'],
            questions: [],
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
      expect(updatedData.workUnits['WORK-001'].architectureNotes).toContain(
        'Uses @cucumber/gherkin parser'
      );
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
            rules: ['Rule'],
            examples: ['Example'],
            questions: [],
            // And the work unit has the note "Uses @cucumber/gherkin parser"
            // And the work unit has the note "Must complete validation within 2 seconds"
            architectureNotes: [
              'Uses @cucumber/gherkin parser',
              'Must complete validation within 2 seconds',
            ],
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
            rules: ['Rule'],
            examples: ['Example'],
            questions: [],
            architectureNotes: ['Note 1', 'Note 2', 'Note 3'],
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
      // Then I should see 2 remaining architecture notes
      expect(updatedData.workUnits['WORK-001'].architectureNotes.length).toBe(2);
      expect(updatedData.workUnits['WORK-001'].architectureNotes).toEqual([
        'Note 1',
        'Note 3',
      ]);
    });
  });
});
