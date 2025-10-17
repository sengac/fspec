import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFile, writeFile, mkdir, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import {
  validateFoundationJson,
  validateTagsJson,
  validateJson,
} from '../json-schema';
import { createMinimalFoundation, createCompleteFoundation } from '../../test-helpers/foundation-fixtures';

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

  describe('Scenario: Validate foundation.json against schema', () => {
    it('should exit with code 0 and display success message for valid foundation.json', async () => {
      // Given I have a file "spec/foundation.json" with valid structure
      const validFoundation = createCompleteFoundation();

      await writeFile(
        foundationJsonPath,
        JSON.stringify(validFoundation, null, 2)
      );

      // When I run `fspec validate-json spec/foundation.json`
      const result = await validateFoundationJson(foundationJsonPath);

      // Then the command should exit with code 0
      expect(result.valid).toBe(true);
      expect(result.errors).toEqual([]);
    });
  });

  describe('Scenario: Detect missing required field in foundation.json', () => {
    it('should exit with code 1 and show validation error for missing required field', async () => {
      // Given I have a file "spec/foundation.json" with missing required field "project.name"
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

      await writeFile(
        foundationJsonPath,
        JSON.stringify(invalidFoundation, null, 2)
      );

      // When I run `fspec validate-json spec/foundation.json`
      const result = await validateFoundationJson(foundationJsonPath);

      // Then the command should exit with code 1
      expect(result.valid).toBe(false);

      // And the output should contain validation errors
      expect(result.errors.length).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Validate tags.json against schema', () => {
    it('should exit with code 0 and display success message for valid tags.json', async () => {
      // Given I have a file "spec/tags.json" with valid structure
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

      await writeFile(tagsJsonPath, JSON.stringify(validTags, null, 2));

      // When I run `fspec validate-json spec/tags.json`
      const result = await validateTagsJson(tagsJsonPath);

      // Then the command should exit with code 0
      expect(result.valid).toBe(true);
      expect(result.errors).toEqual([]);
    });
  });

  describe('Scenario: Detect invalid tag name format', () => {
    it('should exit with code 1 for tag name without @ prefix', async () => {
      // Given I have a file "spec/tags.json"
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

      await writeFile(tagsJsonPath, JSON.stringify(invalidTags, null, 2));

      // When I run `fspec validate-json spec/tags.json`
      const result = await validateTagsJson(tagsJsonPath);

      // Then the command should exit with code 1
      expect(result.valid).toBe(false);

      // And the output should contain "Validation error at /categories/0/tags/0/name"
      const nameError = result.errors.find(err =>
        err.instancePath.includes('/categories/0/tags/0/name')
      );
      expect(nameError).toBeDefined();

      // And the output should contain "must match pattern"
      expect(nameError?.message).toContain('pattern');
    });
  });

  describe('Scenario: Validate all JSON files at once', () => {
    it('should validate both foundation.json and tags.json', async () => {
      // Given I have "spec/foundation.json" and "spec/tags.json"
      const validFoundation = createMinimalFoundation();

      const validTags = {
        $schema: './schemas/tags.schema.json',
        categories: [],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: {
            title: 'Test',
            requirements: [],
            minimumExample: 'test',
          },
          recommendedCombinations: {
            title: 'Test',
            includes: [],
            recommendedExample: 'test',
          },
          orderingConvention: { title: 'Test', order: [], example: 'test' },
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

      await writeFile(
        foundationJsonPath,
        JSON.stringify(validFoundation, null, 2)
      );
      await writeFile(tagsJsonPath, JSON.stringify(validTags, null, 2));

      // When I run `fspec validate-json`
      const results = await validateJson(tmpDir);

      // Then the command should validate both files
      expect(results.length).toBe(2);

      // And if all valid, exit with code 0
      const allValid = results.every(r => r.valid);
      expect(allValid).toBe(true);
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

  describe('Scenario: Show detailed validation errors with context', () => {
    it('should list all validation errors with JSON paths', async () => {
      // Given I have a file "spec/foundation.json" with multiple validation errors
      const invalidFoundation = {
        $schema: './schemas/foundation.schema.json',
        // Missing required field: project
        whatWeAreBuilding: {
          // Missing required fields
        },
        // Missing other required top-level fields
      };

      await writeFile(
        foundationJsonPath,
        JSON.stringify(invalidFoundation, null, 2)
      );

      // When I run `fspec validate-json spec/foundation.json`
      const result = await validateFoundationJson(foundationJsonPath);

      // Then the output should list all validation errors
      expect(result.errors.length).toBeGreaterThan(0);

      // And each error should show the JSON path
      result.errors.forEach(error => {
        expect(error).toHaveProperty('instancePath');
      });

      // And each error should show the validation rule violated
      result.errors.forEach(error => {
        expect(error).toHaveProperty('message');
      });

      // And each error should show the expected value or format
      result.errors.forEach(error => {
        expect(error.message).toBeTruthy();
      });
    });
  });
});
