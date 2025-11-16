/**
 * Feature: spec/features/event-storm-artifact-commands-policies-hotspots-external-systems.feature
 *
 * Tests for Event Storm artifact commands: add-policy, add-hotspot, add-external-system, add-bounded-context
 * Coverage: EXMAP-007
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, readFile, rm } from 'fs/promises';
import { join } from 'path';
import { addPolicy } from '../add-policy';
import { addHotspot } from '../add-hotspot';
import { addExternalSystem } from '../add-external-system';
import { addBoundedContext } from '../add-bounded-context';

describe('Feature: Event Storm artifact commands (policies, hotspots, external systems)', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-event-storm-artifacts');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Add policy with when and then flags', () => {
    it('should create policy item with when and then fields', async () => {
      // @step Given I have a work unit AUTH-001 in specifying status
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

      // @step When I run fspec add-policy AUTH-001 "Send welcome email" --when "UserRegistered" --then "SendWelcomeEmail"
      const result = await addPolicy({
        workUnitId: 'AUTH-001',
        text: 'Send welcome email',
        when: 'UserRegistered',
        then: 'SendWelcomeEmail',
        cwd: testDir,
      });

      // @step Then a policy item should be created with id 0
      expect(result.success).toBe(true);
      expect(result.policyId).toBe(0);

      // @step And the policy should have type "policy"
      // @step And the policy should have color "purple"
      // @step And the policy should have when "UserRegistered"
      // @step And the policy should have then "SendWelcomeEmail"
      const workUnitsData = JSON.parse(
        await readFile(join(testDir, 'spec', 'work-units.json'), 'utf-8')
      );
      const policy = workUnitsData.workUnits['AUTH-001'].eventStorm.items[0];
      expect(policy.type).toBe('policy');
      expect(policy.color).toBe('purple');
      expect(policy.text).toBe('Send welcome email');
      expect(policy.when).toBe('UserRegistered');
      expect(policy.then).toBe('SendWelcomeEmail');
    });
  });

  describe('Scenario: Add hotspot with concern flag', () => {
    it('should create hotspot item with concern field', async () => {
      // @step Given I have a work unit AUTH-001 with existing Event Storm items
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

      // @step When I run fspec add-hotspot AUTH-001 "Password reset token expiration" --concern "Unclear timeout duration"
      const result = await addHotspot({
        workUnitId: 'AUTH-001',
        text: 'Password reset token expiration',
        concern: 'Unclear timeout duration',
        cwd: testDir,
      });

      // @step Then a hotspot item should be created
      expect(result.success).toBe(true);
      expect(result.hotspotId).toBe(1);

      // @step And the hotspot should have type "hotspot"
      // @step And the hotspot should have color "red"
      // @step And the hotspot should have concern "Unclear timeout duration"
      const workUnitsData = JSON.parse(
        await readFile(join(testDir, 'spec', 'work-units.json'), 'utf-8')
      );
      const hotspot = workUnitsData.workUnits['AUTH-001'].eventStorm.items[1];
      expect(hotspot.type).toBe('hotspot');
      expect(hotspot.color).toBe('red');
      expect(hotspot.text).toBe('Password reset token expiration');
      expect(hotspot.concern).toBe('Unclear timeout duration');
    });
  });

  describe('Scenario: Add external system with type flag', () => {
    it('should create external system item with integrationType field', async () => {
      // @step Given I have a work unit AUTH-001 in specifying status
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

      // @step When I run fspec add-external-system AUTH-001 "OAuth2Provider" --type REST_API
      const result = await addExternalSystem({
        workUnitId: 'AUTH-001',
        text: 'OAuth2Provider',
        type: 'REST_API',
        cwd: testDir,
      });

      // @step Then an external system item should be created
      expect(result.success).toBe(true);
      expect(result.externalSystemId).toBe(0);

      // @step And the external system should have type "external_system"
      // @step And the external system should have color "pink"
      // @step And the external system should have integrationType "REST_API"
      const workUnitsData = JSON.parse(
        await readFile(join(testDir, 'spec', 'work-units.json'), 'utf-8')
      );
      const externalSystem =
        workUnitsData.workUnits['AUTH-001'].eventStorm.items[0];
      expect(externalSystem.type).toBe('external_system');
      expect(externalSystem.color).toBe('pink');
      expect(externalSystem.text).toBe('OAuth2Provider');
      expect(externalSystem.integrationType).toBe('REST_API');
    });
  });

  describe('Scenario: Add bounded context with description flag', () => {
    it('should create bounded context item with description field and null color', async () => {
      // @step Given I have a work unit AUTH-001 in specifying status
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

      // @step When I run fspec add-bounded-context AUTH-001 "User Management" --description "Handles user registration, authentication, and profile management"
      const result = await addBoundedContext({
        workUnitId: 'AUTH-001',
        text: 'User Management',
        description:
          'Handles user registration, authentication, and profile management',
        cwd: testDir,
      });

      // @step Then a bounded context item should be created
      expect(result.success).toBe(true);
      expect(result.boundedContextId).toBe(0);

      // @step And the bounded context should have type "bounded_context"
      // @step And the bounded context should have color null
      // @step And the bounded context should have description "Handles user registration, authentication, and profile management"
      const workUnitsData = JSON.parse(
        await readFile(join(testDir, 'spec', 'work-units.json'), 'utf-8')
      );
      const boundedContext =
        workUnitsData.workUnits['AUTH-001'].eventStorm.items[0];
      expect(boundedContext.type).toBe('bounded_context');
      expect(boundedContext.color).toBeNull();
      expect(boundedContext.text).toBe('User Management');
      expect(boundedContext.description).toBe(
        'Handles user registration, authentication, and profile management'
      );
    });
  });

  describe('Scenario: Initialize eventStorm section on first command', () => {
    it('should initialize eventStorm section with correct structure', async () => {
      // @step Given I have a work unit AUTH-001 with no eventStorm section
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

      // @step When I run fspec add-policy AUTH-001 "Send notification"
      const result = await addPolicy({
        workUnitId: 'AUTH-001',
        text: 'Send notification',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      // @step Then the eventStorm section should be initialized
      // @step And the eventStorm level should be "process_modeling"
      // @step And the eventStorm items array should contain the new policy
      // @step And the nextItemId should be 1
      const workUnitsData = JSON.parse(
        await readFile(join(testDir, 'spec', 'work-units.json'), 'utf-8')
      );
      const eventStorm = workUnitsData.workUnits['AUTH-001'].eventStorm;
      expect(eventStorm).toBeDefined();
      expect(eventStorm.level).toBe('process_modeling');
      expect(eventStorm.items).toHaveLength(1);
      expect(eventStorm.items[0].type).toBe('policy');
      expect(eventStorm.nextItemId).toBe(1);
    });
  });
});
