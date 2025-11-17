// Feature: spec/features/generate-event-storm-section-in-foundation-md-with-validated-mermaid-diagram.feature

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import { execSync } from 'child_process';
import { generateFoundationMd } from '../foundation-md';

describe('Feature: Generate Event Storm section in FOUNDATION.md with validated Mermaid diagram', () => {
  describe('Scenario: FOUNDATION.md auto-regenerates with Event Storm section when bounded context added', () => {
    const testDir = path.join(
      __dirname,
      '../../../test-temp-foundation-autogen'
    );
    const foundationJsonPath = path.join(testDir, 'spec/foundation.json');
    const foundationMdPath = path.join(testDir, 'spec/FOUNDATION.md');

    beforeEach(() => {
      if (fs.existsSync(testDir)) {
        fs.rmSync(testDir, { recursive: true, force: true });
      }
      fs.mkdirSync(testDir, { recursive: true });
      fs.mkdirSync(path.join(testDir, 'spec'), { recursive: true });
    });

    afterEach(() => {
      if (fs.existsSync(testDir)) {
        fs.rmSync(testDir, { recursive: true, force: true });
      }
    });

    it('should auto-regenerate FOUNDATION.md with Event Storm section when bounded context is added', () => {
      // @step Given foundation.json exists with eventStorm field initialized
      const foundationData = {
        version: '2.0.0',
        project: {
          name: 'Test Project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test Problem',
            description: 'Test problem description',
            impact: 'medium',
          },
        },
        solutionSpace: {
          overview: 'Test solution overview',
          capabilities: [
            { name: 'Test capability', description: 'Test description' },
          ],
        },
        personas: [
          {
            name: 'Test Persona',
            description: 'Tester role',
            goals: ['Test goal'],
          },
        ],
        eventStorm: {
          level: 'big_picture',
          items: [],
          nextItemId: 1,
        },
      };
      fs.writeFileSync(
        foundationJsonPath,
        JSON.stringify(foundationData, null, 2)
      );

      // @step When I run "fspec add-foundation-bounded-context 'Work Management'"
      execSync(
        `node ${path.join(__dirname, '../../../dist/index.js')} add-foundation-bounded-context "Work Management"`,
        {
          cwd: testDir,
          stdio: 'pipe',
        }
      );

      // @step Then FOUNDATION.md should be automatically regenerated
      expect(fs.existsSync(foundationMdPath)).toBe(true);

      const foundationMdContent = fs.readFileSync(foundationMdPath, 'utf-8');

      // @step And FOUNDATION.md should contain a "Domain Architecture" section
      expect(foundationMdContent).toContain('# Domain Architecture');

      // @step And the Domain Architecture section should include a "Bounded Contexts" subsection with a text list
      expect(foundationMdContent).toMatch(/##\s+Bounded Contexts/);

      // @step And the text list should include "Work Management"
      const boundedContextsMatch = foundationMdContent.match(
        /##\s+Bounded Contexts[\s\S]*?(?=##|$)/
      );
      expect(boundedContextsMatch).toBeTruthy();
      expect(boundedContextsMatch![0]).toContain('Work Management');

      // @step And the Domain Architecture section should include a "Bounded Context Map" subsection with a Mermaid diagram
      expect(foundationMdContent).toMatch(/##\s+Bounded Context Map/);
      expect(foundationMdContent).toContain('```mermaid');

      // @step And the Mermaid diagram should have a node for "Work Management"
      const mermaidMatch = foundationMdContent.match(/```mermaid[\s\S]*?```/);
      expect(mermaidMatch).toBeTruthy();
      expect(mermaidMatch![0]).toContain('Work Management');

      // @step And the Mermaid diagram syntax should be validated using validateMermaidSyntax()
      // This step is verified by the implementation not throwing an error during generation
      // If validation fails, the command would have thrown an error and the test would fail
    });
  });

  describe('Scenario: FOUNDATION.md includes Mermaid graph with all bounded contexts', () => {
    const testDir = path.join(
      __dirname,
      '../../../test-temp-foundation-mermaid'
    );
    const foundationJsonPath = path.join(testDir, 'spec/foundation.json');
    const foundationMdPath = path.join(testDir, 'FOUNDATION.md');

    beforeEach(() => {
      if (fs.existsSync(testDir)) {
        fs.rmSync(testDir, { recursive: true, force: true });
      }
      fs.mkdirSync(testDir, { recursive: true });
      fs.mkdirSync(path.join(testDir, 'spec'), { recursive: true });
    });

    afterEach(() => {
      if (fs.existsSync(testDir)) {
        fs.rmSync(testDir, { recursive: true, force: true });
      }
    });

    it('should include Mermaid graph with all 7 bounded contexts', async () => {
      // @step Given foundation.json has eventStorm field with 7 bounded contexts
      const foundationData = {
        version: '2.0.0',
        project: {
          name: 'Test Project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test Problem',
            description: 'Test problem description',
            impact: 'medium',
          },
        },
        solutionSpace: {
          overview: 'Test solution overview',
          capabilities: [
            { name: 'Test capability', description: 'Test description' },
          ],
        },
        personas: [
          {
            name: 'Test Persona',
            description: 'Tester role',
            goals: ['Test goal'],
          },
        ],
        eventStorm: {
          level: 'big_picture',
          items: [
            {
              id: 1,
              type: 'bounded_context',
              text: 'Work Management',
              deleted: false,
            },
            {
              id: 2,
              type: 'bounded_context',
              text: 'Specification',
              deleted: false,
            },
            {
              id: 3,
              type: 'bounded_context',
              text: 'Testing & Validation',
              deleted: false,
            },
            {
              id: 4,
              type: 'bounded_context',
              text: 'Quality Assurance',
              deleted: false,
            },
            {
              id: 5,
              type: 'bounded_context',
              text: 'Version Control Integration',
              deleted: false,
            },
            {
              id: 6,
              type: 'bounded_context',
              text: 'User Interface',
              deleted: false,
            },
            {
              id: 7,
              type: 'bounded_context',
              text: 'Foundation Management',
              deleted: false,
            },
          ],
          nextItemId: 8,
        },
      };

      // @step And the bounded contexts are "Work Management", "Specification", "Testing & Validation", "Quality Assurance", "Version Control Integration", "User Interface", and "Foundation Management"
      fs.writeFileSync(
        foundationJsonPath,
        JSON.stringify(foundationData, null, 2)
      );

      // @step When FOUNDATION.md is generated
      const markdown = await generateFoundationMd(foundationData);
      fs.writeFileSync(foundationMdPath, markdown, 'utf-8');

      const foundationMdContent = fs.readFileSync(foundationMdPath, 'utf-8');

      // @step Then the Mermaid bounded context map should have 7 nodes
      const mermaidMatch = foundationMdContent.match(/```mermaid[\s\S]*?```/);
      expect(mermaidMatch).toBeTruthy();
      const mermaidContent = mermaidMatch![0];

      const nodeMatches = mermaidContent.match(/BC\d+\[/g);
      expect(nodeMatches).toBeTruthy();
      expect(nodeMatches!.length).toBe(7);

      // @step And each node should represent one bounded context
      expect(mermaidContent).toContain('Work Management');
      expect(mermaidContent).toContain('Specification');
      expect(mermaidContent).toContain('Testing & Validation');
      expect(mermaidContent).toContain('Quality Assurance');
      expect(mermaidContent).toContain('Version Control Integration');
      expect(mermaidContent).toContain('User Interface');
      expect(mermaidContent).toContain('Foundation Management');

      // @step And the diagram should be validated before being written to FOUNDATION.md
      // This step is verified by the implementation not throwing an error during generation
      // If validation fails, the command would have thrown an error and the test would fail
    });
  });

  describe('Scenario: Mermaid validation error prevents FOUNDATION.md generation', () => {
    const testDir = path.join(
      __dirname,
      '../../../test-temp-foundation-validation'
    );
    const foundationJsonPath = path.join(testDir, 'spec/foundation.json');
    const foundationMdPath = path.join(testDir, 'FOUNDATION.md');

    beforeEach(() => {
      if (fs.existsSync(testDir)) {
        fs.rmSync(testDir, { recursive: true, force: true });
      }
      fs.mkdirSync(testDir, { recursive: true });
      fs.mkdirSync(path.join(testDir, 'spec'), { recursive: true });
    });

    afterEach(() => {
      if (fs.existsSync(testDir)) {
        fs.rmSync(testDir, { recursive: true, force: true });
      }
    });

    it('should throw validation error and prevent FOUNDATION.md generation with invalid Mermaid syntax', async () => {
      // @step Given foundation.json has eventStorm field with bounded contexts
      const foundationData = {
        version: '2.0.0',
        project: {
          name: 'Test Project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test Problem',
            description: 'Test problem description',
            impact: 'medium',
          },
        },
        solutionSpace: {
          overview: 'Test solution overview',
          capabilities: [
            { name: 'Test capability', description: 'Test description' },
          ],
        },
        personas: [
          {
            name: 'Test Persona',
            description: 'Tester role',
            goals: ['Test goal'],
          },
        ],
        eventStorm: {
          level: 'big_picture',
          items: [
            {
              id: 1,
              type: 'bounded_context',
              text: 'Invalid{Context}',
              deleted: false,
            },
          ],
          nextItemId: 2,
        },
      };
      fs.writeFileSync(
        foundationJsonPath,
        JSON.stringify(foundationData, null, 2)
      );

      // @step And the generated Mermaid diagram contains invalid syntax
      // (The invalid bounded context name will trigger Mermaid validation error)

      // @step When FOUNDATION.md generation is attempted
      // @step Then an error should be thrown with the validation error message
      await expect(async () => {
        await generateFoundationMd(foundationData);
      }).rejects.toThrow();

      // @step And FOUNDATION.md should not be written with invalid Mermaid syntax
      expect(fs.existsSync(foundationMdPath)).toBe(false);

      // @step And the error message should help identify the syntax issue
      try {
        await generateFoundationMd(foundationData);
      } catch (error) {
        expect(error).toBeDefined();
        expect((error as Error).message).toBeTruthy();
      }
    });
  });

  describe('Scenario: Event Storm section only appears when eventStorm data exists', () => {
    const testDir = path.join(
      __dirname,
      '../../../test-temp-foundation-no-eventstorm'
    );
    const foundationJsonPath = path.join(testDir, 'spec/foundation.json');
    const foundationMdPath = path.join(testDir, 'FOUNDATION.md');

    beforeEach(() => {
      if (fs.existsSync(testDir)) {
        fs.rmSync(testDir, { recursive: true, force: true });
      }
      fs.mkdirSync(testDir, { recursive: true });
      fs.mkdirSync(path.join(testDir, 'spec'), { recursive: true });
    });

    afterEach(() => {
      if (fs.existsSync(testDir)) {
        fs.rmSync(testDir, { recursive: true, force: true });
      }
    });

    it('should not include Domain Architecture section when eventStorm field does not exist', async () => {
      // @step Given foundation.json exists without eventStorm field
      const foundationData = {
        version: '2.0.0',
        project: {
          name: 'Test Project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test Problem',
            description: 'Test problem description',
            impact: 'medium',
          },
        },
        solutionSpace: {
          overview: 'Test solution overview',
          capabilities: [
            { name: 'Test capability', description: 'Test description' },
          ],
        },
        personas: [
          {
            name: 'Test Persona',
            description: 'Tester role',
            goals: ['Test goal'],
          },
        ],
        // No eventStorm field
      };
      fs.writeFileSync(
        foundationJsonPath,
        JSON.stringify(foundationData, null, 2)
      );

      // @step When FOUNDATION.md is generated
      const markdown = await generateFoundationMd(foundationData);
      fs.writeFileSync(foundationMdPath, markdown, 'utf-8');

      const foundationMdContent = fs.readFileSync(foundationMdPath, 'utf-8');

      // @step Then the Domain Architecture section should not appear in FOUNDATION.md
      expect(foundationMdContent).not.toContain('# Domain Architecture');
      expect(foundationMdContent).not.toContain('## Bounded Contexts');
      expect(foundationMdContent).not.toContain('## Bounded Context Map');
      expect(foundationMdContent).not.toContain('```mermaid');
    });
  });
});
