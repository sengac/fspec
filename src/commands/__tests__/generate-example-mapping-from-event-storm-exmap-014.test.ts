// Feature: spec/features/generate-example-mapping-from-event-storm.feature

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import {
  mkdtempSync,
  mkdirSync,
  rmSync,
  writeFileSync,
  readFileSync,
} from 'fs';
import { tmpdir } from 'os';
import type { WorkUnitsData } from '../../types';
import { generateExampleMappingFromEventStorm } from '../generate-example-mapping-from-event-storm';

describe('Feature: Generate Example Mapping from Event Storm', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(() => {
    if (tmpDir) {
      rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Generate business rule from Event Storm policy', () => {
    it('should derive rule from policy when/then fields', async () => {
      // @step Given work unit "AUTH-001" has Event Storm with policy item
      const workUnitsFile = join(tmpDir, 'spec', 'work-units.json');
      const workUnitsData: WorkUnitsData = {
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
            type: 'story',
            title: 'User Authentication',
            description: 'Test',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            eventStorm: {
              level: 'process_modeling',
              items: [
                {
                  id: 0,
                  type: 'policy',
                  color: 'purple',
                  text: 'Send Welcome Email Policy',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                  when: 'UserRegistered',
                  then: 'SendWelcomeEmail',
                },
              ],
              nextItemId: 1,
            },
            rules: [],
            examples: [],
            questions: [],
          },
        },
      };
      mkdirSync(join(tmpDir, 'spec'), { recursive: true });
      writeFileSync(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // @step And the policy has when="UserRegistered" and then="SendWelcomeEmail"
      const policy = workUnitsData.workUnits['AUTH-001'].eventStorm?.items[0];
      expect(policy).toBeDefined();
      expect(policy?.type).toBe('policy');
      if (policy?.type === 'policy') {
        expect(policy.when).toBe('UserRegistered');
        expect(policy.then).toBe('SendWelcomeEmail');
      }

      // @step When I run "fspec generate-example-mapping-from-event-storm AUTH-001"
      const result = await generateExampleMappingFromEventStorm({
        workUnitId: 'AUTH-001',
        cwd: tmpDir,
      });

      // @step Then a new rule should be added to AUTH-001 Example Mapping
      expect(result.success).toBe(true);
      const updatedData = JSON.parse(
        readFileSync(workUnitsFile, 'utf-8')
      ) as WorkUnitsData;
      const workUnit = updatedData.workUnits['AUTH-001'];
      expect(workUnit.rules).toHaveLength(1);

      // @step And the rule text should be "System must send welcome email after user registration"
      expect(workUnit.rules![0].text).toContain('welcome email');
      expect(workUnit.rules![0].text).toContain('after user regist');

      // @step And the rule should be derived from policy when/then fields
      expect(workUnit.rules![0].text).toContain('send');
    });
  });

  describe('Scenario: Generate scenario example from domain event', () => {
    it('should derive example from event text', async () => {
      // @step Given work unit "AUTH-001" has Event Storm with event item
      const workUnitsFile = join(tmpDir, 'spec', 'work-units.json');
      const workUnitsData: WorkUnitsData = {
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
            type: 'story',
            title: 'User Authentication',
            description: 'Test',
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
                  text: 'UserAuthenticated',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                },
              ],
              nextItemId: 1,
            },
            rules: [],
            examples: [],
            questions: [],
          },
        },
      };
      mkdirSync(join(tmpDir, 'spec'), { recursive: true });
      writeFileSync(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // @step And the event text is "UserAuthenticated"
      const event = workUnitsData.workUnits['AUTH-001'].eventStorm?.items[0];
      expect(event).toBeDefined();
      expect(event?.type).toBe('event');
      expect(event?.text).toBe('UserAuthenticated');

      // @step When I run "fspec generate-example-mapping-from-event-storm AUTH-001"
      const result = await generateExampleMappingFromEventStorm({
        workUnitId: 'AUTH-001',
        cwd: tmpDir,
      });

      // @step Then NO examples should be added to AUTH-001 Example Mapping (BUG-089 fix)
      expect(result.success).toBe(true);
      expect(result.examplesAdded).toBe(0);
      const updatedData = JSON.parse(
        readFileSync(workUnitsFile, 'utf-8')
      ) as WorkUnitsData;
      const workUnit = updatedData.workUnits['AUTH-001'];
      expect(workUnit.examples).toHaveLength(0);

      // @step And the examples list should remain empty
      expect(workUnit.examples).toBeDefined();
      expect(workUnit.examples).toEqual([]);
    });
  });

  describe('Scenario: Generate question from Event Storm hotspot', () => {
    it('should derive question from hotspot concern', async () => {
      // @step Given work unit "AUTH-001" has Event Storm with hotspot item
      const workUnitsFile = join(tmpDir, 'spec', 'work-units.json');
      const workUnitsData: WorkUnitsData = {
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
            type: 'story',
            title: 'User Authentication',
            description: 'Test',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            eventStorm: {
              level: 'process_modeling',
              items: [
                {
                  id: 0,
                  type: 'hotspot',
                  color: 'red',
                  text: 'Password Reset Timeout',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                  concern: 'Unclear password reset timeout',
                },
              ],
              nextItemId: 1,
            },
            rules: [],
            examples: [],
            questions: [],
          },
        },
      };
      mkdirSync(join(tmpDir, 'spec'), { recursive: true });
      writeFileSync(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // @step And the hotspot concern is "Unclear password reset timeout"
      const hotspot = workUnitsData.workUnits['AUTH-001'].eventStorm?.items[0];
      expect(hotspot).toBeDefined();
      expect(hotspot?.type).toBe('hotspot');
      if (hotspot?.type === 'hotspot') {
        expect(hotspot.concern).toBe('Unclear password reset timeout');
      }

      // @step When I run "fspec generate-example-mapping-from-event-storm AUTH-001"
      const result = await generateExampleMappingFromEventStorm({
        workUnitId: 'AUTH-001',
        cwd: tmpDir,
      });

      // @step Then a new question should be added to AUTH-001 Example Mapping
      expect(result.success).toBe(true);
      const updatedData = JSON.parse(
        readFileSync(workUnitsFile, 'utf-8')
      ) as WorkUnitsData;
      const workUnit = updatedData.workUnits['AUTH-001'];
      expect(workUnit.questions).toHaveLength(1);

      // @step And the question text should be "@human: What should password reset token timeout be?"
      expect(workUnit.questions![0].text).toContain('@human:');
      expect(workUnit.questions![0].text.toLowerCase()).toContain(
        'password reset'
      );
      expect(workUnit.questions![0].text.toLowerCase()).toContain('timeout');

      // @step And the question should be marked as unanswered
      expect(workUnit.questions![0].answer).toBeUndefined();
    });
  });
});
