/**
 * Feature: spec/features/generic-and-unhelpful-examples-auto-generated-from-event-storm-domain-events.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { generateExampleMappingFromEventStorm } from '../generate-example-mapping-from-event-storm';
import type { WorkUnitsData } from '../../types';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import {
  writeJsonTestFile,
  readJsonTestFile,
} from '../../test-helpers/test-file-operations';

describe('Feature: Generic and unhelpful examples auto-generated from Event Storm domain events', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('event-storm-skip-example-generation');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Transform Event Storm without generating examples from domain events', () => {
    it('should not generate examples from domain events', async () => {
      // @step Given a work unit with Event Storm containing domain event "TrackPlayed"
      const workUnitsData: WorkUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            type: 'story',
            title: 'Test Story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              {
                state: 'specifying',
                timestamp: new Date().toISOString(),
              },
            ],
            eventStorm: {
              level: 'process_modeling',
              items: [
                {
                  id: 0,
                  type: 'event',
                  color: 'orange',
                  text: 'TrackPlayed',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                },
              ],
              nextItemId: 1,
            },
            questions: [],
          },
        },
        states: {
          backlog: [],
          specifying: ['TEST-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeJsonTestFile(setup.workUnitsFile, workUnitsData);

      // @step When I transform Event Storm to Example Mapping
      const result = await generateExampleMappingFromEventStorm({
        workUnitId: 'TEST-001',
        cwd: setup.testDir,
      });

      expect(result.success).toBe(true);

      // Read updated data
      // Use readJsonTestFile instead
      const updatedData: WorkUnitsData = await readJsonTestFile(
        setup.workUnitsFile
      );

      // @step Then 0 examples should be added
      expect(result.examplesAdded).toBe(0);

      // @step And the examples list should remain empty
      const workUnit = updatedData.workUnits['TEST-001'];
      expect(workUnit.examples).toBeDefined();
      expect(workUnit.examples).toHaveLength(0);
    });
  });

  describe('Scenario: Preserve policy and hotspot transformations while skipping event transformation', () => {
    it('should transform policies and hotspots but not events', async () => {
      // @step Given a work unit with Event Storm containing:
      const workUnitsData: WorkUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        workUnits: {
          'TEST-002': {
            id: 'TEST-002',
            type: 'story',
            title: 'Test Story 2',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              {
                state: 'specifying',
                timestamp: new Date().toISOString(),
              },
            ],
            eventStorm: {
              level: 'process_modeling',
              items: [
                {
                  id: 0,
                  type: 'policy',
                  color: 'purple',
                  text: 'Load dashboard policy',
                  when: 'UserAuthenticated',
                  then: 'LoadDashboard',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                },
                {
                  id: 1,
                  type: 'hotspot',
                  color: 'pink',
                  text: 'Session expiry concern',
                  concern: 'What happens if session expires?',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                },
                {
                  id: 2,
                  type: 'event',
                  color: 'orange',
                  text: 'TrackPlayed',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                },
              ],
              nextItemId: 3,
            },
            questions: [],
          },
        },
        states: {
          backlog: [],
          specifying: ['TEST-002'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeJsonTestFile(setup.workUnitsFile, workUnitsData);

      // @step When I transform Event Storm to Example Mapping
      const result = await generateExampleMappingFromEventStorm({
        workUnitId: 'TEST-002',
        cwd: setup.testDir,
      });

      expect(result.success).toBe(true);

      // Read updated data
      // Use readJsonTestFile instead
      const updatedData: WorkUnitsData = await readJsonTestFile(
        setup.workUnitsFile
      );
      const workUnit = updatedData.workUnits['TEST-002'];

      // @step Then 1 rule should be added from the policy
      expect(result.rulesAdded).toBe(1);
      expect(workUnit.rules).toHaveLength(1);
      expect(workUnit.rules![0].text).toContain('load dashboard');
      expect(workUnit.rules![0].text).toContain('user authenticated');

      // @step And 1 question should be added from the hotspot
      expect(result.questionsAdded).toBe(1);
      expect(workUnit.questions).toHaveLength(1);
      expect(workUnit.questions[0].text).toContain(
        'What happens if session expires?'
      );

      // @step And 0 examples should be added from the event
      expect(result.examplesAdded).toBe(0);
      expect(workUnit.examples).toHaveLength(0);
    });
  });

  describe('Scenario: Verify no generic examples are generated', () => {
    it('should report 0 examples added and maintain empty examples array', async () => {
      // @step Given a work unit with Event Storm containing domain event "PlaylistSaved"
      const workUnitsData: WorkUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        workUnits: {
          'TEST-003': {
            id: 'TEST-003',
            type: 'story',
            title: 'Test Story 3',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              {
                state: 'specifying',
                timestamp: new Date().toISOString(),
              },
            ],
            eventStorm: {
              level: 'process_modeling',
              items: [
                {
                  id: 0,
                  type: 'event',
                  color: 'orange',
                  text: 'PlaylistSaved',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                },
              ],
              nextItemId: 1,
            },
            questions: [],
          },
        },
        states: {
          backlog: [],
          specifying: ['TEST-003'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeJsonTestFile(setup.workUnitsFile, workUnitsData);

      // @step When I transform Event Storm to Example Mapping
      const result = await generateExampleMappingFromEventStorm({
        workUnitId: 'TEST-003',
        cwd: setup.testDir,
      });

      expect(result.success).toBe(true);

      // @step Then the transformation result should show "Examples added: 0"
      expect(result.examplesAdded).toBe(0);

      // @step And the work unit should have an empty examples array
      // Use readJsonTestFile instead
      const updatedData: WorkUnitsData = await readJsonTestFile(
        setup.workUnitsFile
      );
      const workUnit = updatedData.workUnits['TEST-003'];
      expect(workUnit.examples).toBeDefined();
      expect(workUnit.examples).toHaveLength(0);
    });
  });
});
