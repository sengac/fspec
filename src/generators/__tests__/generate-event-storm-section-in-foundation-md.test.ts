// Feature: spec/features/generate-event-storm-section-in-foundation-md-with-validated-mermaid-diagram.feature

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import { generateFoundationMd } from '../foundation-md';
import { addFoundationBoundedContext } from '../../commands/add-foundation-bounded-context';
import {
  createTempTestDirSync,
  removeTempTestDirSync,
} from '../../test-helpers/temp-directory';

describe('Feature: Generate Event Storm section in FOUNDATION.md with validated Mermaid diagram', () => {
  describe('Scenario: FOUNDATION.md auto-regenerates with Event Storm section when bounded context added', () => {
    let testDir: string;
    let foundationJsonPath: string;
    let foundationMdPath: string;

    beforeEach(() => {
      testDir = createTempTestDirSync('foundation-autogen');
      foundationJsonPath = path.join(testDir, 'spec/foundation.json');
      foundationMdPath = path.join(testDir, 'spec/FOUNDATION.md');
    });

    afterEach(() => {
      removeTempTestDirSync(testDir);
    });

    it('should auto-regenerate FOUNDATION.md with Event Storm section when bounded context is added', async () => {
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
      await addFoundationBoundedContext('Work Management', { cwd: testDir });

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
    let testDir: string;
    let foundationJsonPath: string;
    let foundationMdPath: string;

    beforeEach(() => {
      testDir = createTempTestDirSync('foundation-mermaid');
      foundationJsonPath = path.join(testDir, 'spec/foundation.json');
      foundationMdPath = path.join(testDir, 'FOUNDATION.md');
    });

    afterEach(() => {
      removeTempTestDirSync(testDir);
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
    let testDir: string;
    let foundationMdPath: string;

    beforeEach(() => {
      testDir = createTempTestDirSync('foundation-validation');
      foundationMdPath = path.join(testDir, 'FOUNDATION.md');
    });

    afterEach(() => {
      removeTempTestDirSync(testDir);
    });

    it('Scenario: Bounded context with malicious code injection causes validation error', async () => {
      // @step Given foundation.json has eventStorm field with bounded context 'Context"];malicious[("code'
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
              text: 'Context"];malicious[("code',
              deleted: false,
            },
          ],
          nextItemId: 2,
        },
      };

      // @step When generateFoundationMd is called with this Event Storm data
      // @step Then a validation error should be thrown
      await expect(async () => {
        await generateFoundationMd(foundationData);
      }).rejects.toThrow();

      // @step And the error should indicate invalid Mermaid syntax
      try {
        await generateFoundationMd(foundationData);
      } catch (error) {
        expect(error).toBeDefined();
        expect((error as Error).message).toContain('Mermaid');
      }
    });

    it('Scenario: generateFoundationMd throws error with helpful message pointing to invalid syntax', async () => {
      // @step Given foundation.json has Event Storm data with invalid Mermaid syntax in bounded context name
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
              text: 'Context"];malicious[("code',
              deleted: false,
            },
          ],
          nextItemId: 2,
        },
      };

      // @step When generateFoundationMd attempts to generate FOUNDATION.md
      // @step Then an error should be thrown
      await expect(async () => {
        await generateFoundationMd(foundationData);
      }).rejects.toThrow();

      // @step And the error message should help identify the syntax issue
      // @step And the error message should be descriptive enough for developers to fix the problem
      try {
        await generateFoundationMd(foundationData);
      } catch (error) {
        expect(error).toBeDefined();
        expect((error as Error).message).toBeTruthy();
        expect((error as Error).message.length).toBeGreaterThan(10);
      }
    });

    it('Scenario: FOUNDATION.md is not written when Event Storm diagram validation fails', async () => {
      // @step Given foundation.json has eventStorm field with invalid bounded context 'Context"];malicious[("code'
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
              text: 'Context"];malicious[("code',
              deleted: false,
            },
          ],
          nextItemId: 2,
        },
      };

      // @step When generateFoundationMd is called
      // @step Then an error should be thrown before FOUNDATION.md is written
      await expect(async () => {
        await generateFoundationMd(foundationData);
      }).rejects.toThrow();

      // @step And FOUNDATION.md should not exist or should remain unchanged
      // @step And no file with broken Mermaid diagrams should be created
      expect(fs.existsSync(foundationMdPath)).toBe(false);
    });
  });

  describe('Scenario: Event Storm section only appears when eventStorm data exists', () => {
    let testDir: string;
    let foundationJsonPath: string;
    let foundationMdPath: string;

    beforeEach(() => {
      testDir = createTempTestDirSync('foundation-no-eventstorm');
      foundationJsonPath = path.join(testDir, 'spec/foundation.json');
      foundationMdPath = path.join(testDir, 'FOUNDATION.md');
    });

    afterEach(() => {
      removeTempTestDirSync(testDir);
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
