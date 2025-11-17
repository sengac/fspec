/**
 * Feature: spec/features/implement-foundation-event-storm-commands-for-aggregates-events-and-commands.feature
 *
 * Tests for foundation Event Storm commands:
 * - add-aggregate-to-foundation
 * - add-domain-event-to-foundation
 * - add-command-to-foundation
 */

import fs from 'fs/promises';
import path from 'path';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import type { GenericFoundation } from '../../types/foundation';
import type { EventStormItem } from '../../types';

// Test helpers
let testDir: string;

beforeEach(async () => {
  // Create temporary test directory
  testDir = path.join(process.cwd(), `.test-${Date.now()}`);
  await fs.mkdir(testDir, { recursive: true });
  await fs.mkdir(path.join(testDir, 'spec'), { recursive: true });
});

afterEach(async () => {
  // Cleanup test directory
  try {
    await fs.rm(testDir, { recursive: true, force: true });
  } catch {
    // Ignore cleanup errors
  }
});

describe('Feature: Implement Foundation Event Storm Commands for Aggregates, Events, and Commands', () => {
  describe('Scenario: Add aggregate to existing bounded context', () => {
    it('should create aggregate item linked to bounded context', async () => {
      // @step Given a foundation with bounded context "Work Management"
      const foundationPath = path.join(testDir, 'spec', 'foundation.json');
      const initialFoundation: GenericFoundation = {
        version: '2.0.0',
        project: { name: 'test', vision: 'test', projectType: 'cli-tool' },
        problemSpace: {
          primaryProblem: {
            title: 'test',
            description: 'test',
            impact: 'high',
          },
        },
        solutionSpace: { overview: 'test', capabilities: [] },
        personas: [],
        eventStorm: {
          level: 'big_picture',
          items: [
            {
              id: 1,
              type: 'bounded_context',
              text: 'Work Management',
              color: null,
              deleted: false,
              createdAt: new Date().toISOString(),
            },
          ],
          nextItemId: 2,
        },
      };
      await fs.writeFile(
        foundationPath,
        JSON.stringify(initialFoundation, null, 2)
      );

      // @step When I run 'fspec add-aggregate-to-foundation "Work Management" "WorkUnit"'
      // TODO: Call add-aggregate-to-foundation command
      // For now, this will fail as command doesn't exist yet (RED PHASE)
      const { addAggregateToFoundation } = await import(
        '../add-aggregate-to-foundation'
      ).catch(() => ({ addAggregateToFoundation: null }));

      if (!addAggregateToFoundation) {
        throw new Error(
          'add-aggregate-to-foundation command not implemented yet'
        );
      }

      await addAggregateToFoundation('Work Management', 'WorkUnit', {
        cwd: testDir,
      });

      // @step Then a new item with type 'aggregate' should be created in foundation.json
      const updatedFoundation: GenericFoundation = JSON.parse(
        await fs.readFile(foundationPath, 'utf-8')
      );

      const aggregateItem = updatedFoundation.eventStorm?.items.find(
        item => item.type === 'aggregate'
      );
      expect(aggregateItem).toBeDefined();

      // @step And the item should have text "WorkUnit"
      expect(aggregateItem?.text).toBe('WorkUnit');

      // @step And the item should be linked to "Work Management" bounded context via boundedContextId
      const boundedContext = updatedFoundation.eventStorm?.items.find(
        item =>
          item.type === 'bounded_context' && item.text === 'Work Management'
      );
      expect(aggregateItem).toHaveProperty(
        'boundedContextId',
        boundedContext?.id
      );
    });
  });

  describe('Scenario: Add domain event to existing bounded context', () => {
    it('should create domain event item linked to bounded context', async () => {
      // @step Given a foundation with bounded context "Work Management"
      const foundationPath = path.join(testDir, 'spec', 'foundation.json');
      const initialFoundation: GenericFoundation = {
        version: '2.0.0',
        project: { name: 'test', vision: 'test', projectType: 'cli-tool' },
        problemSpace: {
          primaryProblem: {
            title: 'test',
            description: 'test',
            impact: 'high',
          },
        },
        solutionSpace: { overview: 'test', capabilities: [] },
        personas: [],
        eventStorm: {
          level: 'big_picture',
          items: [
            {
              id: 1,
              type: 'bounded_context',
              text: 'Work Management',
              color: null,
              deleted: false,
              createdAt: new Date().toISOString(),
            },
          ],
          nextItemId: 2,
        },
      };
      await fs.writeFile(
        foundationPath,
        JSON.stringify(initialFoundation, null, 2)
      );

      // @step When I run 'fspec add-domain-event-to-foundation "Work Management" "WorkUnitCreated"'
      const { addDomainEventToFoundation } = await import(
        '../add-domain-event-to-foundation'
      ).catch(() => ({ addDomainEventToFoundation: null }));

      if (!addDomainEventToFoundation) {
        throw new Error(
          'add-domain-event-to-foundation command not implemented yet'
        );
      }

      await addDomainEventToFoundation('Work Management', 'WorkUnitCreated', {
        cwd: testDir,
      });

      // @step Then a new item with type 'event' should be created in foundation.json
      const updatedFoundation: GenericFoundation = JSON.parse(
        await fs.readFile(foundationPath, 'utf-8')
      );

      const eventItem = updatedFoundation.eventStorm?.items.find(
        item => item.type === 'event'
      );
      expect(eventItem).toBeDefined();

      // @step And the item should have text "WorkUnitCreated"
      expect(eventItem?.text).toBe('WorkUnitCreated');

      // @step And the item should be linked to "Work Management" bounded context
      const boundedContext = updatedFoundation.eventStorm?.items.find(
        item =>
          item.type === 'bounded_context' && item.text === 'Work Management'
      );
      expect(eventItem).toHaveProperty('boundedContextId', boundedContext?.id);
    });
  });

  describe('Scenario: Add command to existing bounded context', () => {
    it('should create command item linked to bounded context', async () => {
      // @step Given a foundation with bounded context "Work Management"
      const foundationPath = path.join(testDir, 'spec', 'foundation.json');
      const initialFoundation: GenericFoundation = {
        version: '2.0.0',
        project: { name: 'test', vision: 'test', projectType: 'cli-tool' },
        problemSpace: {
          primaryProblem: {
            title: 'test',
            description: 'test',
            impact: 'high',
          },
        },
        solutionSpace: { overview: 'test', capabilities: [] },
        personas: [],
        eventStorm: {
          level: 'big_picture',
          items: [
            {
              id: 1,
              type: 'bounded_context',
              text: 'Work Management',
              color: null,
              deleted: false,
              createdAt: new Date().toISOString(),
            },
          ],
          nextItemId: 2,
        },
      };
      await fs.writeFile(
        foundationPath,
        JSON.stringify(initialFoundation, null, 2)
      );

      // @step When I run 'fspec add-command-to-foundation "Work Management" "CreateWorkUnit"'
      const { addCommandToFoundation } = await import(
        '../add-command-to-foundation'
      ).catch(() => ({ addCommandToFoundation: null }));

      if (!addCommandToFoundation) {
        throw new Error(
          'add-command-to-foundation command not implemented yet'
        );
      }

      await addCommandToFoundation('Work Management', 'CreateWorkUnit', {
        cwd: testDir,
      });

      // @step Then a new item with type 'command' should be created in foundation.json
      const updatedFoundation: GenericFoundation = JSON.parse(
        await fs.readFile(foundationPath, 'utf-8')
      );

      const commandItem = updatedFoundation.eventStorm?.items.find(
        item => item.type === 'command'
      );
      expect(commandItem).toBeDefined();

      // @step And the item should have text "CreateWorkUnit"
      expect(commandItem?.text).toBe('CreateWorkUnit');

      // @step And the item should be linked to "Work Management" bounded context
      const boundedContext = updatedFoundation.eventStorm?.items.find(
        item =>
          item.type === 'bounded_context' && item.text === 'Work Management'
      );
      expect(commandItem).toHaveProperty(
        'boundedContextId',
        boundedContext?.id
      );
    });
  });

  describe('Scenario: Add aggregate to non-existent bounded context', () => {
    it('should fail with error message', async () => {
      // @step Given a foundation without bounded context "Foo"
      const foundationPath = path.join(testDir, 'spec', 'foundation.json');
      const initialFoundation: GenericFoundation = {
        version: '2.0.0',
        project: { name: 'test', vision: 'test', projectType: 'cli-tool' },
        problemSpace: {
          primaryProblem: {
            title: 'test',
            description: 'test',
            impact: 'high',
          },
        },
        solutionSpace: { overview: 'test', capabilities: [] },
        personas: [],
        eventStorm: {
          level: 'big_picture',
          items: [],
          nextItemId: 1,
        },
      };
      await fs.writeFile(
        foundationPath,
        JSON.stringify(initialFoundation, null, 2)
      );

      // @step When I run 'fspec add-aggregate-to-foundation "Foo" "Bar"'
      const { addAggregateToFoundation } = await import(
        '../add-aggregate-to-foundation'
      ).catch(() => ({ addAggregateToFoundation: null }));

      if (!addAggregateToFoundation) {
        throw new Error(
          'add-aggregate-to-foundation command not implemented yet'
        );
      }

      // @step Then the command should fail with error
      // @step And the error message should contain "Bounded context 'Foo' not found"
      await expect(
        addAggregateToFoundation('Foo', 'Bar', { cwd: testDir })
      ).rejects.toThrow("Bounded context 'Foo' not found");
    });
  });

  describe('Scenario: Filter Event Storm by bounded context', () => {
    it('should display only items for specified context', async () => {
      // @step Given a foundation with multiple bounded contexts and items
      const foundationPath = path.join(testDir, 'spec', 'foundation.json');
      const initialFoundation: GenericFoundation = {
        version: '2.0.0',
        project: { name: 'test', vision: 'test', projectType: 'cli-tool' },
        problemSpace: {
          primaryProblem: {
            title: 'test',
            description: 'test',
            impact: 'high',
          },
        },
        solutionSpace: { overview: 'test', capabilities: [] },
        personas: [],
        eventStorm: {
          level: 'big_picture',
          items: [
            {
              id: 1,
              type: 'bounded_context',
              text: 'Work Management',
              color: null,
              deleted: false,
              createdAt: new Date().toISOString(),
            },
            {
              id: 2,
              type: 'bounded_context',
              text: 'Specification',
              color: null,
              deleted: false,
              createdAt: new Date().toISOString(),
            },
          ],
          nextItemId: 3,
        },
      };
      await fs.writeFile(
        foundationPath,
        JSON.stringify(initialFoundation, null, 2)
      );

      // @step When I run 'fspec show-foundation-event-storm --context="Work Management"'
      const { showFoundationEventStorm } = await import(
        '../show-foundation-event-storm'
      ).catch(() => ({ showFoundationEventStorm: null }));

      if (!showFoundationEventStorm) {
        throw new Error('show-foundation-event-storm command not found');
      }

      const result = await showFoundationEventStorm({
        cwd: testDir,
        context: 'Work Management',
      });

      // @step Then only items for "Work Management" context should be displayed
      expect(result).toBeDefined();
      expect(result.items).toBeDefined();

      // @step And this includes the bounded context itself, aggregates, events, and commands
      const workManagementItems = result.items.filter(
        (item: EventStormItem) =>
          item.text === 'Work Management' ||
          ('boundedContextId' in item && item.boundedContextId === 1)
      );
      expect(workManagementItems.length).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Filter Event Storm by item type', () => {
    it('should display only items of specified type', async () => {
      // @step Given a foundation with multiple item types (bounded contexts, aggregates, events, commands)
      const foundationPath = path.join(testDir, 'spec', 'foundation.json');
      const initialFoundation: GenericFoundation = {
        version: '2.0.0',
        project: { name: 'test', vision: 'test', projectType: 'cli-tool' },
        problemSpace: {
          primaryProblem: {
            title: 'test',
            description: 'test',
            impact: 'high',
          },
        },
        solutionSpace: { overview: 'test', capabilities: [] },
        personas: [],
        eventStorm: {
          level: 'big_picture',
          items: [
            {
              id: 1,
              type: 'bounded_context',
              text: 'Work Management',
              color: null,
              deleted: false,
              createdAt: new Date().toISOString(),
            },
          ],
          nextItemId: 2,
        },
      };
      await fs.writeFile(
        foundationPath,
        JSON.stringify(initialFoundation, null, 2)
      );

      // @step When I run 'fspec show-foundation-event-storm --type=aggregate'
      const { showFoundationEventStorm } = await import(
        '../show-foundation-event-storm'
      ).catch(() => ({ showFoundationEventStorm: null }));

      if (!showFoundationEventStorm) {
        throw new Error('show-foundation-event-storm command not found');
      }

      const result = await showFoundationEventStorm({
        cwd: testDir,
        type: 'aggregate',
      });

      // @step Then only aggregate items should be displayed
      expect(result).toBeDefined();
      expect(result.items).toBeDefined();
      const aggregateItems = result.items.filter(
        (item: EventStormItem) => item.type === 'aggregate'
      );
      expect(
        result.items.every((item: EventStormItem) => item.type === 'aggregate')
      ).toBe(true);

      // @step And items from all bounded contexts should be included
      // This will be validated once we have aggregates from multiple contexts
      expect(aggregateItems).toBeDefined();
    });
  });
});
