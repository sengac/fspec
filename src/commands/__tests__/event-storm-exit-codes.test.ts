/**
 * Feature: spec/features/event-storm-commands-exit-with-code-1-on-success.feature
 *
 * Tests for BUG-086: Event Storm commands exit with code 0 on success
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import { addDomainEvent } from '../add-domain-event';
import { addCommand } from '../add-command';
import { addPolicy } from '../add-policy';
import { addHotspot } from '../add-hotspot';
import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';

describe('Feature: Event Storm commands exit with code 0 on success', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('event-storm-exit-codes');
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');

    // Initialize work units file with a work unit in specifying state
    await writeFile(
      workUnitsFile,
      JSON.stringify(
        {
          meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
          workUnits: {
            'UI-001': {
              id: 'UI-001',
              type: 'story',
              title: 'Test Story',
              status: 'specifying',
              createdAt: new Date().toISOString(),
              updatedAt: new Date().toISOString(),
            },
          },
          states: {
            backlog: [],
            specifying: ['UI-001'],
            testing: [],
            implementing: [],
            validating: [],
            done: [],
            blocked: [],
          },
        },
        null,
        2
      )
    );
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: add-domain-event returns exit code 0 on success', () => {
    it('should succeed when adding domain event', async () => {
      // @step Given I have a work unit "UI-001" in specifying state
      // (Already set up in beforeEach)

      // @step When I run "fspec add-domain-event UI-001 TestEvent"
      const result = await addDomainEvent({
        workUnitId: 'UI-001',
        text: 'TestEvent',
        cwd: testDir,
      });

      // @step Then the command should succeed
      expect(result.success).toBe(true);

      // @step And the domain event "TestEvent" should be added to UI-001
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(workUnits.workUnits['UI-001'].eventStorm).toBeDefined();
      expect(workUnits.workUnits['UI-001'].eventStorm.items).toHaveLength(1);
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].text).toBe(
        'TestEvent'
      );
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].type).toBe(
        'event'
      );
    });
  });

  describe('Scenario: add-command returns exit code 0 on success', () => {
    it('should succeed when adding command', async () => {
      // @step Given I have a work unit "UI-001" in specifying state
      // (Already set up in beforeEach)

      // @step When I run "fspec add-command UI-001 TestCommand"
      const result = await addCommand({
        workUnitId: 'UI-001',
        text: 'TestCommand',
        cwd: testDir,
      });

      // @step Then the command should succeed
      expect(result.success).toBe(true);

      // @step And the command "TestCommand" should be added to UI-001
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(workUnits.workUnits['UI-001'].eventStorm).toBeDefined();
      expect(workUnits.workUnits['UI-001'].eventStorm.items).toHaveLength(1);
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].text).toBe(
        'TestCommand'
      );
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].type).toBe(
        'command'
      );
    });
  });

  describe('Scenario: add-policy returns exit code 0 on success', () => {
    it('should succeed when adding policy', async () => {
      // @step Given I have a work unit "UI-001" in specifying state
      // (Already set up in beforeEach)

      // @step When I run "fspec add-policy UI-001 'Send email' --when UserRegistered --then SendEmail"
      const result = await addPolicy({
        workUnitId: 'UI-001',
        text: 'Send email',
        when: 'UserRegistered',
        then: 'SendEmail',
        cwd: testDir,
      });

      // @step Then the command should succeed
      expect(result.success).toBe(true);

      // @step And the policy "Send email" should be added to UI-001
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(workUnits.workUnits['UI-001'].eventStorm).toBeDefined();
      expect(workUnits.workUnits['UI-001'].eventStorm.items).toHaveLength(1);
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].text).toBe(
        'Send email'
      );
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].type).toBe(
        'policy'
      );
    });
  });

  describe('Scenario: add-hotspot returns exit code 0 on success', () => {
    it('should succeed when adding hotspot', async () => {
      // @step Given I have a work unit "UI-001" in specifying state
      // (Already set up in beforeEach)

      // @step When I run "fspec add-hotspot UI-001 'Email timeout' --concern 'Unclear timeout duration'"
      const result = await addHotspot({
        workUnitId: 'UI-001',
        text: 'Email timeout',
        concern: 'Unclear timeout duration',
        cwd: testDir,
      });

      // @step Then the command should succeed
      expect(result.success).toBe(true);

      // @step And the hotspot "Email timeout" should be added to UI-001
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(workUnits.workUnits['UI-001'].eventStorm).toBeDefined();
      expect(workUnits.workUnits['UI-001'].eventStorm.items).toHaveLength(1);
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].text).toBe(
        'Email timeout'
      );
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].type).toBe(
        'hotspot'
      );
    });
  });

  describe('Scenario: Multiple events can be added sequentially', () => {
    it('should add multiple events when called sequentially', async () => {
      // @step Given I have a work unit "UI-001" in specifying state
      // (Already set up in beforeEach)

      // @step When I add two domain events
      await addDomainEvent({
        workUnitId: 'UI-001',
        text: 'Event1',
        cwd: testDir,
      });

      await addDomainEvent({
        workUnitId: 'UI-001',
        text: 'Event2',
        cwd: testDir,
      });

      // @step Then both events should be added
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(workUnits.workUnits['UI-001'].eventStorm).toBeDefined();
      expect(workUnits.workUnits['UI-001'].eventStorm.items).toHaveLength(2);
      expect(workUnits.workUnits['UI-001'].eventStorm.items[0].text).toBe(
        'Event1'
      );
      expect(workUnits.workUnits['UI-001'].eventStorm.items[1].text).toBe(
        'Event2'
      );
    });
  });

  describe('Scenario: Event Storm command returns error for non-existent work unit', () => {
    it('should fail when work unit does not exist', async () => {
      // @step Given I have no work unit "NONEXISTENT-001"
      // (No such work unit exists in beforeEach)

      // @step When I run "fspec add-domain-event NONEXISTENT-001 Event"
      const result = await addDomainEvent({
        workUnitId: 'NONEXISTENT-001',
        text: 'Event',
        cwd: testDir,
      });

      // @step Then the command should fail
      expect(result.success).toBe(false);

      // @step And an error message should be present
      expect(result.error).toBeDefined();
      expect(result.error).toContain('NONEXISTENT-001');
    });
  });
});
