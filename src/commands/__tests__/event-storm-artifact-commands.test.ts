/**
 * Feature: spec/features/event-storm-artifact-commands-events-commands-aggregates.feature
 *
 * Tests for Event Storm artifact commands (add-domain-event, add-command, add-aggregate)
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { addDomainEvent } from '../add-domain-event';
import { addCommand } from '../add-command';
import { addAggregate } from '../add-aggregate';

describe('Feature: Event Storm artifact commands (events, commands, aggregates)', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-event-storm-commands');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Add domain event to work unit', () => {
    it('should create event with proper Event Storm fields', async () => {
      // @step Given I have a work unit "AUTH-001" in specifying status
      const workUnitsContent = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsContent, null, 2)
      );

      // @step When I run "fspec add-domain-event AUTH-001 "UserRegistered""
      const result = await addDomainEvent({
        workUnitId: 'AUTH-001',
        text: 'UserRegistered',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      // Read the updated work units file
      const { readFile: read } = await import('fs/promises');
      const updatedContent = await read(
        join(testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const updatedData = JSON.parse(updatedContent);
      const workUnit = updatedData.workUnits['AUTH-001'];

      // @step Then eventStorm section should be initialized with level "process_modeling"
      expect(workUnit.eventStorm).toBeDefined();
      expect(workUnit.eventStorm.level).toBe('process_modeling');

      // @step And an event item should be created with id=0
      expect(workUnit.eventStorm.items).toHaveLength(1);
      expect(workUnit.eventStorm.items[0].id).toBe(0);

      // @step And the event should have type="event"
      expect(workUnit.eventStorm.items[0].type).toBe('event');

      // @step And the event should have color="orange"
      expect(workUnit.eventStorm.items[0].color).toBe('orange');

      // @step And the event should have text="UserRegistered"
      expect(workUnit.eventStorm.items[0].text).toBe('UserRegistered');

      // @step And the event should have deleted=false
      expect(workUnit.eventStorm.items[0].deleted).toBe(false);

      // @step And the event should have createdAt timestamp
      expect(workUnit.eventStorm.items[0].createdAt).toBeDefined();
      expect(typeof workUnit.eventStorm.items[0].createdAt).toBe('string');

      // @step And nextItemId should be 1
      expect(workUnit.eventStorm.nextItemId).toBe(1);
    });
  });

  describe('Scenario: Add command with actor flag', () => {
    it('should create command with actor field', async () => {
      // @step Given I have a work unit "AUTH-001" with existing eventStorm section
      const workUnitsContent = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            eventStorm: {
              level: 'process_modeling',
              items: [
                {
                  id: 0,
                  type: 'event',
                  color: 'orange',
                  text: 'UserRegistered',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                },
              ],
              nextItemId: 1,
            },
          },
        },
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsContent, null, 2)
      );

      // @step When I run "fspec add-command AUTH-001 "AuthenticateUser" --actor "User""
      const result = await addCommand({
        workUnitId: 'AUTH-001',
        text: 'AuthenticateUser',
        actor: 'User',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      // Read the updated work units file
      const { readFile: read } = await import('fs/promises');
      const updatedContent = await read(
        join(testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const updatedData = JSON.parse(updatedContent);
      const workUnit = updatedData.workUnits['AUTH-001'];

      // @step Then a command item should be created with id=1
      expect(workUnit.eventStorm.items).toHaveLength(2);
      expect(workUnit.eventStorm.items[1].id).toBe(1);

      // @step And the command should have type="command"
      expect(workUnit.eventStorm.items[1].type).toBe('command');

      // @step And the command should have color="blue"
      expect(workUnit.eventStorm.items[1].color).toBe('blue');

      // @step And the command should have text="AuthenticateUser"
      expect(workUnit.eventStorm.items[1].text).toBe('AuthenticateUser');

      // @step And the command should have actor="User"
      expect(workUnit.eventStorm.items[1].actor).toBe('User');

      // @step And nextItemId should be 2
      expect(workUnit.eventStorm.nextItemId).toBe(2);
    });
  });

  describe('Scenario: Add aggregate with responsibilities flag', () => {
    it('should create aggregate with responsibilities array', async () => {
      // @step Given I have a work unit "AUTH-001" with 2 Event Storm items
      const workUnitsContent = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            eventStorm: {
              level: 'process_modeling',
              items: [
                {
                  id: 0,
                  type: 'event',
                  color: 'orange',
                  text: 'UserRegistered',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                },
                {
                  id: 1,
                  type: 'command',
                  color: 'blue',
                  text: 'AuthenticateUser',
                  actor: 'User',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                },
              ],
              nextItemId: 2,
            },
          },
        },
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsContent, null, 2)
      );

      // @step When I run "fspec add-aggregate AUTH-001 "User" --responsibilities "Authentication,Profile management""
      const result = await addAggregate({
        workUnitId: 'AUTH-001',
        text: 'User',
        responsibilities: 'Authentication,Profile management',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      // Read the updated work units file
      const { readFile: read } = await import('fs/promises');
      const updatedContent = await read(
        join(testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const updatedData = JSON.parse(updatedContent);
      const workUnit = updatedData.workUnits['AUTH-001'];

      // @step Then an aggregate item should be created with id=2
      expect(workUnit.eventStorm.items).toHaveLength(3);
      expect(workUnit.eventStorm.items[2].id).toBe(2);

      // @step And the aggregate should have type="aggregate"
      expect(workUnit.eventStorm.items[2].type).toBe('aggregate');

      // @step And the aggregate should have color="yellow"
      expect(workUnit.eventStorm.items[2].color).toBe('yellow');

      // @step And the aggregate should have text="User"
      expect(workUnit.eventStorm.items[2].text).toBe('User');

      // @step And the aggregate should have responsibilities=["Authentication", "Profile management"]
      expect(workUnit.eventStorm.items[2].responsibilities).toEqual([
        'Authentication',
        'Profile management',
      ]);

      // @step And nextItemId should be 3
      expect(workUnit.eventStorm.nextItemId).toBe(3);
    });
  });

  describe('Scenario: Initialize eventStorm section on first command', () => {
    it('should create eventStorm section when adding first item', async () => {
      // @step Given I have a work unit "AUTH-001" without eventStorm section
      const workUnitsContent = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsContent, null, 2)
      );

      // @step When I run "fspec add-domain-event AUTH-001 "FirstEvent""
      const result = await addDomainEvent({
        workUnitId: 'AUTH-001',
        text: 'FirstEvent',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      // Read the updated work units file
      const { readFile: read } = await import('fs/promises');
      const updatedContent = await read(
        join(testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const updatedData = JSON.parse(updatedContent);
      const workUnit = updatedData.workUnits['AUTH-001'];

      // @step Then eventStorm section should be created
      expect(workUnit.eventStorm).toBeDefined();

      // @step And eventStorm.level should be "process_modeling"
      expect(workUnit.eventStorm.level).toBe('process_modeling');

      // @step And eventStorm.items should be an empty array initially
      // (This step is conceptual - items array is initialized empty, then the event is added)

      // @step And the new event should be appended to items
      expect(workUnit.eventStorm.items).toHaveLength(1);
      expect(workUnit.eventStorm.items[0].text).toBe('FirstEvent');

      // @step And eventStorm.nextItemId should start at 0 and increment to 1
      expect(workUnit.eventStorm.nextItemId).toBe(1);
    });
  });
});
