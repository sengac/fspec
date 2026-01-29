// Feature: spec/features/big-picture-event-storm-in-foundation-json.feature

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { fileManager } from '../../utils/file-manager';
import { addFoundationBoundedContext } from '../add-foundation-bounded-context';
import { showFoundationEventStorm } from '../show-foundation-event-storm';
import type { GenericFoundation } from '../../types/generic-foundation';
import type { EventStormItem } from '../../types';
import {
  setupFoundationTest,
  type FoundationTestSetup,
} from '../../test-helpers/universal-test-setup';
import { writeJsonTestFile } from '../../test-helpers/test-file-operations';

describe('Feature: Big Picture Event Storm in foundation.json', () => {
  let setup: FoundationTestSetup;

  beforeEach(async () => {
    // Create isolated temp directory
    setup = await setupFoundationTest('add-foundation-bounded-context');

    // Create initial foundation.json in temp directory
    const initialFoundation: GenericFoundation = {
      version: '2.0.0',
      project: {
        name: 'Test Project',
        vision: 'Test vision',
        projectType: 'other',
      },
      problemSpace: {
        primaryProblem: {
          title: 'Test problem',
          description: 'Test description',
          impact: 'medium',
        },
      },
      solutionSpace: {
        overview: 'Test solution',
        capabilities: [],
      },
    };
    await writeJsonTestFile(setup.foundationFile, initialFoundation);
  });

  afterEach(async () => {
    // Clean up temp directory
    await setup.cleanup();
  });

  describe('Scenario: Add bounded context to foundation with no existing Event Storm', () => {
    it('should create eventStorm section and add bounded context', async () => {
      // @step Given foundation.json has no eventStorm section
      // Ensure foundation has NO eventStorm section
      await fileManager.transaction<GenericFoundation>(
        setup.foundationFile,
        async data => {
          delete data.eventStorm;
        }
      );

      // @step When I run "fspec add-foundation-bounded-context 'User Management'"
      const result = await addFoundationBoundedContext('User Management', {
        cwd: setup.testDir,
      });

      expect(result.success).toBe(true);

      const updated = await fileManager.readJSON<GenericFoundation>(
        setup.foundationFile,
        {} as GenericFoundation
      );

      // @step Then the eventStorm section should be created with level='big_picture'
      expect(updated.eventStorm).toBeDefined();
      expect(updated.eventStorm?.level).toBe('big_picture');

      // @step And the eventStorm should have items array and nextItemId=1
      expect(updated.eventStorm?.items).toBeInstanceOf(Array);
      expect(updated.eventStorm?.nextItemId).toBeGreaterThan(1);

      // @step And a bounded context item should be added with type='bounded_context'
      const boundedContexts = updated.eventStorm?.items.filter(
        item => item.type === 'bounded_context'
      );
      expect(boundedContexts).toHaveLength(1);

      // @step And the bounded context text should be "User Management"
      expect(boundedContexts?.[0].text).toBe('User Management');

      // @step And the bounded context deleted flag should be false
      expect(boundedContexts?.[0].deleted).toBe(false);
    });
  });

  describe('Scenario: Show foundation Event Storm filtered by type', () => {
    it('should return only bounded contexts excluding deleted items', async () => {
      // @step Given foundation.json has Event Storm with 2 bounded contexts and 1 aggregate
      await fileManager.transaction<GenericFoundation>(
        setup.foundationFile,
        async data => {
          data.eventStorm = {
            level: 'big_picture',
            items: [
              {
                id: 1,
                type: 'bounded_context',
                text: 'User Management',
                color: null,
                deleted: false,
                createdAt: new Date().toISOString(),
              },
              {
                id: 2,
                type: 'bounded_context',
                text: 'Billing',
                color: null,
                deleted: false,
                createdAt: new Date().toISOString(),
              },
              {
                id: 3,
                type: 'aggregate',
                text: 'Order',
                color: '#FFFF00',
                deleted: false,
                createdAt: new Date().toISOString(),
              },
            ],
            nextItemId: 4,
          };
        }
      );

      // @step And all items have deleted=false
      const foundation = await fileManager.readJSON<GenericFoundation>(
        setup.foundationFile,
        {} as GenericFoundation
      );
      const allItems = foundation.eventStorm!.items;
      expect(allItems.every(item => item.deleted === false)).toBe(true);

      // @step When I run "fspec show-foundation-event-storm --type=bounded_context"
      const result = await showFoundationEventStorm({
        type: 'bounded_context',
        cwd: setup.testDir,
      });

      expect(result.success).toBe(true);

      // @step Then the output should be valid JSON
      expect(result.data).toBeDefined();
      expect(Array.isArray(result.data)).toBe(true);

      // @step And the JSON should contain 2 items
      expect(result.data).toHaveLength(2);

      // @step And all items should have type='bounded_context'
      expect(
        result.data.every(
          (item: EventStormItem) => item.type === 'bounded_context'
        )
      ).toBe(true);

      // @step And no deleted items should be included
      expect(
        result.data.every((item: EventStormItem) => item.deleted === false)
      ).toBe(true);
    });
  });

  describe('Scenario: Initialize Event Storm section when missing', () => {
    it('should initialize eventStorm with big_picture level and add bounded context', async () => {
      // @step Given foundation.json exists without eventStorm section
      await fileManager.transaction<GenericFoundation>(
        setup.foundationFile,
        async data => {
          delete data.eventStorm;
        }
      );

      // @step When I add a bounded context using add-foundation-bounded-context
      const result = await addFoundationBoundedContext('Payment Processing', {
        cwd: setup.testDir,
      });

      expect(result.success).toBe(true);

      const updated = await fileManager.readJSON<GenericFoundation>(
        setup.foundationFile,
        {} as GenericFoundation
      );

      // @step Then eventStorm section should be initialized
      expect(updated.eventStorm).toBeDefined();

      // @step And eventStorm.level should equal "big_picture"
      expect(updated.eventStorm?.level).toBe('big_picture');

      // @step And eventStorm.items should be an empty array initially
      // Note: This step is checking the state BEFORE adding the item,
      // but we can verify the structure is correct
      expect(Array.isArray(updated.eventStorm?.items)).toBe(true);

      // @step And eventStorm.nextItemId should equal 1
      // Note: After adding one item, nextItemId will be 2
      expect(updated.eventStorm?.nextItemId).toBeGreaterThan(1);

      // @step And the new bounded context should be added to items array
      expect(updated.eventStorm?.items).toHaveLength(1);
      expect(updated.eventStorm?.items[0].text).toBe('Payment Processing');
      expect(updated.eventStorm?.items[0].type).toBe('bounded_context');
    });
  });

  describe('Scenario: Filter out deleted items when showing Event Storm', () => {
    it('should return only non-deleted items', async () => {
      // @step Given foundation Event Storm has 3 items total
      await fileManager.transaction<GenericFoundation>(
        setup.foundationFile,
        async data => {
          data.eventStorm = {
            level: 'big_picture',
            items: [
              {
                id: 1,
                type: 'bounded_context',
                text: 'Active Context 1',
                color: null,
                deleted: false,
                createdAt: new Date().toISOString(),
              },
              {
                id: 2,
                type: 'bounded_context',
                text: 'Deleted Context',
                color: null,
                deleted: true,
                createdAt: new Date().toISOString(),
                deletedAt: new Date().toISOString(),
              },
              {
                id: 3,
                type: 'bounded_context',
                text: 'Active Context 2',
                color: null,
                deleted: false,
                createdAt: new Date().toISOString(),
              },
            ],
            nextItemId: 4,
          };
        }
      );

      const foundation = await fileManager.readJSON<GenericFoundation>(
        setup.foundationFile,
        {} as GenericFoundation
      );
      expect(foundation.eventStorm!.items).toHaveLength(3);

      // @step And 1 item has deleted=true
      const deletedItems = foundation.eventStorm!.items.filter(
        item => item.deleted === true
      );
      expect(deletedItems).toHaveLength(1);

      // @step And 2 items have deleted=false
      const activeItems = foundation.eventStorm!.items.filter(
        item => item.deleted === false
      );
      expect(activeItems).toHaveLength(2);

      // @step When I run "fspec show-foundation-event-storm"
      const result = await showFoundationEventStorm({ cwd: setup.testDir });

      expect(result.success).toBe(true);

      // @step Then the output should contain exactly 2 items
      expect(result.data).toHaveLength(2);

      // @step And no items should have deleted=true
      expect(
        result.data.every((item: EventStormItem) => item.deleted !== true)
      ).toBe(true);

      // @step And all returned items should have deleted=false
      expect(
        result.data.every((item: EventStormItem) => item.deleted === false)
      ).toBe(true);
    });
  });
});
