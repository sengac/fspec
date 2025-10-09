import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFile, writeFile, mkdir, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { validateFoundationJson, validateTagsJson, validateJson } from '../json-schema';

describe('Feature: Validate JSON Files Against Schemas', () => {
  let tmpDir: string;
  let foundationJsonPath: string;
  let tagsJsonPath: string;
  let foundationSchemaPath: string;
  let tagsSchemaPath: string;

  beforeEach(async () => {
    // Create temporary directory
    tmpDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });
    await mkdir(join(tmpDir, 'spec'), { recursive: true });

    foundationJsonPath = join(tmpDir, 'spec', 'foundation.json');
    tagsJsonPath = join(tmpDir, 'spec', 'tags.json');

    // Schemas are bundled in src/schemas/ - tests will use the actual bundled schemas
    foundationSchemaPath = join(process.cwd(), 'src', 'schemas', 'foundation.schema.json');
    tagsSchemaPath = join(process.cwd(), 'src', 'schemas', 'tags.schema.json');
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(tmpDir, { recursive: true, force: true });
  });

  describe('Scenario: Validate foundation.json against schema', () => {
    it('should exit with code 0 and display success message for valid foundation.json', async () => {
      // Given I have a file "spec/foundation.json" with valid structure
      const validFoundation = {
        $schema: './schemas/foundation.schema.json',
        project: {
          name: 'fspec',
          description: 'A CLI tool for AI agents',
          repository: 'https://github.com/rquast/fspec',
          license: 'MIT',
          importantNote: 'This is a legitimate developer tool',
        },
        whatWeAreBuilding: {
          projectOverview: 'Test overview',
          technicalRequirements: {
            coreTechnologies: [
              { category: 'Language', name: 'TypeScript' },
            ],
            architecture: {
              pattern: 'CLI',
              fileStructure: 'test',
              deploymentTarget: 'local',
              integrationModel: ['CLI'],
            },
            developmentAndOperations: {
              developmentTools: 'test',
              testingStrategy: 'test',
              logging: 'test',
              validation: 'test',
              formatting: 'test',
            },
            keyLibraries: [
              {
                category: 'Core',
                libraries: [{ name: 'commander', description: 'CLI framework' }],
              },
            ],
          },
          nonFunctionalRequirements: [
            { category: 'Reliability', requirements: ['test'] },
          ],
        },
        whyWeAreBuildingIt: {
          problemDefinition: {
            primary: {
              title: 'Test Problem',
              description: 'Test description',
              points: ['point 1'],
            },
            secondary: ['secondary problem'],
          },
          painPoints: {
            currentState: 'test state',
            specific: [
              {
                title: 'Pain 1',
                impact: 'high',
                frequency: 'often',
                cost: 'expensive',
              },
            ],
          },
          stakeholderImpact: [
            { stakeholder: 'Developers', description: 'Impact description' },
          ],
          theoreticalSolutions: [
            {
              title: 'Solution 1',
              selected: true,
              description: 'Test solution',
              pros: ['pro 1'],
              cons: ['con 1'],
              feasibility: 'high',
            },
          ],
          developmentMethodology: {
            name: 'ACDD',
            description: 'Test',
            steps: ['step 1'],
            ensures: ['ensure 1'],
          },
          successCriteria: [{ title: 'Criterion 1', criteria: ['test'] }],
          constraintsAndAssumptions: {
            constraints: [{ category: 'Technical', items: ['constraint 1'] }],
            assumptions: [{ category: 'Technical', items: ['assumption 1'] }],
          },
        },
        architectureDiagrams: [
          { title: 'Diagram 1', mermaidCode: 'graph TB' },
        ],
        coreCommands: {
          categories: [
            {
              title: 'Feature Commands',
              commands: [
                {
                  command: 'fspec create-feature',
                  description: 'Create feature',
                  status: '✅',
                },
              ],
            },
          ],
        },
        featureInventory: {
          phases: [
            {
              phase: '@phase1',
              title: 'Phase 1',
              description: 'Test phase',
              features: [
                {
                  featureFile: 'test.feature',
                  command: 'fspec test',
                  description: 'Test feature',
                },
              ],
            },
          ],
          tagUsageSummary: {
            phaseDistribution: [{ tag: '@phase1', count: 1, percentage: '100%' }],
            componentDistribution: [{ tag: '@cli', count: 1, percentage: '100%' }],
            featureGroupDistribution: [{ tag: '@feature-management', count: 1, percentage: '100%' }],
            priorityDistribution: [{ tag: '@critical', count: 1, percentage: '100%' }],
            testingCoverage: [{ tag: '@unit-test', count: 1, percentage: '100%' }],
          },
        },
        notes: {
          developmentStatus: [
            {
              phase: '@phase1',
              title: 'Phase 1',
              status: 'COMPLETE',
              items: ['item 1'],
            },
          ],
        },
      };

      await writeFile(foundationJsonPath, JSON.stringify(validFoundation, null, 2));

      // When I run `fspec validate-json spec/foundation.json`
      const result = await validateFoundationJson(foundationJsonPath, foundationSchemaPath);

      // Then the command should exit with code 0
      expect(result.valid).toBe(true);
      expect(result.errors).toEqual([]);
    });
  });

  describe('Scenario: Detect missing required field in foundation.json', () => {
    it('should exit with code 1 and show validation error for missing required field', async () => {
      // Given I have a file "spec/foundation.json" with missing required field "project.name"
      const invalidFoundation = {
        $schema: './schemas/foundation.schema.json',
        project: {
          // Missing required field: name
          description: 'A CLI tool',
          repository: 'https://github.com/test',
          license: 'MIT',
          importantNote: 'Note',
        },
      };

      await writeFile(foundationJsonPath, JSON.stringify(invalidFoundation, null, 2));

      // When I run `fspec validate-json spec/foundation.json`
      const result = await validateFoundationJson(foundationJsonPath, foundationSchemaPath);

      // Then the command should exit with code 1
      expect(result.valid).toBe(false);

      // And the output should contain "Validation error at /project"
      expect(result.errors.length).toBeGreaterThan(0);
      expect(result.errors.some((err) => err.instancePath.includes('/project'))).toBe(true);

      // And the output should contain "must have required property 'name'"
      expect(result.errors.some((err) => err.message.includes('required property'))).toBe(true);
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
          examples: [{ description: 'All phase 1', command: 'fspec list-features --tag=@phase1' }],
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
          componentStats: [{ component: '@cli', count: 28, percentage: '100%' }],
          featureGroupStats: [{ featureGroup: '@feature-management', count: 11, percentage: '39%' }],
          updateCommand: 'fspec tag-stats',
        },
        validation: {
          rules: [{ rule: 'Registry Compliance', description: 'All tags must be registered' }],
          commands: [{ description: 'Validate all', command: 'fspec validate-tags' }],
        },
        references: [
          { title: 'Gherkin Reference', url: 'https://cucumber.io/docs/gherkin' },
        ],
      };

      await writeFile(tagsJsonPath, JSON.stringify(validTags, null, 2));

      // When I run `fspec validate-json spec/tags.json`
      const result = await validateTagsJson(tagsJsonPath, tagsSchemaPath);

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
      const result = await validateTagsJson(tagsJsonPath, tagsSchemaPath);

      // Then the command should exit with code 1
      expect(result.valid).toBe(false);

      // And the output should contain "Validation error at /categories/0/tags/0/name"
      const nameError = result.errors.find((err) =>
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
      const validFoundation = {
        $schema: './schemas/foundation.schema.json',
        project: {
          name: 'fspec',
          description: 'Test',
          repository: 'https://github.com/test',
          license: 'MIT',
          importantNote: 'Note',
        },
        whatWeAreBuilding: {
          projectOverview: 'Test',
          technicalRequirements: {
            coreTechnologies: [],
            architecture: {
              pattern: 'CLI',
              fileStructure: 'test',
              deploymentTarget: 'local',
              integrationModel: [],
            },
            developmentAndOperations: {
              developmentTools: 'test',
              testingStrategy: 'test',
              logging: 'test',
              validation: 'test',
              formatting: 'test',
            },
            keyLibraries: [],
          },
          nonFunctionalRequirements: [],
        },
        whyWeAreBuildingIt: {
          problemDefinition: {
            primary: { title: 'Test', description: 'Test', points: [] },
            secondary: [],
          },
          painPoints: { currentState: 'Test', specific: [] },
          stakeholderImpact: [],
          theoreticalSolutions: [],
          developmentMethodology: {
            name: 'ACDD',
            description: 'Test',
            steps: [],
            ensures: [],
          },
          successCriteria: [],
          constraintsAndAssumptions: { constraints: [], assumptions: [] },
        },
        architectureDiagrams: [],
        coreCommands: { categories: [] },
        featureInventory: {
          phases: [],
          tagUsageSummary: {
            phaseDistribution: [],
            componentDistribution: [],
            featureGroupDistribution: [],
            priorityDistribution: [],
            testingCoverage: [],
          },
        },
        notes: { developmentStatus: [] },
      };

      const validTags = {
        $schema: './schemas/tags.schema.json',
        categories: [],
        combinationExamples: [],
        usageGuidelines: {
          requiredCombinations: { title: 'Test', requirements: [], minimumExample: 'test' },
          recommendedCombinations: { title: 'Test', includes: [], recommendedExample: 'test' },
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

      await writeFile(foundationJsonPath, JSON.stringify(validFoundation, null, 2));
      await writeFile(tagsJsonPath, JSON.stringify(validTags, null, 2));

      // When I run `fspec validate-json`
      const results = await validateJson(tmpDir);

      // Then the command should validate both files
      expect(results.length).toBe(2);

      // And if all valid, exit with code 0
      const allValid = results.every((r) => r.valid);
      expect(allValid).toBe(true);
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

      await writeFile(foundationJsonPath, JSON.stringify(invalidFoundation, null, 2));

      // When I run `fspec validate-json spec/foundation.json`
      const result = await validateFoundationJson(foundationJsonPath, foundationSchemaPath);

      // Then the output should list all validation errors
      expect(result.errors.length).toBeGreaterThan(0);

      // And each error should show the JSON path
      result.errors.forEach((error) => {
        expect(error).toHaveProperty('instancePath');
      });

      // And each error should show the validation rule violated
      result.errors.forEach((error) => {
        expect(error).toHaveProperty('message');
      });

      // And each error should show the expected value or format
      result.errors.forEach((error) => {
        expect(error.message).toBeTruthy();
      });
    });
  });
});
