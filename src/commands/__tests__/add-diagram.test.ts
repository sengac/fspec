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
      // Given I have a FOUNDATION.md with an "Architecture" section
      const content = `# Project Foundation

## Architecture

This is the architecture section.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec add-diagram Architecture "Component Diagram" "graph TD\n  A-->B"`
      const result = await addDiagram({
        section: 'Architecture',
        title: 'Component Diagram',
        code: 'graph TD\n  A-->B',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(join(testDir, 'spec/FOUNDATION.md'), 'utf-8');

      // And the "Architecture" section should contain a diagram titled "Component Diagram"
      expect(updatedContent).toContain('### Component Diagram');

      // And the diagram should be in a mermaid code block
      expect(updatedContent).toContain('```mermaid');
      expect(updatedContent).toContain('graph TD');
      expect(updatedContent).toContain('A-->B');

      // And other content in the section should be preserved
      expect(updatedContent).toContain('This is the architecture section.');
    });
  });

  describe('Scenario: Add diagram to new section', () => {
    it('should create new section with diagram', async () => {
      // Given I have a FOUNDATION.md without a "Data Flow" section
      const content = `# Project Foundation

## Architecture

Some content.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec add-diagram "Data Flow" "User Login Flow" "sequenceDiagram\n  User->>Server: Login"`
      const result = await addDiagram({
        section: 'Data Flow',
        title: 'User Login Flow',
        code: 'sequenceDiagram\n  User->>Server: Login',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(join(testDir, 'spec/FOUNDATION.md'), 'utf-8');

      // And a new "Data Flow" section should be created
      expect(updatedContent).toContain('## Data Flow');

      // And the section should contain the diagram
      expect(updatedContent).toContain('### User Login Flow');
      expect(updatedContent).toContain('sequenceDiagram');
    });
  });

  describe('Scenario: Replace existing diagram with same title', () => {
    it('should replace existing diagram', async () => {
      // Given I have a diagram titled "System Overview" in the "Architecture" section
      const content = `# Project Foundation

## Architecture

### System Overview

\`\`\`mermaid
graph TD
  Old-->Content
\`\`\`
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec add-diagram Architecture "System Overview" "graph LR\n  New-->Diagram"`
      const result = await addDiagram({
        section: 'Architecture',
        title: 'System Overview',
        code: 'graph LR\n  New-->Diagram',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(join(testDir, 'spec/FOUNDATION.md'), 'utf-8');

      // And the "System Overview" diagram should be updated
      expect(updatedContent).toContain('New-->Diagram');

      // And the old diagram content should be replaced
      expect(updatedContent).not.toContain('Old-->Content');
    });
  });

  describe('Scenario: Add multiple diagrams to same section', () => {
    it('should add multiple diagrams', async () => {
      // Given I have a diagram "Diagram 1" in the "Architecture" section
      const content = `# Project Foundation

## Architecture

### Diagram 1

\`\`\`mermaid
graph TD
  A-->B
\`\`\`
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec add-diagram Architecture "Diagram 2" "graph TD\n  C-->D"`
      const result = await addDiagram({
        section: 'Architecture',
        title: 'Diagram 2',
        code: 'graph TD\n  C-->D',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(join(testDir, 'spec/FOUNDATION.md'), 'utf-8');

      // And the "Architecture" section should contain both diagrams
      expect(updatedContent).toContain('### Diagram 1');
      expect(updatedContent).toContain('### Diagram 2');

      // And both "Diagram 1" and "Diagram 2" should be present
      expect(updatedContent).toContain('A-->B');
      expect(updatedContent).toContain('C-->D');
    });
  });

  describe('Scenario: Create FOUNDATION.md if it doesn\'t exist', () => {
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
      await expect(access(join(testDir, 'spec/FOUNDATION.md'))).resolves.toBeUndefined();

      const content = await readFile(join(testDir, 'spec/FOUNDATION.md'), 'utf-8');

      // And it should contain the "Architecture" section with the diagram
      expect(content).toContain('## Architecture');
      expect(content).toContain('### Initial Diagram');
    });
  });

  describe('Scenario: Preserve existing FOUNDATION.md sections', () => {
    it('should preserve other sections', async () => {
      // Given I have a FOUNDATION.md with sections "What We Are Building" and "Why"
      const content = `# Project Foundation

## What We Are Building

This is what we are building.

## Why

This is why.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec add-diagram Architecture "Diagram" "graph TD\n  A-->B"`
      const result = await addDiagram({
        section: 'Architecture',
        title: 'Diagram',
        code: 'graph TD\n  A-->B',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(join(testDir, 'spec/FOUNDATION.md'), 'utf-8');

      // And the "What We Are Building" section should be preserved
      expect(updatedContent).toContain('## What We Are Building');
      expect(updatedContent).toContain('This is what we are building.');

      // And the "Why" section should be preserved
      expect(updatedContent).toContain('## Why');
      expect(updatedContent).toContain('This is why.');

      // And the "Architecture" section should be added
      expect(updatedContent).toContain('## Architecture');
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

      const content = await readFile(join(testDir, 'spec/FOUNDATION.md'), 'utf-8');

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

      const content = await readFile(join(testDir, 'spec/FOUNDATION.md'), 'utf-8');

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

      const content = await readFile(join(testDir, 'spec/FOUNDATION.md'), 'utf-8');

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

      const content = await readFile(join(testDir, 'spec/FOUNDATION.md'), 'utf-8');

      // And all diagram lines should be preserved
      expect(content).toContain('A-->B');
      expect(content).toContain('B-->C');
      expect(content).toContain('C-->D');
    });
  });
});
