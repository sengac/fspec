import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import {
  validateFoundationJson,
  validateTagsJson,
  validateFromFile,
  formatValidationErrors,
} from '../validate-json-schema';
import type { Foundation } from '../../types/foundation';
import type { Tags } from '../../types/tags';
import { createMinimalFoundation } from '../../test-helpers/foundation-fixtures';

describe('Feature: Internal JSON Schema Validation', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-validate-json-schema');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Validate valid foundation.json', () => {
    it('should pass validation for valid foundation.json', () => {
      // Given I have a foundation.json object with valid structure
      const validFoundation = createMinimalFoundation() as unknown as Foundation;

      // When the validation utility validates it against generic-foundation.schema.json
      const result = validateFoundationJson(validFoundation);

      // Then the validation should pass
      expect(result.valid).toBe(true);
      // And no errors should be returned
      expect(result.errors).toBeUndefined();
    });
  });

  describe('Scenario: Detect missing required field in foundation.json', () => {
    it('should fail when required field project.name is missing', () => {
      // Given I have a foundation.json object missing required field "project.name"
      const invalidFoundation = {
        $schema: '../src/schemas/foundation.schema.json',
        project: {
          // name is missing
          description: 'Test description',
          repository: 'https://github.com/test/test',
          license: 'MIT',
          importantNote: 'Test note',
        },
      };

      // When the validation utility validates it against foundation.schema.json
      const result = validateFoundationJson(
        invalidFoundation as unknown as Foundation
      );

      // Then the validation should fail
      expect(result.valid).toBe(false);
      expect(result.errors).toBeDefined();
      expect(result.errors!.length).toBeGreaterThan(0);

      // And there should be errors - either at root (/) for missing top-level fields
      // or at /project for missing project.name
      // AJV reports first error which could be missing root fields
      const errors = result.errors!;
      const hasRootError = errors.some(e => e.path === '/');
      const hasProjectError = errors.some(
        e => e.path === '/project' && e.message.includes('name')
      );

      // Either root-level missing fields OR missing project.name should be reported
      expect(hasRootError || hasProjectError).toBe(true);
    });
  });

  describe('Scenario: Validate valid tags.json', () => {
    it('should pass validation for valid tags.json', () => {
      // Given I have a tags.json object with valid structure
      const validTags: Tags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Test',
            required: false,
            tags: [{ name: '@phase1', description: 'Phase 1' }],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: {
            title: '',
            requirements: [],
            minimumExample: '',
          },
          recommendedCombinations: {
            title: '',
            includes: [],
            recommendedExample: '',
          },
          orderingConvention: { title: '', order: [], example: '' },
        },
        addingNewTags: {
          process: [],
          namingConventions: [],
          antiPatterns: { dont: [], do: [] },
        },
        queries: { title: '', examples: [] },
        statistics: {
          lastUpdated: '2025-01-01T00:00:00Z',
          phaseStats: [],
          componentStats: [],
          featureGroupStats: [],
          updateCommand: '',
        },
        validation: { rules: [], commands: [] },
        references: [],
      };

      // When the validation utility validates it against tags.schema.json
      const result = validateTagsJson(validTags);

      // Then the validation should pass
      expect(result.valid).toBe(true);
      // And no errors should be returned
      expect(result.errors).toBeUndefined();
    });
  });

  describe('Scenario: Detect invalid tag name format', () => {
    it('should fail when tag name does not match pattern', () => {
      // Given I have a tags.json object
      // And it contains a tag "phase1" without @ prefix
      const invalidTags: Tags = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Test',
            required: false,
            tags: [{ name: 'phase1', description: 'Invalid - no @ prefix' }], // Invalid tag name
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: {
            title: '',
            requirements: [],
            minimumExample: '',
          },
          recommendedCombinations: {
            title: '',
            includes: [],
            recommendedExample: '',
          },
          orderingConvention: { title: '', order: [], example: '' },
        },
        addingNewTags: {
          process: [],
          namingConventions: [],
          antiPatterns: { dont: [], do: [] },
        },
        queries: { title: '', examples: [] },
        statistics: {
          lastUpdated: '2025-01-01T00:00:00Z',
          phaseStats: [],
          componentStats: [],
          featureGroupStats: [],
          updateCommand: '',
        },
        validation: { rules: [], commands: [] },
        references: [],
      };

      // When the validation utility validates it against tags.schema.json
      const result = validateTagsJson(invalidTags);

      // Then the validation should fail
      expect(result.valid).toBe(false);
      expect(result.errors).toBeDefined();

      // And the error should indicate the tag name field path
      const error = result.errors![0];
      expect(error.path).toBe('/categories/0/tags/0/name');

      // And the error should mention pattern requirement "^@[a-z0-9-]+$"
      expect(error.message).toContain('pattern');
    });
  });

  describe('Scenario: Validate from file path', () => {
    it('should read and validate JSON file', async () => {
      // Given I have a file "spec/foundation.json" with valid JSON
      const validFoundation = createMinimalFoundation();

      const filePath = join(testDir, 'spec', 'foundation.json');
      await writeFile(filePath, JSON.stringify(validFoundation, null, 2));

      // When the validation utility reads and validates the file
      const result = await validateFromFile(filePath, 'foundation');

      // Then the validation should pass
      expect(result.valid).toBe(true);
      // And the JSON data should be returned
      expect(result.data).toBeDefined();
      expect(result.data!.project.name).toBe('Test Project');
    });
  });

  describe('Scenario: Handle malformed JSON file', () => {
    it('should throw SyntaxError for invalid JSON', async () => {
      // Given I have a file "spec/foundation.json" with invalid JSON syntax
      const filePath = join(testDir, 'spec', 'foundation.json');
      await writeFile(filePath, '{ invalid json syntax here }');

      // When the validation utility tries to read the file
      // Then it should throw a SyntaxError
      await expect(validateFromFile(filePath, 'foundation')).rejects.toThrow();

      // And the error message should indicate JSON parsing failure
      try {
        await validateFromFile(filePath, 'foundation');
      } catch (error) {
        expect(error).toBeInstanceOf(Error);
        expect((error as Error).message).toContain('JSON');
      }
    });
  });

  describe('Scenario: Format validation errors for display', () => {
    it('should format Ajv errors as human-readable messages', () => {
      // Given I have multiple validation errors from Ajv
      const ajvErrors = [
        {
          instancePath: '/project/name',
          schemaPath: '#/properties/project/properties/name/type',
          keyword: 'type',
          params: { type: 'string' },
          message: 'must be string',
        },
        {
          instancePath: '/project',
          schemaPath: '#/properties/project/required',
          keyword: 'required',
          params: { missingProperty: 'description' },
          message: "must have required property 'description'",
        },
      ];

      // When I format the errors for display
      const formatted = formatValidationErrors(ajvErrors);

      // Then each error should show the JSON path
      expect(formatted[0].path).toBe('/project/name');
      expect(formatted[1].path).toBe('/project');

      // And each error should show the validation rule violated
      expect(formatted[0].message).toContain('must be string');
      expect(formatted[1].message).toContain(
        "must have required property 'description'"
      );

      // And each error should be human-readable
      expect(formatted[0]).toHaveProperty('path');
      expect(formatted[0]).toHaveProperty('message');
    });
  });
});
