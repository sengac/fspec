/**
 * Feature: spec/features/validate-mermaid-diagrams-during-foundation-regeneration.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile, access } from 'fs/promises';
import { join } from 'path';
import { generateFoundationMdCommand } from '../generate-foundation-md';
import type { Foundation } from '../../types/foundation';

describe('Feature: Validate Mermaid Diagrams During Foundation Regeneration', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(
      process.cwd(),
      'test-tmp-generate-foundation-diagram-validation'
    );
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  // Helper to create minimal valid foundation
  const createMinimalFoundation = (): Foundation => ({
    $schema: '../src/schemas/foundation.schema.json',
    project: {
      name: 'Test Project',
      description: 'Test description',
      repository: 'https://github.com/test/repo',
      license: 'MIT',
      importantNote: 'Test note',
    },
    whatWeAreBuilding: {
      projectOverview: 'Test overview',
      technicalRequirements: {
        coreTechnologies: [],
        architecture: {
          pattern: 'Test pattern',
          fileStructure: 'Test structure',
          deploymentTarget: 'Test target',
          integrationModel: [],
        },
        developmentAndOperations: {
          developmentTools: 'Test tools',
          testingStrategy: 'Test strategy',
          logging: 'Test logging',
          validation: 'Test validation',
          formatting: 'Test formatting',
        },
        keyLibraries: [],
      },
      nonFunctionalRequirements: [],
    },
    whyWeAreBuildingIt: {
      problemDefinition: {
        primary: {
          title: 'Test Problem',
          description: 'Test description',
          points: [],
        },
        secondary: [],
      },
      painPoints: {
        currentState: 'Test state',
        specific: [],
      },
      stakeholderImpact: [],
      theoreticalSolutions: [],
      developmentMethodology: {
        name: 'Test Methodology',
        description: 'Test description',
        steps: [],
        ensures: [],
      },
      successCriteria: [],
      constraintsAndAssumptions: {
        constraints: [],
        assumptions: [],
      },
    },
    architectureDiagrams: [],
    coreCommands: {
      categories: [],
    },
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
    notes: {
      developmentStatus: [],
    },
  });

  describe('Scenario: Generate FOUNDATION.md with all valid diagrams', () => {
    it('should exit with code 0 and create FOUNDATION.md when all diagrams are valid', async () => {
      // Given I have a valid file "spec/foundation.json" with 3 Mermaid diagrams
      const foundation = createMinimalFoundation();
      foundation.architectureDiagrams = [
        {
          section: 'Architecture',
          title: 'System Context',
          mermaidCode: 'graph TB\n  A[AI Agent]\n  B[fspec CLI]',
        },
        {
          section: 'Architecture',
          title: 'Data Flow',
          mermaidCode: 'graph LR\n  JSON[foundation.json]\n  MD[FOUNDATION.md]',
        },
        {
          section: 'Architecture',
          title: 'Components',
          mermaidCode: 'graph TD\n  CLI-->Parser\n  Parser-->Generator',
        },
      ];

      // And all diagrams have valid Mermaid syntax
      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(foundation, null, 2)
      );

      // When I run `fspec generate-foundation-md`
      const result = await generateFoundationMdCommand({ cwd: testDir });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the file "spec/FOUNDATION.md" should be created
      const foundationMd = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );
      expect(foundationMd).toBeDefined();

      // And the output should display "✓ Generated spec/FOUNDATION.md from spec/foundation.json"
      expect(result.message).toContain('Generated spec/FOUNDATION.md');
    });
  });

  describe('Scenario: Fail generation with detailed error for single invalid diagram', () => {
    it('should exit with code 1 and show detailed error for single invalid diagram', async () => {
      // Given I have a file "spec/foundation.json" with 3 diagrams
      const foundation = createMinimalFoundation();
      foundation.architectureDiagrams = [
        {
          section: 'Architecture',
          title: 'Valid Diagram',
          mermaidCode: 'graph TB\n  A-->B',
        },
        {
          section: 'Architecture',
          title: 'Data Flow',
          // And the diagram at architectureDiagrams[1] with title "Data Flow" has invalid Mermaid syntax
          mermaidCode: 'invalid mermaid syntax here @#$%',
        },
        {
          section: 'Architecture',
          title: 'Another Valid',
          mermaidCode: 'graph LR\n  C-->D',
        },
      ];

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(foundation, null, 2)
      );

      // When I run `fspec generate-foundation-md`
      const result = await generateFoundationMdCommand({ cwd: testDir });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should display "Diagram validation failed" in red
      expect(result.error).toContain('Diagram validation failed');

      // And the output should show the diagram position "architectureDiagrams[1]"
      expect(result.error).toContain('architectureDiagrams[1]');

      // And the output should show the diagram title "Data Flow"
      expect(result.error).toContain('Data Flow');

      // And the output should show the detailed Mermaid error message
      expect(result.error).toBeDefined();

      // And the file "spec/FOUNDATION.md" should not be modified
      await expect(
        access(join(testDir, 'spec/FOUNDATION.md'))
      ).rejects.toThrow();
    });
  });

  describe('Scenario: Show all validation errors for multiple invalid diagrams', () => {
    it('should show all validation errors when multiple diagrams are invalid', async () => {
      // Given I have a file "spec/foundation.json" with 5 diagrams
      const foundation = createMinimalFoundation();
      foundation.architectureDiagrams = [
        {
          section: 'Architecture',
          title: 'Valid 1',
          mermaidCode: 'graph TB\n  A-->B',
        },
        {
          section: 'Architecture',
          title: 'Invalid 1',
          // And diagrams at architectureDiagrams[1] and architectureDiagrams[3] have invalid syntax
          mermaidCode: 'bad syntax @#$',
        },
        {
          section: 'Architecture',
          title: 'Valid 2',
          mermaidCode: 'graph LR\n  C-->D',
        },
        {
          section: 'Architecture',
          title: 'Invalid 2',
          mermaidCode: 'also bad !@#$%',
        },
        {
          section: 'Architecture',
          title: 'Valid 3',
          mermaidCode: 'graph TD\n  E-->F',
        },
      ];

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(foundation, null, 2)
      );

      // When I run `fspec generate-foundation-md`
      const result = await generateFoundationMdCommand({ cwd: testDir });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should display "Found 2 invalid diagrams"
      expect(result.error).toContain('Found 2 invalid diagram');

      // And the output should show errors for BOTH architectureDiagrams[1] and architectureDiagrams[3]
      expect(result.error).toContain('architectureDiagrams[1]');
      expect(result.error).toContain('architectureDiagrams[3]');

      // And each error should include position, title, and Mermaid error message
      expect(result.error).toContain('Invalid 1');
      expect(result.error).toContain('Invalid 2');

      // And the file "spec/FOUNDATION.md" should not be modified
      await expect(
        access(join(testDir, 'spec/FOUNDATION.md'))
      ).rejects.toThrow();
    });
  });

  describe('Scenario: Error message includes guidance to fix and regenerate', () => {
    it('should include guidance in error message', async () => {
      // Given I have a file "spec/foundation.json" with an invalid diagram
      const foundation = createMinimalFoundation();
      foundation.architectureDiagrams = [
        {
          section: 'Architecture',
          title: 'Bad Diagram',
          mermaidCode: 'invalid syntax',
        },
      ];

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(foundation, null, 2)
      );

      // When I run `fspec generate-foundation-md`
      const result = await generateFoundationMdCommand({ cwd: testDir });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should display "Fix the diagram(s) in spec/foundation.json"
      expect(result.error).toContain('Fix the diagram(s)');
      expect(result.error).toContain('spec/foundation.json');

      // And the output should display "Run 'fspec generate-foundation-md' again after fixing"
      expect(result.error).toContain('generate-foundation-md');
      expect(result.error).toContain('after fixing');
    });
  });

  describe('Scenario: Successful regeneration after fixing invalid diagrams', () => {
    it('should succeed after fixing invalid diagrams', async () => {
      // Given I previously failed generation due to invalid diagrams
      const invalidFoundation = createMinimalFoundation();
      invalidFoundation.architectureDiagrams = [
        {
          section: 'Architecture',
          title: 'Bad Diagram',
          mermaidCode: 'invalid syntax',
        },
      ];

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(invalidFoundation, null, 2)
      );

      const firstResult = await generateFoundationMdCommand({ cwd: testDir });
      expect(firstResult.success).toBe(false);

      // And I have now fixed all diagram syntax errors in "spec/foundation.json"
      const fixedFoundation = createMinimalFoundation();
      fixedFoundation.architectureDiagrams = [
        {
          section: 'Architecture',
          title: 'Good Diagram',
          mermaidCode: 'graph TB\n  A-->B',
        },
      ];

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(fixedFoundation, null, 2)
      );

      // When I run `fspec generate-foundation-md` again
      const secondResult = await generateFoundationMdCommand({ cwd: testDir });

      // Then the command should exit with code 0
      expect(secondResult.success).toBe(true);

      // And the file "spec/FOUNDATION.md" should be created
      const foundationMd = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );
      expect(foundationMd).toBeDefined();

      // And the output should display "✓ Generated spec/FOUNDATION.md from spec/foundation.json"
      expect(secondResult.message).toContain('Generated spec/FOUNDATION.md');
    });
  });
});
