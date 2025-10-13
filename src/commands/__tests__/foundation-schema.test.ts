/**
 * Feature: spec/features/foundation-schema-guidance.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { showFoundationSchema } from '../show-foundation-schema';
import { validateFoundationSchema } from '../validate-foundation-schema';

describe('Feature: Foundation Schema Guidance for AI Agents', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-foundation-schema');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Display foundation schema with rich descriptions', () => {
    it('should display complete JSON Schema with rich descriptions and examples', async () => {
      // Given I am an AI agent working on a project using fspec
      // When I run "fspec show-foundation-schema"
      const result = await showFoundationSchema({
        cwd: testDir,
      });

      // Then I should receive the complete JSON Schema for foundation.json
      expect(result.success).toBe(true);
      expect(result.output).toBeDefined();

      // And the schema should include rich descriptions explaining the intent behind each section
      const schema = JSON.parse(result.output!);
      expect(schema.properties.whatWeAreBuilding.properties.projectOverview.description).toBeDefined();
      expect(schema.properties.whatWeAreBuilding.properties.projectOverview.description).toContain('WHAT and HOW');

      // And the schema should include examples of good vs. bad content for key fields
      expect(schema.properties.whatWeAreBuilding.properties.projectOverview.examples).toBeDefined();

      // And the schema should include format instructions for structured fields
      expect(schema.properties.whyWeAreBuildingIt.properties.problemDefinition).toBeDefined();

      // And the output should be valid JSON Schema format
      expect(schema.$schema).toBe('http://json-schema.org/draft-07/schema#');
      expect(schema.type).toBe('object');
    });
  });

  describe('Scenario: Schema accepts flexible field naming (camelCase and snake_case)', () => {
    it('should validate foundation.json with snake_case field names', async () => {
      // Given I have foundation.json with snake_case field names
      const foundationData = {
        $schema: '../src/schemas/foundation.schema.json',
        project: {
          name: 'Test Project',
          description: 'Test',
          repository: 'https://test.com',
          license: 'MIT',
          importantNote: 'Test',
        },
        // And the file uses "architecture_diagrams" instead of "architectureDiagrams"
        architecture_diagrams: [],
        what_we_are_building: {
          // And the file uses "project_overview" instead of "projectOverview"
          project_overview: 'A test project overview',
          // And the file uses "technical_requirements" instead of "technicalRequirements"
          technical_requirements: {
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
        why_we_are_building_it: {
          problemDefinition: {
            primary: {
              title: 'Test',
              description: 'Test',
              coreProblems: ['Problem 1', 'Problem 2', 'Problem 3'],
            },
          },
          painPoints: {
            beforeFspec: 'Test',
            afterFspec: 'Test',
          },
          stakeholderImpact: [],
          whyACDD: {
            title: 'Test',
            description: 'Test',
            specificationByExample: {
              principle: 'Test',
              benefit: 'Test',
            },
            bdd: {
              principle: 'Test',
              benefit: 'Test',
            },
            acdd: {
              principle: 'Test',
              benefit: 'Test',
              challenge: 'Test',
            },
          },
          successCriteria: [],
        },
      };

      await writeFile(
        join(testDir, 'spec', 'foundation.json'),
        JSON.stringify(foundationData, null, 2)
      );

      // When I run "fspec validate-foundation-schema"
      const result = await validateFoundationSchema({
        cwd: testDir,
      });

      // Then the validation should pass
      expect(result.success).toBe(true);

      // And the output should confirm all fields are valid
      expect(result.output).toContain('valid');
    });
  });

  describe('Scenario: Schema provides semantic guidance for projectOverview', () => {
    it('should show detailed guidance about projectOverview content', async () => {
      // Given I have foundation.json with projectOverview field
      // And the schema description states: "Focus on WHAT and HOW, not WHY. Business justification belongs in whyWeAreBuildingIt"
      // When I read the schema using "fspec show-foundation-schema"
      const result = await showFoundationSchema({
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const schema = JSON.parse(result.output!);
      const projectOverviewSchema =
        schema.properties.whatWeAreBuilding.properties.projectOverview;

      // Then I should see detailed guidance on projectOverview content
      expect(projectOverviewSchema.description).toBeDefined();

      // And the description should specify it should be 2-4 sentences
      expect(projectOverviewSchema.description).toMatch(/2[- ]?4 sentence/i);

      // And the description should specify to describe: what systems are built, who uses them, how they integrate
      expect(projectOverviewSchema.description).toContain('what systems');
      expect(projectOverviewSchema.description).toContain('who uses');
      expect(projectOverviewSchema.description).toContain('how');

      // And the description should warn against including business justification
      expect(projectOverviewSchema.description).toContain('WHY');
      expect(projectOverviewSchema.description).toContain('whyWeAreBuildingIt');
    });
  });

  describe('Scenario: Validate foundation.json with minimum array length requirements', () => {
    it('should fail validation when coreProblems has fewer than 3 items', async () => {
      // Given I have foundation.json with problemDefinition.primary.coreProblems
      // And the array contains only 2 items
      const foundationData = {
        $schema: '../src/schemas/foundation.schema.json',
        project: {
          name: 'Test',
          description: 'Test',
          repository: 'https://test.com',
          license: 'MIT',
          importantNote: 'Test',
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
            primary: {
              title: 'Problem',
              description: 'Description',
              // And the schema requires minimum 3 items for coreProblems
              coreProblems: ['Problem 1', 'Problem 2'], // Only 2 items
            },
          },
          painPoints: {
            beforeFspec: 'Test',
            afterFspec: 'Test',
          },
          stakeholderImpact: [],
          whyACDD: {
            title: 'Test',
            description: 'Test',
            specificationByExample: {
              principle: 'Test',
              benefit: 'Test',
            },
            bdd: {
              principle: 'Test',
              benefit: 'Test',
            },
            acdd: {
              principle: 'Test',
              benefit: 'Test',
              challenge: 'Test',
            },
          },
          successCriteria: [],
        },
        architectureDiagrams: [],
      };

      await writeFile(
        join(testDir, 'spec', 'foundation.json'),
        JSON.stringify(foundationData, null, 2)
      );

      // When I run "fspec validate-foundation-schema"
      const result = await validateFoundationSchema({
        cwd: testDir,
      });

      // Then the validation should fail
      expect(result.success).toBe(false);

      // And the error message should state: "Field problemDefinition.primary.coreProblems must have at least 3 items (found 2)"
      expect(result.error).toContain('coreProblems');
      expect(result.error).toContain('3');
      expect(result.error).toContain('2');

      // And the error message should be clear enough for AI to self-correct
      expect(result.error).toBeDefined();
    });
  });

  describe('Scenario: Schema allows additional custom sections for extensibility', () => {
    it('should allow custom sections when additionalProperties is true', async () => {
      // Given I have foundation.json with standard sections (project, whatWeAreBuilding, whyWeAreBuildingIt)
      // And I add a custom section called "deploymentStrategy"
      const foundationData = {
        $schema: '../src/schemas/foundation.schema.json',
        project: {
          name: 'Test',
          description: 'Test',
          repository: 'https://test.com',
          license: 'MIT',
          importantNote: 'Test',
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
            primary: {
              title: 'Test',
              description: 'Test',
              coreProblems: ['P1', 'P2', 'P3'],
            },
          },
          painPoints: {
            beforeFspec: 'Test',
            afterFspec: 'Test',
          },
          stakeholderImpact: [],
          whyACDD: {
            title: 'Test',
            description: 'Test',
            specificationByExample: {
              principle: 'Test',
              benefit: 'Test',
            },
            bdd: {
              principle: 'Test',
              benefit: 'Test',
            },
            acdd: {
              principle: 'Test',
              benefit: 'Test',
              challenge: 'Test',
            },
          },
          successCriteria: [],
        },
        architectureDiagrams: [],
        // Custom section
        deploymentStrategy: {
          environments: ['dev', 'staging', 'production'],
          cicd: 'GitHub Actions',
        },
      };

      await writeFile(
        join(testDir, 'spec', 'foundation.json'),
        JSON.stringify(foundationData, null, 2)
      );

      // And the schema has additionalProperties: true
      // When I run "fspec validate-foundation-schema"
      const result = await validateFoundationSchema({
        cwd: testDir,
      });

      // Then the validation should pass
      expect(result.success).toBe(true);

      // And the custom section should be allowed
      // And the output should confirm the file is valid
      expect(result.output).toContain('valid');
    });
  });

  describe('Scenario: Mermaid syntax validation delegated to mermaid library', () => {
    it('should pass JSON Schema validation even with invalid Mermaid syntax', async () => {
      // Given I have foundation.json with architectureDiagrams array
      // And one diagram contains invalid Mermaid syntax
      const foundationData = {
        $schema: '../src/schemas/foundation.schema.json',
        project: {
          name: 'Test',
          description: 'Test',
          repository: 'https://test.com',
          license: 'MIT',
          importantNote: 'Test',
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
            primary: {
              title: 'Test',
              description: 'Test',
              coreProblems: ['P1', 'P2', 'P3'],
            },
          },
          painPoints: {
            beforeFspec: 'Test',
            afterFspec: 'Test',
          },
          stakeholderImpact: [],
          whyACDD: {
            title: 'Test',
            description: 'Test',
            specificationByExample: {
              principle: 'Test',
              benefit: 'Test',
            },
            bdd: {
              principle: 'Test',
              benefit: 'Test',
            },
            acdd: {
              principle: 'Test',
              benefit: 'Test',
              challenge: 'Test',
            },
          },
          successCriteria: [],
        },
        architectureDiagrams: [
          {
            section: 'Architecture',
            title: 'System Diagram',
            // Invalid Mermaid syntax
            mermaidCode: 'this is not valid mermaid syntax <<<>>>',
          },
        ],
      };

      await writeFile(
        join(testDir, 'spec', 'foundation.json'),
        JSON.stringify(foundationData, null, 2)
      );

      // And the JSON Schema only validates that mermaidCode is a string
      // When I run "fspec validate-foundation-schema"
      const result = await validateFoundationSchema({
        cwd: testDir,
      });

      // Then the JSON Schema validation should pass
      expect(result.success).toBe(true);

      // And Mermaid syntax validation should be handled separately by mermaid.parse()
      // (This test verifies JSON Schema doesn't validate Mermaid syntax)
    });
  });

  describe('Scenario: AI agent workflow - read schema, update foundation, validate', () => {
    it('should support complete workflow: read schema, update, validate', async () => {
      // Given I am an AI agent working on updating foundation.json
      // When I run "fspec show-foundation-schema"
      const schemaResult = await showFoundationSchema({
        cwd: testDir,
      });

      // Then I receive the schema with rich descriptions and examples
      expect(schemaResult.success).toBe(true);
      const schema = JSON.parse(schemaResult.output!);
      expect(schema.properties.whatWeAreBuilding.properties.projectOverview.description).toBeDefined();
      expect(schema.properties.whatWeAreBuilding.properties.projectOverview.examples).toBeDefined();

      // When I use the schema guidance to update foundation.json with "fspec update-foundation"
      const validFoundationData = {
        $schema: '../src/schemas/foundation.schema.json',
        project: {
          name: 'My Project',
          description: 'A test project',
          repository: 'https://github.com/test/test',
          license: 'MIT',
          importantNote: 'This is a legitimate project',
        },
        whatWeAreBuilding: {
          projectOverview:
            'A CLI tool that provides AI agents with structured workflow for building software using ACDD.',
          technicalRequirements: {
            coreTechnologies: [],
            architecture: {
              pattern: 'CLI',
              fileStructure: 'standard',
              deploymentTarget: 'local',
              integrationModel: [],
            },
            developmentAndOperations: {
              developmentTools: 'TypeScript',
              testingStrategy: 'Vitest',
              logging: 'console',
              validation: 'Gherkin parser',
              formatting: 'Prettier',
            },
            keyLibraries: [],
          },
          nonFunctionalRequirements: [],
        },
        whyWeAreBuildingIt: {
          problemDefinition: {
            primary: {
              title: 'AI Agents Lack Structure',
              description: 'AI agents struggle without workflow',
              coreProblems: [
                'Context fragility',
                'Workflow chaos',
                'No discovery phase',
              ],
            },
          },
          painPoints: {
            beforeFspec: 'No structure',
            afterFspec: 'Clear workflow',
          },
          stakeholderImpact: [],
          whyACDD: {
            title: 'Why ACDD',
            description: 'ACDD enforces workflow',
            specificationByExample: {
              principle: 'Concrete examples',
              benefit: 'Testable specs',
            },
            bdd: {
              principle: 'Given/When/Then',
              benefit: 'Shared language',
            },
            acdd: {
              principle: 'Specs first',
              benefit: 'No over-implementation',
              challenge: 'AI agents need tooling',
            },
          },
          successCriteria: [],
        },
        architectureDiagrams: [],
      };

      await writeFile(
        join(testDir, 'spec', 'foundation.json'),
        JSON.stringify(validFoundationData, null, 2)
      );

      // And I run "fspec validate-foundation-schema"
      const validateResult = await validateFoundationSchema({
        cwd: testDir,
      });

      // Then the validation should pass
      expect(validateResult.success).toBe(true);

      // And I should have confidence the documentation follows the expected format
      expect(validateResult.output).toContain('valid');
    });
  });
});
