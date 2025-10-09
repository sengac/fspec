import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { deleteDiagram } from '../delete-diagram';

describe('Feature: Delete Architecture Diagram from Foundation', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-delete-diagram');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Delete diagram by section and title', () => {
    it('should delete diagram and regenerate FOUNDATION.md', async () => {
      // Given I have a foundation.json with a diagram
      const foundationData = {
        $schema: '../src/schemas/foundation.schema.json',
        project: {
          name: 'Test Project',
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
              points: [],
            },
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
        architectureDiagrams: [
          {
            section: 'Architecture Diagrams',
            title: 'System Context',
            mermaidCode: 'graph TD\n  A-->B',
          },
        ],
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

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(foundationData, null, 2)
      );

      // When I run delete-diagram
      const result = await deleteDiagram({
        section: 'Architecture Diagrams',
        title: 'System Context',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the diagram should be removed from foundation.json
      const updatedContent = await readFile(
        join(testDir, 'spec/foundation.json'),
        'utf-8'
      );
      const updatedData = JSON.parse(updatedContent);
      expect(updatedData.architectureDiagrams).toHaveLength(0);

      // And the output should show success message
      expect(result.message).toContain('Deleted diagram');
      expect(result.message).toContain('System Context');
    });
  });

  describe('Scenario: Delete one of multiple diagrams', () => {
    it('should delete only the specified diagram', async () => {
      // Given I have a foundation.json with 3 diagrams
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
        architectureDiagrams: [
          {
            section: 'Architecture Diagrams',
            title: 'Diagram 1',
            mermaidCode: 'graph TD\n  A-->B',
          },
          {
            section: 'Architecture Diagrams',
            title: 'Diagram 2',
            mermaidCode: 'graph TD\n  C-->D',
          },
          {
            section: 'Architecture Diagrams',
            title: 'Diagram 3',
            mermaidCode: 'graph TD\n  E-->F',
          },
        ],
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

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(foundationData, null, 2)
      );

      // When I run delete-diagram for Diagram 2
      const result = await deleteDiagram({
        section: 'Architecture Diagrams',
        title: 'Diagram 2',
        cwd: testDir,
      });

      // Then only the specified diagram should be removed
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/foundation.json'),
        'utf-8'
      );
      const updatedData = JSON.parse(updatedContent);

      // And the other 2 diagrams should remain
      expect(updatedData.architectureDiagrams).toHaveLength(2);
      expect(updatedData.architectureDiagrams[0].title).toBe('Diagram 1');
      expect(updatedData.architectureDiagrams[1].title).toBe('Diagram 3');
    });
  });

  describe('Scenario: Handle non-existent diagram', () => {
    it('should error when diagram does not exist', async () => {
      // Given I have a foundation.json
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

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(foundationData, null, 2)
      );

      // When I run delete-diagram with non-existent diagram
      const result = await deleteDiagram({
        section: 'Architecture Diagrams',
        title: 'Non-Existent Diagram',
        cwd: testDir,
      });

      // Then the command should exit with error
      expect(result.success).toBe(false);
      expect(result.error).toContain('not found');
      expect(result.error).toContain('Non-Existent Diagram');
    });
  });

  describe('Scenario: Delete last diagram in section', () => {
    it('should leave architectureDiagrams array empty', async () => {
      // Given I have a foundation.json with only 1 diagram
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
        architectureDiagrams: [
          {
            section: 'Architecture Diagrams',
            title: 'Last Diagram',
            mermaidCode: 'graph TD\n  A-->B',
          },
        ],
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

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(foundationData, null, 2)
      );

      // When I run delete-diagram
      const result = await deleteDiagram({
        section: 'Architecture Diagrams',
        title: 'Last Diagram',
        cwd: testDir,
      });

      // Then the architectureDiagrams array should be empty
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/foundation.json'),
        'utf-8'
      );
      const updatedData = JSON.parse(updatedContent);
      expect(updatedData.architectureDiagrams).toHaveLength(0);
      expect(updatedData.architectureDiagrams).toEqual([]);
    });
  });

  describe('Scenario: Preserve other foundation.json sections', () => {
    it('should only modify architectureDiagrams array', async () => {
      // Given I have a foundation.json with project info and diagrams
      const foundationData = {
        $schema: '../src/schemas/foundation.schema.json',
        project: {
          name: 'Important Project',
          description: 'Important description',
          repository: 'https://test.com',
          license: 'MIT',
          importantNote: 'Important note',
        },
        whatWeAreBuilding: {
          projectOverview: 'Important overview',
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
              title: 'Important problem',
              description: 'Test',
              points: [],
            },
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
        architectureDiagrams: [
          {
            section: 'Architecture Diagrams',
            title: 'Test Diagram',
            mermaidCode: 'graph TD\n  A-->B',
          },
        ],
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

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(foundationData, null, 2)
      );

      // When I run delete-diagram
      const result = await deleteDiagram({
        section: 'Architecture Diagrams',
        title: 'Test Diagram',
        cwd: testDir,
      });

      // Then other sections should remain unchanged
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/foundation.json'),
        'utf-8'
      );
      const updatedData = JSON.parse(updatedContent);

      expect(updatedData.project.name).toBe('Important Project');
      expect(updatedData.project.description).toBe('Important description');
      expect(updatedData.whatWeAreBuilding.projectOverview).toBe(
        'Important overview'
      );
      expect(updatedData.whyWeAreBuildingIt.problemDefinition.primary.title).toBe(
        'Important problem'
      );
    });
  });

  describe('Scenario: JSON-backed workflow - delete from foundation.json', () => {
    it('should load, modify, and save foundation.json', async () => {
      // Given I have foundation.json with multiple diagrams
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
        architectureDiagrams: [
          {
            section: 'Architecture Diagrams',
            title: 'Current Diagram',
            mermaidCode: 'graph TD\n  A-->B',
          },
          {
            section: 'Architecture Diagrams',
            title: 'Outdated Diagram',
            mermaidCode: 'graph TD\n  C-->D',
          },
        ],
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

      await writeFile(
        join(testDir, 'spec/foundation.json'),
        JSON.stringify(foundationData, null, 2)
      );

      // When I run delete-diagram
      const result = await deleteDiagram({
        section: 'Architecture Diagrams',
        title: 'Outdated Diagram',
        cwd: testDir,
      });

      // Then the command should load and modify foundation.json
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/foundation.json'),
        'utf-8'
      );
      const updatedData = JSON.parse(updatedContent);

      // And remove the specified diagram
      expect(updatedData.architectureDiagrams).toHaveLength(1);
      expect(updatedData.architectureDiagrams[0].title).toBe('Current Diagram');

      // And the command should exit with code 0
      expect(result.success).toBe(true);
    });
  });
});
