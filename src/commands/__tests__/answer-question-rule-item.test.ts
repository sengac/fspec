/**
 * Feature: spec/features/answer-question-data-integrity.feature
 *
 * Tests for BUG-054: answer-question must create RuleItem objects, not raw strings
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { answerQuestion } from '../answer-question';
import type { WorkUnitsData, RuleItem } from '../../types';

import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';
describe('Feature: Answer Question Data Integrity', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('answer-question');
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Answer question with --add-to rule creates RuleItem object', () => {
    it('should create a RuleItem object with all required fields', async () => {
      // Given a work unit "TEST-001" with a question "Should this be standalone?"
      const workUnitsData: WorkUnitsData = {
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            title: 'Test Work Unit',
            type: 'task',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            nextRuleId: 0,
            nextExampleId: 0,
            nextQuestionId: 0,
            questions: [
              {
                id: 0,
                text: 'Should this be standalone?',
                deleted: false,
                selected: false,
                createdAt: new Date().toISOString(),
              },
            ],
            rules: [],
          },
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec answer-question TEST-001 0 --answer 'Yes, standalone script' --add-to rule"
      const result = await answerQuestion({
        workUnitId: 'TEST-001',
        index: 0,
        answer: 'Yes, standalone script',
        addTo: 'rule',
        cwd: testDir,
      });

      // Then the result should indicate success
      expect(result.success).toBe(true);
      expect(result.addedTo).toBe('rules');

      // And the rules array should contain a RuleItem object
      const updatedData = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );

      const workUnit = updatedData.workUnits['TEST-001'];
      expect(workUnit.rules).toHaveLength(1);

      const ruleItem = workUnit.rules[0] as RuleItem;

      // And the RuleItem should have an id field with a number
      expect(ruleItem).toHaveProperty('id');
      expect(typeof ruleItem.id).toBe('number');

      // And the RuleItem should have a text field with "Yes, standalone script"
      expect(ruleItem).toHaveProperty('text');
      expect(ruleItem.text).toBe('Yes, standalone script');

      // And the RuleItem should have a deleted field set to false
      expect(ruleItem).toHaveProperty('deleted');
      expect(ruleItem.deleted).toBe(false);

      // And the RuleItem should have a createdAt field with an ISO timestamp
      expect(ruleItem).toHaveProperty('createdAt');
      expect(ruleItem.createdAt).toMatch(/^\d{4}-\d{2}-\d{2}T/);

      // And show-work-unit should NOT display "[undefined] undefined"
      // (This is implicitly tested by the object structure being correct)
    });
  });

  describe('Scenario: RuleItem ID uses nextRuleId counter', () => {
    it('should use nextRuleId and increment the counter', async () => {
      // Given a work unit "TEST-003" with nextRuleId set to 5
      const workUnitsData: WorkUnitsData = {
        version: '0.7.0', // Prevent migration from running
        workUnits: {
          'TEST-003': {
            id: 'TEST-003',
            title: 'Test Work Unit',
            type: 'task',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            nextRuleId: 5,
            nextExampleId: 0,
            nextQuestionId: 0,
            questions: [
              {
                id: 0,
                text: 'Is validation needed?',
                deleted: false,
                selected: false,
                createdAt: new Date().toISOString(),
              },
            ],
            rules: [],
          },
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec answer-question TEST-003 0 --answer 'Yes' --add-to rule"
      await answerQuestion({
        workUnitId: 'TEST-003',
        index: 0,
        answer: 'Yes',
        addTo: 'rule',
        cwd: testDir,
      });

      // Then the new RuleItem should have id 5
      const updatedData = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );

      const workUnit = updatedData.workUnits['TEST-003'];
      expect(workUnit.rules[0].id).toBe(5);

      // And the work unit nextRuleId should be incremented to 6
      expect(workUnit.nextRuleId).toBe(6);
    });
  });

  describe('Scenario: Multiple answer-question calls create sequential IDs', () => {
    it('should create rules with sequential IDs', async () => {
      // Given a work unit "TEST-004" with two questions
      const workUnitsData: WorkUnitsData = {
        workUnits: {
          'TEST-004': {
            id: 'TEST-004',
            title: 'Test Work Unit',
            type: 'task',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            nextRuleId: 0,
            nextExampleId: 0,
            nextQuestionId: 0,
            questions: [
              {
                id: 0,
                text: 'First question?',
                deleted: false,
                selected: false,
                createdAt: new Date().toISOString(),
              },
              {
                id: 1,
                text: 'Second question?',
                deleted: false,
                selected: false,
                createdAt: new Date().toISOString(),
              },
            ],
            rules: [],
          },
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec answer-question TEST-004 0 --answer 'First answer' --add-to rule"
      await answerQuestion({
        workUnitId: 'TEST-004',
        index: 0,
        answer: 'First answer',
        addTo: 'rule',
        cwd: testDir,
      });

      // And I run "fspec answer-question TEST-004 1 --answer 'Second answer' --add-to rule"
      await answerQuestion({
        workUnitId: 'TEST-004',
        index: 1,
        answer: 'Second answer',
        addTo: 'rule',
        cwd: testDir,
      });

      // Then the first RuleItem should have id 0
      const updatedData = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );

      const workUnit = updatedData.workUnits['TEST-004'];

      // And the second RuleItem should have id 1
      expect(workUnit.rules[0].id).toBe(0);
      expect(workUnit.rules[1].id).toBe(1);

      // And both RuleItems should have proper object structure
      expect(workUnit.rules[0]).toHaveProperty('text');
      expect(workUnit.rules[0]).toHaveProperty('deleted');
      expect(workUnit.rules[0]).toHaveProperty('createdAt');
      expect(workUnit.rules[1]).toHaveProperty('text');
      expect(workUnit.rules[1]).toHaveProperty('deleted');
      expect(workUnit.rules[1]).toHaveProperty('createdAt');
    });
  });

  describe('Scenario: Existing corrupt data does not affect new additions', () => {
    it('should create valid RuleItem even when existing rules are corrupt', async () => {
      // Given a work unit "TEST-005" with corrupt string data in rules array
      const workUnitsData: WorkUnitsData = {
        workUnits: {
          'TEST-005': {
            id: 'TEST-005',
            title: 'Test Work Unit',
            type: 'task',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            nextRuleId: 2,
            nextExampleId: 0,
            nextQuestionId: 0,
            questions: [
              {
                id: 0,
                text: 'New question?',
                deleted: false,
                selected: false,
                createdAt: new Date().toISOString(),
              },
            ],
            // Simulate corrupt data (raw strings) - using type assertion to simulate the bug
            rules: [
              'Corrupt string 1',
              'Corrupt string 2',
            ] as unknown as RuleItem[],
          },
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec answer-question TEST-005 0 --answer 'New answer' --add-to rule"
      await answerQuestion({
        workUnitId: 'TEST-005',
        index: 0,
        answer: 'New answer',
        addTo: 'rule',
        cwd: testDir,
      });

      // Then the new entry should be a proper RuleItem object
      const updatedData = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );

      const workUnit = updatedData.workUnits['TEST-005'];
      const newRule = workUnit.rules[2]; // Third item (after 2 corrupt ones)

      // And the new RuleItem should have all required fields
      expect(newRule).toHaveProperty('id');
      expect(newRule).toHaveProperty('text');
      expect(newRule).toHaveProperty('deleted');
      expect(newRule).toHaveProperty('createdAt');
      expect(newRule.id).toBe(2);
      expect(newRule.text).toBe('New answer');
    });
  });
});
