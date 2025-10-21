import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFile, writeFile, mkdir, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import {
  validateFoundationJson,
  validateFoundationObject,
  validateTagsJson,
  validateTagsObject,
  formatValidationErrors,
} from '../json-schema';
import {
  createMinimalFoundation,
  createCompleteFoundation,
} from '../../test-helpers/foundation-fixtures';

describe('Feature: Validate JSON Files Against Schemas', () => {
  let tmpDir: string;
  let foundationJsonPath: string;
  let tagsJsonPath: string;

  beforeEach(async () => {
    // Create temporary directory
    tmpDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });
    await mkdir(join(tmpDir, 'spec'), { recursive: true });

    foundationJsonPath = join(tmpDir, 'spec', 'foundation.json');
    tagsJsonPath = join(tmpDir, 'spec', 'tags.json');
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(tmpDir, { recursive: true, force: true });
  });

  describe('Scenario: Validate valid foundation.json', () => {
    it('should pass validation and return no errors', () => {
      // Given I have a foundation.json object with valid structure
      const validFoundation = createCompleteFoundation();

      // When the validation utility validates it against foundation.schema.json
      const result = validateFoundationObject(validFoundation);

      // Then the validation should pass
      expect(result.valid).toBe(true);

      // And no errors should be returned
      expect(result.errors).toEqual([]);
    });
  });

  describe('Scenario: Detect missing required field in foundation.json', () => {
    it('should exit with code 1 and show validation error for missing required field', () => {
      // Given I have a foundation.json object missing required field "project.name"
      const invalidFoundation = {
        version: '2.0.0',
        project: {
          // Missing required fields: name, vision, projectType
          repository: 'https://github.com/test',
          license: 'MIT',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test',
            description: 'Test',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test',
          capabilities: [],
        },
      };

      // When the validation utility validates it against foundation.schema.json
      const result = validateFoundationObject(invalidFoundation);

      // Then the validation should fail
      expect(result.valid).toBe(false);

      // And the error should indicate "/project" path
      expect(result.errors.some(e => e.instancePath === '/project')).toBe(true);

      // And the error should mention "must have required property 'name'"
      const projectError = result.errors.find(
        e => e.instancePath === '/project'
      );
      expect(projectError?.message).toContain(
        "must have required property 'name'"
      );
    });
  });

  describe('Scenario: Validate valid tags.json', () => {
    it('should pass validation and return no errors', () => {
      // Given I have a tags.json object with valid structure
      const validTags = {
        $schema: './schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Phase identification tags',
            required: true,
            tags: [
              {
                name: '@phase1',
                description: 'Phase 1',
                usage: 'Core features',
              },
            ],
          },
        ],
        combinationExamples: [
          {
            title: 'Example 1',
            tags: '@phase1 @cli',
            interpretation: ['Phase 1', 'CLI component'],
          },
        ],
        usageGuidelines: {
          requiredCombinations: {
            title: 'Required',
            requirements: ['Phase tag', 'Component tag'],
            minimumExample: '@phase1 @cli @feature-management',
          },
          recommendedCombinations: {
            title: 'Recommended',
            includes: ['Technical tags'],
            recommendedExample: '@phase1 @cli @feature-management @gherkin',
          },
          orderingConvention: {
            title: 'Order',
            order: ['Phase', 'Component', 'Feature group'],
            example: '@phase1 @cli @feature-management',
          },
        },
        addingNewTags: {
          process: [{ step: 'Step 1', description: 'Identify need' }],
          namingConventions: ['Use lowercase'],
          antiPatterns: {
            dont: [{ description: 'Create overlapping tags' }],
            do: [{ description: 'Reuse existing tags' }],
          },
        },
        queries: {
          title: 'Common Queries',
          examples: [
            {
              description: 'All phase 1',
              command: 'fspec list-features --tag=@phase1',
            },
          ],
        },
        statistics: {
          lastUpdated: '2025-01-15T10:30:00Z',
          phaseStats: [
            {
              phase: 'Phase 1',
              total: 5,
              complete: 5,
              inProgress: 0,
              planned: 0,
            },
          ],
          componentStats: [
            { component: '@cli', count: 28, percentage: '100%' },
          ],
          featureGroupStats: [
            {
              featureGroup: '@feature-management',
              count: 11,
              percentage: '39%',
            },
          ],
          updateCommand: 'fspec tag-stats',
        },
        validation: {
          rules: [
            {
              rule: 'Registry Compliance',
              description: 'All tags must be registered',
            },
          ],
          commands: [
            { description: 'Validate all', command: 'fspec validate-tags' },
          ],
        },
        references: [
          {
            title: 'Gherkin Reference',
            url: 'https://cucumber.io/docs/gherkin',
          },
        ],
      };

      // When the validation utility validates it against tags.schema.json
      const result = validateTagsObject(validTags);

      // Then the validation should pass
      expect(result.valid).toBe(true);

      // And no errors should be returned
      expect(result.errors).toEqual([]);
    });
  });

  describe('Scenario: Detect invalid tag name format', () => {
    it('should fail validation for tag name without @ prefix', () => {
      // Given I have a tags.json object
      // And it contains a tag "phase1" without @ prefix
      const invalidTags = {
        $schema: './schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Phase tags',
            required: true,
            tags: [
              {
                name: 'phase1', // Invalid: missing @ prefix
                description: 'Phase 1',
              },
            ],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: {
            title: 'Test',
            requirements: ['test'],
            minimumExample: 'test',
          },
          recommendedCombinations: {
            title: 'Test',
            includes: ['test'],
            recommendedExample: 'test',
          },
          orderingConvention: {
            title: 'Test',
            order: ['test'],
            example: 'test',
          },
        },
        addingNewTags: {
          process: [],
          namingConventions: [],
          antiPatterns: { dont: [], do: [] },
        },
        queries: { title: 'Test', examples: [] },
        statistics: {
          lastUpdated: '2025-01-15T10:30:00Z',
          phaseStats: [],
          componentStats: [],
          featureGroupStats: [],
          updateCommand: 'test',
        },
        validation: { rules: [], commands: [] },
        references: [],
      };

      // When the validation utility validates it against tags.schema.json
      const result = validateTagsObject(invalidTags);

      // Then the validation should fail
      expect(result.valid).toBe(false);

      // And the error should indicate the tag name field path
      const nameError = result.errors.find(err =>
        err.instancePath.includes('/categories/0/tags/0/name')
      );
      expect(nameError).toBeDefined();

      // And the error should mention pattern requirement "^@[a-z0-9-]+$"
      expect(nameError?.message).toContain('pattern');
    });
  });

  describe('Scenario: Handle malformed JSON file', () => {
    it('should throw a SyntaxError for invalid JSON syntax', async () => {
      // Given I have a file "spec/foundation.json" with invalid JSON syntax
      const malformedJson = '{"project": {"name": "test",}'; // Trailing comma - invalid JSON

      await writeFile(foundationJsonPath, malformedJson);

      // When the validation utility tries to read the file
      // Then it should throw a SyntaxError
      await expect(
        validateFoundationJson(foundationJsonPath)
      ).rejects.toThrow();

      // And the error message should indicate JSON parsing failure
      try {
        await validateFoundationJson(foundationJsonPath);
      } catch (error: unknown) {
        if (error instanceof Error) {
          expect(error.message.toLowerCase()).toMatch(/json|parse|syntax/);
        }
      }
    });
  });

  describe('Scenario: Format validation errors for display', () => {
    it('should format errors with JSON path and human-readable messages', () => {
      // Given I have multiple validation errors from Ajv
      const invalidFoundation = {
        version: '2.0.0',
        project: {
          // Missing required field: name
        },
        // Missing other required top-level fields
      };

      const result = validateFoundationObject(invalidFoundation);
      expect(result.errors.length).toBeGreaterThan(0); // Ensure we have errors

      // When I format the errors for display
      const formattedErrors = formatValidationErrors(result.errors);

      // Then each error should show the JSON path
      formattedErrors.forEach(errorMessage => {
        expect(errorMessage).toMatch(/Validation error at .+:/);
      });

      // And each error should show the validation rule violated
      formattedErrors.forEach(errorMessage => {
        expect(errorMessage).toContain('must');
      });

      // And each error should be human-readable
      const firstError = formattedErrors[0];
      expect(firstError).toContain('Validation error at');
      expect(firstError).toContain(':');
    });
  });
});
