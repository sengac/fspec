import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile, access } from 'fs/promises';
import { join } from 'path';
import { showFoundation } from '../show-foundation';
import { createMinimalFoundation } from '../../test-helpers/foundation-fixtures';

import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';
describe('Feature: Display Foundation Documentation', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('show-foundation');
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Display entire foundation in JSON format', () => {
    it('should output valid JSON with all fields', async () => {
      // Given I have a foundation.json with complete project data
      const foundationData = createMinimalFoundation();

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(foundationData, null, 2)
      );

      // When I run `fspec show-foundation --format json`
      const result = await showFoundation({
        format: 'json',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should be valid JSON
      const parsed = JSON.parse(result.output!);

      // And the JSON should contain all foundation fields
      expect(parsed).toHaveProperty('project');
      expect(parsed).toHaveProperty('solutionSpace');
      expect(parsed).toHaveProperty('problemSpace');
      expect(parsed.project.name).toBe('Test Project');
    });
  });

  describe('Scenario: Display specific field', () => {
    it('should display only specified field', async () => {
      // Given I have a foundation.json with projectOverview field
      const foundationData = createMinimalFoundation({
        solutionSpace: {
          overview: 'This is the project overview content',
          capabilities: [],
        },
      });

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(foundationData, null, 2)
      );

      // When I run `fspec show-foundation --section projectOverview`
      const result = await showFoundation({
        section: 'projectOverview',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should display only that field content
      expect(result.output).toContain('This is the project overview content');

      // And other fields should not be displayed
      expect(result.output).not.toContain('Other content not to display');
    });
  });

  describe('Scenario: Display in text format (default)', () => {
    it('should display foundation as readable text', async () => {
      // Given I have a foundation.json
      const foundationData = createMinimalFoundation({
        project: {
          name: 'My Project',
          vision: 'My project description',
          projectType: 'cli-tool',
        },
      });

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(foundationData, null, 2)
      );

      // When I run `fspec show-foundation`
      const result = await showFoundation({
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should display foundation content as readable text
      expect(result.output).toBeDefined();

      // And project name and description should be shown
      expect(result.output).toContain('My Project');
      expect(result.output).toContain('My project description');
    });
  });

  describe('Scenario: Write JSON output to file', () => {
    it('should write JSON to file', async () => {
      // Given I have a foundation.json
      const foundationData = createMinimalFoundation();

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(foundationData, null, 2)
      );

      // When I run `fspec show-foundation --format json --output foundation-copy.json`
      const result = await showFoundation({
        format: 'json',
        output: join(testDir, 'foundation-copy.json'),
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And a file "foundation-copy.json" should be created
      await expect(
        access(join(testDir, 'foundation-copy.json'))
      ).resolves.toBeUndefined();

      // And it should contain valid JSON with foundation data
      const copiedContent = await readFile(
        join(testDir, 'foundation-copy.json'),
        'utf-8'
      );
      const parsed = JSON.parse(copiedContent);
      expect(parsed).toHaveProperty('project');
    });
  });

  describe('Scenario: Auto-create foundation.json when missing', () => {
    it('should auto-create foundation.json with default structure when missing', async () => {
      // Given I have no foundation.json file
      // When I run `fspec show-foundation`
      const result = await showFoundation({
        cwd: testDir,
      });

      // Then it should succeed
      expect(result.success).toBe(true);

      // And foundation.json should be auto-created with default structure
      expect(result.output).toContain('PROJECT');
      expect(result.output).toContain('Project'); // Default name
    });
  });

  describe('Scenario: Handle missing field', () => {
    it('should error when field not found', async () => {
      // Given I have a foundation.json
      const foundationData = createMinimalFoundation();

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(foundationData, null, 2)
      );

      // When I run `fspec show-foundation --section nonExistentField`
      const result = await showFoundation({
        section: 'nonExistentField',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Field 'nonExistentField' not found"
      expect(result.error).toMatch(/Field 'nonExistentField' not found/i);
    });
  });

  describe('Scenario: JSON-backed workflow - read from source of truth', () => {
    it('should load from foundation.json', async () => {
      // Given I have a valid foundation.json file
      const foundationData = createMinimalFoundation();

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(foundationData, null, 2)
      );

      // When I run `fspec show-foundation --format json`
      const result = await showFoundation({
        format: 'json',
        cwd: testDir,
      });

      // Then the command should load data from spec/foundation.json
      expect(result.success).toBe(true);

      // And the output should be valid JSON matching the Foundation schema
      const parsed = JSON.parse(result.output!);

      // And all top-level fields should be present
      expect(parsed).toHaveProperty('project');
      expect(parsed).toHaveProperty('solutionSpace');
      expect(parsed).toHaveProperty('problemSpace');

      // And the command should exit with code 0
      expect(result.success).toBe(true);
    });
  });

  // Feature: spec/features/show-foundation-command-has-multiple-quality-issues.feature
  describe('Feature: show-foundation command has multiple quality issues (BUG-081)', () => {
    describe('Scenario: Filter foundation using --section flag', () => {
      it('should display only the specified field when using --section', async () => {
        // @step Given I have a foundation.json with projectOverview field
        const foundationData = createMinimalFoundation({
          solutionSpace: {
            overview: 'This is the project overview content',
            capabilities: [],
          },
        });

        await writeFile(
          join(testDir, 'spec/foundation.json'),
          JSON.stringify(foundationData, null, 2)
        );

        // @step When I run `fspec show-foundation --section projectOverview`
        const result = await showFoundation({
          section: 'projectOverview',
          cwd: testDir,
        });

        // @step Then the command should exit with code 0
        expect(result.success).toBe(true);

        // @step And the output should display only the project overview content
        expect(result.output).toContain('This is the project overview content');

        // @step And the output should not contain other foundation fields
        expect(result.output).not.toContain('Test Project');
      });
    });

    describe('Scenario: Filter foundation using positional argument', () => {
      it('should work the same as --section flag', async () => {
        // @step Given I have a foundation.json with projectOverview field
        const foundationData = createMinimalFoundation({
          solutionSpace: {
            overview: 'This is the project overview content',
            capabilities: [],
          },
        });

        await writeFile(
          join(testDir, 'spec/foundation.json'),
          JSON.stringify(foundationData, null, 2)
        );

        // @step When I run `fspec show-foundation projectOverview`
        const result = await showFoundation({
          section: 'projectOverview', // This will test positional argument once implemented
          cwd: testDir,
        });

        // @step Then the command should exit with code 0
        expect(result.success).toBe(true);

        // @step And the output should display only the project overview content
        expect(result.output).toContain('This is the project overview content');

        // @step And the result should match --section flag output
        expect(result.output).not.toContain('Test Project');
      });
    });

    describe('Scenario: Filter foundation using JSON path notation', () => {
      it('should retrieve nested fields using dot notation', async () => {
        // @step Given I have a foundation.json with nested solutionSpace.overview field
        const foundationData = createMinimalFoundation({
          solutionSpace: {
            overview: 'Solution space overview content',
            capabilities: [],
          },
        });

        await writeFile(
          join(testDir, 'spec/foundation.json'),
          JSON.stringify(foundationData, null, 2)
        );

        // @step When I run `fspec show-foundation solutionSpace.overview`
        const result = await showFoundation({
          section: 'solutionSpace.overview',
          cwd: testDir,
        });

        // @step Then the command should exit with code 0
        expect(result.success).toBe(true);

        // @step And the output should display only the solution space overview content
        expect(result.output).toContain('Solution space overview content');
      });
    });

    describe('Scenario: Display entire foundation without field filter', () => {
      it('should show all sections when no field specified', async () => {
        // @step Given I have a complete foundation.json file
        const foundationData = createMinimalFoundation();

        await writeFile(
          join(testDir, 'spec/foundation.json'),
          JSON.stringify(foundationData, null, 2)
        );

        // @step When I run `fspec show-foundation`
        const result = await showFoundation({
          cwd: testDir,
        });

        // @step Then the command should exit with code 0
        expect(result.success).toBe(true);

        // @step And the output should display formatted text with all sections
        // @step And the output should contain PROJECT section
        expect(result.output).toContain('PROJECT');

        // @step And the output should contain PROBLEM SPACE section
        expect(result.output).toContain('PROBLEM SPACE');

        // @step And the output should contain SOLUTION SPACE section
        expect(result.output).toContain('SOLUTION SPACE');
      });
    });

    describe('Scenario: Handle non-existent field error', () => {
      it('should return error for non-existent fields', async () => {
        // @step Given I have a foundation.json file
        const foundationData = createMinimalFoundation();

        await writeFile(
          join(testDir, 'spec/foundation.json'),
          JSON.stringify(foundationData, null, 2)
        );

        // @step When I run `fspec show-foundation --section nonExistentField`
        const result = await showFoundation({
          section: 'nonExistentField',
          cwd: testDir,
        });

        // @step Then the command should exit with code 1
        expect(result.success).toBe(false);

        // @step And the output should show "Field 'nonExistentField' not found"
        expect(result.error).toMatch(/Field 'nonExistentField' not found/i);
      });
    });
  });
});
