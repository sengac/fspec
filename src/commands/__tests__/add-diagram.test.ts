import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile, access } from 'fs/promises';
import { join } from 'path';
import { addDiagram } from '../add-diagram';

describe('Feature: Add Mermaid Diagram to FOUNDATION.md', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-add-diagram');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Add new diagram to existing section', () => {
    it('should add diagram to existing section', async () => {
      // Given I have a foundation.json with existing data
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = {
        $schema: '../src/schemas/foundation.schema.json',
        project: {
          name: 'Test Project',
          description: 'Test',
          repository: 'https://test.com',
          license: 'MIT',
          importantNote: 'Test note',
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
        foundationJsonPath,
        JSON.stringify(minimalFoundation, null, 2)
      );

      // When I run `fspec add-diagram Architecture "Component Diagram" "graph TD\n  A-->B"`
      const result = await addDiagram({
        section: 'Architecture',
        title: 'Component Diagram',
        code: 'graph TD\n  A-->B',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the foundation.json should contain the new diagram
      const updatedJson = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );
      const diagram = updatedJson.architectureDiagrams.find(
        (d: any) => d.title === 'Component Diagram'
      );
      expect(diagram).toBeDefined();
      expect(diagram.mermaidCode).toBe('graph TD\n  A-->B');

      // And FOUNDATION.md should be regenerated with the diagram
      const updatedContent = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );
      expect(updatedContent).toContain('Component Diagram');
      expect(updatedContent).toContain('```mermaid');
      expect(updatedContent).toContain('graph TD');
      expect(updatedContent).toContain('A-->B');
    });
  });

  describe('Scenario: Add diagram to new section', () => {
    it('should add diagram to architectureDiagrams array', async () => {
      // Given I have a foundation.json with existing data
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = {
        $schema: '../src/schemas/foundation.schema.json',
        project: {
          name: 'Test Project',
          description: 'Test',
          repository: 'https://test.com',
          license: 'MIT',
          importantNote: 'Test note',
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
        foundationJsonPath,
        JSON.stringify(minimalFoundation, null, 2)
      );

      // When I run `fspec add-diagram "Data Flow" "User Login Flow" "sequenceDiagram\n  User->>Server: Login"`
      const result = await addDiagram({
        section: 'Data Flow',
        title: 'User Login Flow',
        code: 'sequenceDiagram\n  User->>Server: Login',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the foundation.json should contain the new diagram
      const updatedJson = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );
      const diagram = updatedJson.architectureDiagrams.find(
        (d: any) => d.title === 'User Login Flow'
      );
      expect(diagram).toBeDefined();
      expect(diagram.mermaidCode).toBe('sequenceDiagram\n  User->>Server: Login');

      // And FOUNDATION.md should be regenerated with the diagram
      const updatedContent = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );
      expect(updatedContent).toContain('User Login Flow');
      expect(updatedContent).toContain('sequenceDiagram');
    });
  });

  describe('Scenario: Replace existing diagram with same title', () => {
    it('should replace existing diagram', async () => {
      // Given I have a foundation.json with an existing diagram
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = {
        $schema: '../src/schemas/foundation.schema.json',
        project: {
          name: 'Test Project',
          description: 'Test',
          repository: 'https://test.com',
          license: 'MIT',
          importantNote: 'Test note',
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
            title: 'System Overview',
            mermaidCode: 'graph TD\n  Old-->Content',
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
        foundationJsonPath,
        JSON.stringify(minimalFoundation, null, 2)
      );

      // When I run `fspec add-diagram Architecture "System Overview" "graph LR\n  New-->Diagram"`
      const result = await addDiagram({
        section: 'Architecture',
        title: 'System Overview',
        code: 'graph LR\n  New-->Diagram',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the foundation.json should have the updated diagram
      const updatedJson = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );
      const diagram = updatedJson.architectureDiagrams.find(
        (d: any) => d.title === 'System Overview'
      );
      expect(diagram).toBeDefined();
      expect(diagram.mermaidCode).toBe('graph LR\n  New-->Diagram');

      // And FOUNDATION.md should be regenerated with new content
      const updatedContent = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );
      expect(updatedContent).toContain('New-->Diagram');
      expect(updatedContent).not.toContain('Old-->Content');
    });
  });

  describe('Scenario: Add multiple diagrams to same section', () => {
    it('should add multiple diagrams', async () => {
      // Given I have a foundation.json with an existing diagram
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = {
        $schema: '../src/schemas/foundation.schema.json',
        project: {
          name: 'Test Project',
          description: 'Test',
          repository: 'https://test.com',
          license: 'MIT',
          importantNote: 'Test note',
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
            title: 'Diagram 1',
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
        foundationJsonPath,
        JSON.stringify(minimalFoundation, null, 2)
      );

      // When I run `fspec add-diagram Architecture "Diagram 2" "graph TD\n  C-->D"`
      const result = await addDiagram({
        section: 'Architecture',
        title: 'Diagram 2',
        code: 'graph TD\n  C-->D',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the foundation.json should contain both diagrams
      const updatedJson = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );
      expect(updatedJson.architectureDiagrams.length).toBe(2);

      const diagram1 = updatedJson.architectureDiagrams.find(
        (d: any) => d.title === 'Diagram 1'
      );
      expect(diagram1).toBeDefined();
      expect(diagram1.mermaidCode).toBe('graph TD\n  A-->B');

      const diagram2 = updatedJson.architectureDiagrams.find(
        (d: any) => d.title === 'Diagram 2'
      );
      expect(diagram2).toBeDefined();
      expect(diagram2.mermaidCode).toBe('graph TD\n  C-->D');

      // And FOUNDATION.md should be regenerated with both diagrams
      const updatedContent = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );
      expect(updatedContent).toContain('Diagram 1');
      expect(updatedContent).toContain('A-->B');
      expect(updatedContent).toContain('Diagram 2');
      expect(updatedContent).toContain('C-->D');
    });
  });

  describe("Scenario: Create FOUNDATION.md if it doesn't exist", () => {
    it('should create FOUNDATION.md', async () => {
      // Given I have no FOUNDATION.md file
      // When I run `fspec add-diagram Architecture "Initial Diagram" "graph TD\n  Start-->End"`
      const result = await addDiagram({
        section: 'Architecture',
        title: 'Initial Diagram',
        code: 'graph TD\n  Start-->End',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And a FOUNDATION.md file should be created
      await expect(
        access(join(testDir, 'spec/FOUNDATION.md'))
      ).resolves.toBeUndefined();

      const content = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );

      // And it should contain the "Architecture Diagrams" section with the diagram
      expect(content).toContain('## 3. Architecture Diagrams');
      expect(content).toContain('### Initial Diagram');
    });
  });

  describe('Scenario: Preserve existing FOUNDATION.md sections', () => {
    it('should preserve foundation.json data and add new diagram', async () => {
      // Given I have a foundation.json with existing content
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = {
        $schema: '../src/schemas/foundation.schema.json',
        project: {
          name: 'Test Project',
          description: 'Test',
          repository: 'https://test.com',
          license: 'MIT',
          importantNote: 'Test note',
        },
        whatWeAreBuilding: {
          projectOverview: 'This is what we are building.',
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
              title: 'Why Problem',
              description: 'This is why.',
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
        foundationJsonPath,
        JSON.stringify(minimalFoundation, null, 2)
      );

      // When I run `fspec add-diagram Architecture "Diagram" "graph TD\n  A-->B"`
      const result = await addDiagram({
        section: 'Architecture',
        title: 'Diagram',
        code: 'graph TD\n  A-->B',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the foundation.json should preserve existing data and add the new diagram
      const updatedJson = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );
      expect(updatedJson.whatWeAreBuilding.projectOverview).toBe(
        'This is what we are building.'
      );
      expect(updatedJson.whyWeAreBuildingIt.problemDefinition.primary.description).toBe(
        'This is why.'
      );
      const diagram = updatedJson.architectureDiagrams.find(
        (d: any) => d.title === 'Diagram'
      );
      expect(diagram).toBeDefined();
      expect(diagram.mermaidCode).toBe('graph TD\n  A-->B');

      // And FOUNDATION.md should be regenerated with all sections
      const updatedContent = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );
      expect(updatedContent).toContain('## 1. What We Are Building');
      expect(updatedContent).toContain('This is what we are building.');
      expect(updatedContent).toContain('## 2. Why We Are Building It');
      expect(updatedContent).toContain('This is why.');
      expect(updatedContent).toContain('## 3. Architecture Diagrams');
      expect(updatedContent).toContain('Diagram');
    });
  });

  describe('Scenario: Support different Mermaid diagram types', () => {
    it('should support sequenceDiagram', async () => {
      // Given I have a FOUNDATION.md
      await writeFile(join(testDir, 'spec/FOUNDATION.md'), '# Foundation\n');

      // When I run `fspec add-diagram Flows "Sequence Diagram" "sequenceDiagram\n  A->>B: Message"`
      const result = await addDiagram({
        section: 'Flows',
        title: 'Sequence Diagram',
        code: 'sequenceDiagram\n  A->>B: Message',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const content = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );

      // And the diagram should use sequenceDiagram syntax
      expect(content).toContain('sequenceDiagram');
    });
  });

  describe('Scenario: Add class diagram', () => {
    it('should support classDiagram', async () => {
      // Given I have a FOUNDATION.md
      await writeFile(join(testDir, 'spec/FOUNDATION.md'), '# Foundation\n');

      // When I run `fspec add-diagram "Class Structure" "Domain Model" "classDiagram\n  Class01 <|-- Class02"`
      const result = await addDiagram({
        section: 'Class Structure',
        title: 'Domain Model',
        code: 'classDiagram\n  Class01 <|-- Class02',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const content = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );

      // And the diagram should use classDiagram syntax
      expect(content).toContain('classDiagram');
    });
  });

  describe('Scenario: Reject empty diagram code', () => {
    it('should reject empty code', async () => {
      // Given I have a FOUNDATION.md
      await writeFile(join(testDir, 'spec/FOUNDATION.md'), '# Foundation\n');

      // When I run `fspec add-diagram Architecture "Empty" ""`
      const result = await addDiagram({
        section: 'Architecture',
        title: 'Empty',
        code: '',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Diagram code cannot be empty"
      expect(result.error).toMatch(/diagram code cannot be empty/i);
    });
  });

  describe('Scenario: Reject empty diagram title', () => {
    it('should reject empty title', async () => {
      // Given I have a FOUNDATION.md
      await writeFile(join(testDir, 'spec/FOUNDATION.md'), '# Foundation\n');

      // When I run `fspec add-diagram Architecture "" "graph TD\n  A-->B"`
      const result = await addDiagram({
        section: 'Architecture',
        title: '',
        code: 'graph TD\n  A-->B',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Diagram title cannot be empty"
      expect(result.error).toMatch(/diagram title cannot be empty/i);
    });
  });

  describe('Scenario: Reject empty section name', () => {
    it('should reject empty section', async () => {
      // Given I have a FOUNDATION.md
      await writeFile(join(testDir, 'spec/FOUNDATION.md'), '# Foundation\n');

      // When I run `fspec add-diagram "" "Title" "graph TD\n  A-->B"`
      const result = await addDiagram({
        section: '',
        title: 'Title',
        code: 'graph TD\n  A-->B',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Section name cannot be empty"
      expect(result.error).toMatch(/section name cannot be empty/i);
    });
  });

  describe('Scenario: Format diagram with proper markdown', () => {
    it('should format diagram correctly', async () => {
      // Given I have a FOUNDATION.md
      await writeFile(join(testDir, 'spec/FOUNDATION.md'), '# Foundation\n');

      // When I run `fspec add-diagram Architecture "Flow" "graph TD\n  A-->B"`
      const result = await addDiagram({
        section: 'Architecture',
        title: 'Flow',
        code: 'graph TD\n  A-->B',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const content = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );

      // And the diagram should be formatted properly
      expect(content).toContain('### Flow');
      expect(content).toContain('```mermaid');
      expect(content).toContain('```');
    });
  });

  describe('Scenario: Handle multi-line diagram code', () => {
    it('should preserve all diagram lines', async () => {
      // Given I have a FOUNDATION.md
      await writeFile(join(testDir, 'spec/FOUNDATION.md'), '# Foundation\n');

      // When I run `fspec add-diagram Architecture "Complex" "graph TD\n  A-->B\n  B-->C\n  C-->D"`
      const result = await addDiagram({
        section: 'Architecture',
        title: 'Complex',
        code: 'graph TD\n  A-->B\n  B-->C\n  C-->D',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const content = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );

      // And all diagram lines should be preserved
      expect(content).toContain('A-->B');
      expect(content).toContain('B-->C');
      expect(content).toContain('C-->D');
    });
  });

  describe('Scenario: JSON-backed workflow - modify JSON and regenerate MD', () => {
    it('should update foundation.json and regenerate FOUNDATION.md', async () => {
      // Given I have a valid foundation.json file
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = {
        $schema: '../src/schemas/foundation.schema.json',
        project: {
          name: 'Test Project',
          description: 'Test',
          repository: 'https://test.com',
          license: 'MIT',
          importantNote: 'Test note',
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
            title: 'Existing Diagram',
            mermaidCode: 'graph TD\n  X-->Y',
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
        foundationJsonPath,
        JSON.stringify(minimalFoundation, null, 2)
      );

      // When I run `fspec add-diagram "Architecture Diagrams" "New System Diagram" "graph TD\n  A-->B"`
      const result = await addDiagram({
        section: 'Architecture Diagrams',
        title: 'New System Diagram',
        code: 'graph TD\n  A-->B',
        cwd: testDir,
      });

      // Then the foundation.json file should be updated with the new diagram
      const updatedFoundationJson = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );

      // And the foundation.json should validate against foundation.schema.json
      expect(updatedFoundationJson.architectureDiagrams).toBeDefined();

      // And the new diagram should be in the architectureDiagrams array
      const newDiagram = updatedFoundationJson.architectureDiagrams.find(
        (d: any) => d.title === 'New System Diagram'
      );
      expect(newDiagram).toBeDefined();

      // And the diagram object should have section, title, and mermaidCode fields
      expect(newDiagram.section).toBe('Architecture Diagrams');
      expect(newDiagram.title).toBe('New System Diagram');
      expect(newDiagram.mermaidCode).toBe('graph TD\n  A-->B');

      // And existing diagrams should be preserved
      const existingDiagram = updatedFoundationJson.architectureDiagrams.find(
        (d: any) => d.title === 'Existing Diagram'
      );
      expect(existingDiagram).toBeDefined();

      // And FOUNDATION.md should be regenerated from foundation.json
      const foundationContent = await readFile(
        join(testDir, 'spec', 'FOUNDATION.md'),
        'utf-8'
      );

      // And FOUNDATION.md should contain the new diagram in a mermaid code block
      expect(foundationContent).toContain('New System Diagram');
      expect(foundationContent).toContain('```mermaid');
      expect(foundationContent).toContain('graph TD');
      expect(foundationContent).toContain('A-->B');

      // And FOUNDATION.md should have the auto-generation warning header
      expect(foundationContent).toContain(
        '<!-- THIS FILE IS AUTO-GENERATED FROM spec/foundation.json -->'
      );
      expect(foundationContent).toContain('<!-- DO NOT EDIT THIS FILE DIRECTLY -->');

      expect(result.success).toBe(true);
    });
  });
});
