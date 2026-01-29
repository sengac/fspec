/**
 * Feature: spec/features/malformed-question-text-when-transforming-event-storm-hotspots-to-example-mapping.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import { writeJsonTestFile } from '../../test-helpers/test-file-operations';
import { generateExampleMappingFromEventStorm } from '../generate-example-mapping-from-event-storm';
import type { WorkUnitsData } from '../../types';

describe('Feature: Malformed question text when transforming Event Storm hotspots to Example Mapping', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('hotspot-question-transformation');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Transform question-format concern without modification', () => {
    it('should preserve question text without template wrapping', async () => {
      // @step Given a hotspot with concern "When should playlists be saved?"
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
                  type: 'hotspot',
                  color: 'pink',
                  text: 'Playlist persistence timing',
                  concern: 'When should playlists be saved?',
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
      const { readFile } = await import('fs/promises');
      const updatedData: WorkUnitsData = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      const questions = updatedData.workUnits['TEST-001'].questions;

      expect(questions).toHaveLength(1);

      // @step Then the generated question should be "@human: When should playlists be saved?"
      expect(questions[0].text).toBe('@human: When should playlists be saved?');

      // @step And the question should not contain "What should"
      expect(questions[0].text).not.toContain('What should');

      // @step And the question should not contain " be?"
      expect(questions[0].text).not.toContain(' be?');
    });
  });

  describe('Scenario: Add question mark if concern lacks one', () => {
    it('should add question mark while preserving original text', async () => {
      // @step Given a hotspot with concern "How long should metadata be cached"
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
                  type: 'hotspot',
                  color: 'pink',
                  text: 'Track metadata caching',
                  concern: 'How long should metadata be cached',
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
      const { readFile } = await import('fs/promises');
      const updatedData: WorkUnitsData = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      const questions = updatedData.workUnits['TEST-002'].questions;

      expect(questions).toHaveLength(1);

      // @step Then the generated question should be "@human: How long should metadata be cached?"
      expect(questions[0].text).toBe(
        '@human: How long should metadata be cached?'
      );

      // @step And the question should preserve the original text
      expect(questions[0].text).toContain('How long should metadata be cached');

      // @step And the question should end with "?"
      expect(questions[0].text).toMatch(/\?$/);
    });
  });

  describe('Scenario: Preserve multiple sentences in concern', () => {
    it('should preserve all sentences and capitalization', async () => {
      // @step Given a hotspot with concern "Should drag-and-drop support multi-select? How to handle edge cases?"
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
                  type: 'hotspot',
                  color: 'pink',
                  text: 'Drag-and-drop complexity',
                  concern:
                    'Should drag-and-drop support multi-select? How to handle edge cases?',
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

      // Read updated data
      const { readFile } = await import('fs/promises');
      const updatedData: WorkUnitsData = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      const questions = updatedData.workUnits['TEST-003'].questions;

      expect(questions).toHaveLength(1);

      // @step Then the generated question should be "@human: Should drag-and-drop support multi-select? How to handle edge cases?"
      expect(questions[0].text).toBe(
        '@human: Should drag-and-drop support multi-select? How to handle edge cases?'
      );

      // @step And both sentences should be preserved
      expect(questions[0].text).toContain('Should drag-and-drop');
      expect(questions[0].text).toContain('How to handle edge cases');

      // @step And capitalization should be preserved
      expect(questions[0].text).toContain('Should'); // Capital S
      expect(questions[0].text).toContain('How'); // Capital H
    });
  });
});
