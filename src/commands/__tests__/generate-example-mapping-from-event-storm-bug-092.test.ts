// Feature: spec/features/duplicate-question-ids-from-generate-example-mapping-from-event-storm.feature

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
import { addQuestion } from '../add-question';

describe('Feature: Duplicate question IDs from generate-example-mapping-from-event-storm', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(() => {
    if (tmpDir) {
      rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Initialize nextQuestionId when undefined', () => {
    it('should initialize nextQuestionId to 0 and assign sequential IDs', async () => {
      // @step Given a work unit with event storm hotspots
      const workUnitsFile = join(tmpDir, 'spec', 'work-units.json');
      const workUnitsData: WorkUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: ['TEST-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            type: 'story',
            title: 'Test Story',
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
                  text: 'Security Model',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                  concern: 'How to secure the system',
                },
                {
                  id: 1,
                  type: 'hotspot',
                  color: 'red',
                  text: 'Performance',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                  concern: 'How to handle load',
                },
              ],
              nextItemId: 2,
            },
            rules: [],
            examples: [],
            questions: [],
          },
        },
      };
      mkdirSync(join(tmpDir, 'spec'), { recursive: true });
      writeFileSync(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // @step And the work unit has no questions array or nextQuestionId is undefined
      const workUnit = workUnitsData.workUnits['TEST-001'];
      expect(workUnit.nextQuestionId).toBeUndefined();

      // @step When I run generate-example-mapping-from-event-storm command
      const result = await generateExampleMappingFromEventStorm({
        workUnitId: 'TEST-001',
        cwd: tmpDir,
      });

      // @step Then nextQuestionId should be initialized to 0
      expect(result.success).toBe(true);
      const updatedData = JSON.parse(
        readFileSync(workUnitsFile, 'utf-8')
      ) as WorkUnitsData;
      const updatedWorkUnit = updatedData.workUnits['TEST-001'];
      expect(updatedWorkUnit.nextQuestionId).toBeDefined();

      // @step And questions should be assigned sequential IDs starting from 0
      expect(updatedWorkUnit.questions).toHaveLength(2);
      expect(updatedWorkUnit.questions![0].id).toBe(0);
      expect(updatedWorkUnit.questions![1].id).toBe(1);
      expect(updatedWorkUnit.nextQuestionId).toBe(2);
    });
  });

  describe('Scenario: Assign sequential IDs when converting hotspots', () => {
    it('should create 3 questions with IDs 0, 1, 2 and nextQuestionId 3', async () => {
      // @step Given a work unit with 3 event storm hotspots
      const workUnitsFile = join(tmpDir, 'spec', 'work-units.json');
      const workUnitsData: WorkUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: ['TEST-002'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'TEST-002': {
            id: 'TEST-002',
            type: 'story',
            title: 'Test Story',
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
                  text: 'Hotspot 1',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                  concern: 'Concern 1',
                },
                {
                  id: 1,
                  type: 'hotspot',
                  color: 'red',
                  text: 'Hotspot 2',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                  concern: 'Concern 2',
                },
                {
                  id: 2,
                  type: 'hotspot',
                  color: 'red',
                  text: 'Hotspot 3',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                  concern: 'Concern 3',
                },
              ],
              nextItemId: 3,
            },
            rules: [],
            examples: [],
            questions: [],
            nextQuestionId: 0,
          },
        },
      };
      mkdirSync(join(tmpDir, 'spec'), { recursive: true });
      writeFileSync(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // @step And the work unit nextQuestionId is 0
      expect(workUnitsData.workUnits['TEST-002'].nextQuestionId).toBe(0);

      // @step When I run generate-example-mapping-from-event-storm command
      const result = await generateExampleMappingFromEventStorm({
        workUnitId: 'TEST-002',
        cwd: tmpDir,
      });

      // @step Then 3 questions should be created with IDs 0, 1, 2
      expect(result.success).toBe(true);
      const updatedData = JSON.parse(
        readFileSync(workUnitsFile, 'utf-8')
      ) as WorkUnitsData;
      const updatedWorkUnit = updatedData.workUnits['TEST-002'];
      expect(updatedWorkUnit.questions).toHaveLength(3);
      expect(updatedWorkUnit.questions![0].id).toBe(0);
      expect(updatedWorkUnit.questions![1].id).toBe(1);
      expect(updatedWorkUnit.questions![2].id).toBe(2);

      // @step And nextQuestionId should be 3
      expect(updatedWorkUnit.nextQuestionId).toBe(3);
    });
  });

  describe('Scenario: Prevent duplicate IDs on multiple invocations', () => {
    it('should assign unique IDs without duplicates on second invocation', async () => {
      // @step Given a work unit with event storm hotspots
      const workUnitsFile = join(tmpDir, 'spec', 'work-units.json');
      const workUnitsData: WorkUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: ['TEST-003'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'TEST-003': {
            id: 'TEST-003',
            type: 'story',
            title: 'Test Story',
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
                  text: 'Initial Hotspot',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                  concern: 'Initial concern',
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

      // @step And I have run generate-example-mapping-from-event-storm once
      await generateExampleMappingFromEventStorm({
        workUnitId: 'TEST-003',
        cwd: tmpDir,
      });

      // @step And questions already exist with specific IDs
      let updatedData = JSON.parse(
        readFileSync(workUnitsFile, 'utf-8')
      ) as WorkUnitsData;
      let updatedWorkUnit = updatedData.workUnits['TEST-003'];
      expect(updatedWorkUnit.questions).toHaveLength(1);
      expect(updatedWorkUnit.questions![0].id).toBe(0);
      const firstNextQuestionId = updatedWorkUnit.nextQuestionId;

      // @step When I add more hotspots and run generate-example-mapping-from-event-storm again
      updatedWorkUnit.eventStorm!.items.push({
        id: 1,
        type: 'hotspot',
        color: 'red',
        text: 'Second Hotspot',
        deleted: false,
        createdAt: new Date().toISOString(),
        concern: 'Second concern',
      });
      updatedWorkUnit.eventStorm!.nextItemId = 2;
      writeFileSync(workUnitsFile, JSON.stringify(updatedData, null, 2));

      await generateExampleMappingFromEventStorm({
        workUnitId: 'TEST-003',
        cwd: tmpDir,
      });

      // @step Then new questions should get sequential IDs without duplicates
      updatedData = JSON.parse(
        readFileSync(workUnitsFile, 'utf-8')
      ) as WorkUnitsData;
      updatedWorkUnit = updatedData.workUnits['TEST-003'];
      // Note: Command processes ALL hotspots each time, so 2 hotspots = 2 new questions
      // First run created 1 question (ID 0), second run creates 2 questions (IDs 1, 2)
      expect(updatedWorkUnit.questions).toHaveLength(3);

      // @step And all question IDs should be unique
      const questionIds = updatedWorkUnit.questions!.map(q => q.id);
      const uniqueIds = new Set(questionIds);
      expect(uniqueIds.size).toBe(questionIds.length); // No duplicates - this is the bug fix!
      expect(updatedWorkUnit.questions![0].id).toBe(0);
      expect(updatedWorkUnit.questions![1].id).toBe(1);
      expect(updatedWorkUnit.questions![2].id).toBe(2);
    });
  });

  describe('Scenario: Integrate with manual add-question command', () => {
    it('should prevent ID collision between generated and manual questions', async () => {
      // @step Given a work unit with event storm hotspots
      const workUnitsFile = join(tmpDir, 'spec', 'work-units.json');
      const workUnitsData: WorkUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: ['TEST-004'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'TEST-004': {
            id: 'TEST-004',
            type: 'story',
            title: 'Test Story',
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
                  text: 'First Hotspot',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                  concern: 'First concern',
                },
                {
                  id: 1,
                  type: 'hotspot',
                  color: 'red',
                  text: 'Second Hotspot',
                  deleted: false,
                  createdAt: new Date().toISOString(),
                  concern: 'Second concern',
                },
              ],
              nextItemId: 2,
            },
            rules: [],
            examples: [],
            questions: [],
          },
        },
      };
      mkdirSync(join(tmpDir, 'spec'), { recursive: true });
      writeFileSync(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // @step And I have run generate-example-mapping-from-event-storm
      await generateExampleMappingFromEventStorm({
        workUnitId: 'TEST-004',
        cwd: tmpDir,
      });

      // @step And questions exist with IDs up to N
      let updatedData = JSON.parse(
        readFileSync(workUnitsFile, 'utf-8')
      ) as WorkUnitsData;
      let updatedWorkUnit = updatedData.workUnits['TEST-004'];
      expect(updatedWorkUnit.questions).toHaveLength(2);
      const maxId = Math.max(...updatedWorkUnit.questions!.map(q => q.id));
      const nextExpectedId = updatedWorkUnit.nextQuestionId;

      // @step When I run manual add-question command
      await addQuestion({
        workUnitId: 'TEST-004',
        question: '@human: Manual question?',
        cwd: tmpDir,
      });

      // @step Then the new question should get ID N+1
      updatedData = JSON.parse(
        readFileSync(workUnitsFile, 'utf-8')
      ) as WorkUnitsData;
      updatedWorkUnit = updatedData.workUnits['TEST-004'];
      expect(updatedWorkUnit.questions).toHaveLength(3);
      const newQuestion = updatedWorkUnit.questions![2];
      expect(newQuestion.id).toBe(nextExpectedId);

      // @step And there should be no ID collision with existing questions
      const allQuestionIds = updatedWorkUnit.questions!.map(q => q.id);
      const uniqueIds = new Set(allQuestionIds);
      expect(uniqueIds.size).toBe(allQuestionIds.length); // All IDs are unique
      expect(newQuestion.id).toBeGreaterThan(maxId); // New ID is greater than all existing
    });
  });
});
