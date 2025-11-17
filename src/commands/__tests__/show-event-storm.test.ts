/**
 * Feature: spec/features/show-event-storm-data-as-json.feature
 * Tests for show-event-storm command
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { fileManager } from '../../utils/file-manager';
import type { WorkUnitsData, EventStormItem } from '../../types';
import { showEventStorm } from '../show-event-storm';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';

describe('Feature: Show Event Storm data as JSON', () => {
  let testDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    // Create isolated temp directory
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    await mkdir(join(testDir, 'spec'), { recursive: true });
    workUnitsFile = join(testDir, 'spec/work-units.json');

    // Create initial work-units.json in temp directory
    const initialData: WorkUnitsData = {
      version: '1.0.0',
      workUnits: {},
      states: {},
    };
    await writeFile(workUnitsFile, JSON.stringify(initialData, null, 2));
  });

  afterEach(async () => {
    // Clean up temp directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Output Event Storm events as JSON array', () => {
    it('should output Event Storm events as valid JSON', async () => {
      // @step Given work unit "TEST-001" has 3 events: "UserLoggedIn", "UserRegistered", "PasswordChanged"
      const events: EventStormItem[] = [
        {
          id: 1,
          type: 'event',
          text: 'UserLoggedIn',
          color: 'orange',
          deleted: false,
          createdAt: new Date().toISOString(),
        },
        {
          id: 2,
          type: 'event',
          text: 'UserRegistered',
          color: 'orange',
          deleted: false,
          createdAt: new Date().toISOString(),
        },
        {
          id: 3,
          type: 'event',
          text: 'PasswordChanged',
          color: 'orange',
          deleted: false,
          createdAt: new Date().toISOString(),
        },
      ];

      // @step And the events are not deleted
      await fileManager.transaction<WorkUnitsData>(
        workUnitsFile,
        async data => {
          data.workUnits['TEST-001'] = {
            id: 'TEST-001',
            title: 'Test Work Unit',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            eventStorm: {
              level: 'process_modeling',
              items: events,
              nextItemId: 4,
            },
          };
        }
      );

      // @step When I run "fspec show-event-storm TEST-001"
      const result = await showEventStorm({
        workUnitId: 'TEST-001',
        cwd: testDir,
      });

      // @step Then the output should be valid JSON
      expect(result.success).toBe(true);
      expect(() => JSON.parse(JSON.stringify(result.data))).not.toThrow();

      // @step And the JSON should contain an array with 3 event objects
      expect(Array.isArray(result.data)).toBe(true);
      expect(result.data).toHaveLength(3);

      // @step And each event object should have "type" as "event"
      result.data.forEach((item: EventStormItem) => {
        expect(item.type).toBe('event');
      });

      // @step And each event object should have a "text" field matching the event name
      expect(result.data[0].text).toBe('UserLoggedIn');
      expect(result.data[1].text).toBe('UserRegistered');
      expect(result.data[2].text).toBe('PasswordChanged');
    });
  });

  describe('Scenario: Output Event Storm bounded context as JSON object', () => {
    it('should output bounded context as valid JSON object', async () => {
      // @step Given work unit "DOMAIN-001" has a bounded context named "User Management"
      const boundedContext: EventStormItem = {
        id: 1,
        type: 'bounded_context',
        text: 'User Management',
        color: null,
        deleted: false,
        createdAt: new Date().toISOString(),
      };

      // @step And the bounded context is not deleted
      await fileManager.transaction<WorkUnitsData>(
        workUnitsFile,
        async data => {
          data.workUnits['DOMAIN-001'] = {
            id: 'DOMAIN-001',
            title: 'Domain Work Unit',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            eventStorm: {
              level: 'software_design',
              items: [boundedContext],
              nextItemId: 2,
            },
          };
        }
      );

      // @step When I run "fspec show-event-storm DOMAIN-001"
      const result = await showEventStorm({
        workUnitId: 'DOMAIN-001',
        cwd: testDir,
      });

      // @step Then the output should be valid JSON
      expect(result.success).toBe(true);
      expect(() => JSON.parse(JSON.stringify(result.data))).not.toThrow();

      // @step And the JSON should contain a bounded_context object
      expect(Array.isArray(result.data)).toBe(true);
      expect(result.data).toHaveLength(1);

      // @step And the object should have "type" as "bounded_context"
      expect(result.data[0].type).toBe('bounded_context');

      // @step And the object should have "text" as "User Management"
      expect(result.data[0].text).toBe('User Management');
    });
  });

  describe('Scenario: Handle work unit with no Event Storm data', () => {
    it('should return error for work unit without Event Storm data', async () => {
      // @step Given work unit "EMPTY-001" has no Event Storm items
      await fileManager.transaction<WorkUnitsData>(
        workUnitsFile,
        async data => {
          data.workUnits['EMPTY-001'] = {
            id: 'EMPTY-001',
            title: 'Empty Work Unit',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            // No eventStorm property
          };
        }
      );

      // @step When I run "fspec show-event-storm EMPTY-001"
      const result = await showEventStorm({
        workUnitId: 'EMPTY-001',
        cwd: testDir,
      });

      // @step Then the command should exit with error
      expect(result.success).toBe(false);

      // @step And the error message should indicate no Event Storm data exists
      expect(result.error).toContain('no Event Storm data');
    });
  });

  describe('Scenario: Filter out deleted Event Storm items', () => {
    it('should filter out deleted items from output', async () => {
      // @step Given work unit "MIX-001" has 5 Event Storm items
      const items: EventStormItem[] = [
        {
          id: 1,
          type: 'event',
          text: 'Event1',
          color: 'orange',
          deleted: false,
          createdAt: new Date().toISOString(),
        },
        {
          id: 2,
          type: 'command',
          text: 'Command1',
          color: 'blue',
          deleted: true,
          createdAt: new Date().toISOString(),
          deletedAt: new Date().toISOString(),
        },
        {
          id: 3,
          type: 'aggregate',
          text: 'Aggregate1',
          color: 'yellow',
          deleted: false,
          createdAt: new Date().toISOString(),
        },
        {
          id: 4,
          type: 'policy',
          text: 'Policy1',
          color: 'purple',
          deleted: true,
          createdAt: new Date().toISOString(),
          deletedAt: new Date().toISOString(),
        },
        {
          id: 5,
          type: 'hotspot',
          text: 'Hotspot1',
          color: 'red',
          deleted: false,
          createdAt: new Date().toISOString(),
        },
      ];

      // @step And 2 items have "deleted" set to true
      // @step And 3 items have "deleted" set to false
      await fileManager.transaction<WorkUnitsData>(
        workUnitsFile,
        async data => {
          data.workUnits['MIX-001'] = {
            id: 'MIX-001',
            title: 'Mixed Work Unit',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            eventStorm: {
              level: 'process_modeling',
              items,
              nextItemId: 6,
            },
          };
        }
      );

      // @step When I run "fspec show-event-storm MIX-001"
      const result = await showEventStorm({
        workUnitId: 'MIX-001',
        cwd: testDir,
      });

      // @step Then the output should be valid JSON
      expect(result.success).toBe(true);
      expect(() => JSON.parse(JSON.stringify(result.data))).not.toThrow();

      // @step And the JSON should contain exactly 3 items
      expect(Array.isArray(result.data)).toBe(true);
      expect(result.data).toHaveLength(3);

      // @step And no deleted items should be included in the output
      result.data.forEach((item: EventStormItem) => {
        expect(item.deleted).toBe(false);
      });
    });
  });
});
