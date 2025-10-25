/**
 * Feature: spec/features/warn-ai-when-estimate-exceeds-13-points-to-break-down-work-unit.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { mkdir, writeFile, rm } from 'fs/promises';
import { updateWorkUnitEstimate } from '../update-work-unit-estimate';
import { showWorkUnit } from '../show-work-unit';
import type { WorkUnit } from '../../types/work-unit';

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

describe('Feature: Warn AI when estimate is > 13 points to break down work unit', () => {
  let tmpDir: string;
  let workUnitsFile: string;
  let featuresDir: string;

  beforeEach(async () => {
    tmpDir = join(
      process.cwd(),
      'tmp',
      `test-large-estimate-warning-${Date.now()}`
    );
    workUnitsFile = join(tmpDir, 'spec', 'work-units.json');
    featuresDir = join(tmpDir, 'spec', 'features');

    await mkdir(featuresDir, { recursive: true });
  });

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true });
  });

  describe('Scenario: Immediate warning when estimating story/bug at 21 points with persistent reminder', () => {
    it('should show immediate warning when setting estimate > 13 for bug', async () => {
      // Given: A work unit "BUG-005" exists with type "bug"
      const workUnitsData: WorkUnitsData = {
        workUnits: {
          'BUG-005': {
            id: 'BUG-005',
            type: 'bug',
            title: 'Fix authentication bug',
            description: 'Auth system failing',
            status: 'specifying',
            createdAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['BUG-005'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // And: BUG-005 has a completed feature file without prefill placeholders
      const featureFile = join(featuresDir, 'fix-auth-bug.feature');
      await writeFile(
        featureFile,
        `@BUG-005
Feature: Fix authentication bug

  Background: User Story
    As a developer
    I want to fix the auth bug
    So that users can log in

  Scenario: Fix the bug
    Given the auth system is broken
    When I apply the fix
    Then users can log in
`
      );

      // When: I run "fspec update-work-unit-estimate BUG-005 21"
      const result = await updateWorkUnitEstimate({
        workUnitId: 'BUG-005',
        estimate: 21,
        cwd: tmpDir,
      });

      // Then: The command should succeed
      expect(result.success).toBe(true);

      // And: The output should contain a system-reminder warning about estimate > 13 points
      // (This will be tested by checking the function returns expected warning)
      // Note: The actual console output test will be in integration tests
    });

    it('should show persistent reminder in show-work-unit output', async () => {
      // Given: BUG-005 has estimate of 21 points
      const workUnitsData: WorkUnitsData = {
        workUnits: {
          'BUG-005': {
            id: 'BUG-005',
            type: 'bug',
            title: 'Fix authentication bug',
            description: 'Auth system failing',
            status: 'specifying',
            estimate: 21,
            createdAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['BUG-005'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      const featureFile = join(featuresDir, 'fix-auth-bug.feature');
      await writeFile(
        featureFile,
        `@BUG-005
Feature: Fix authentication bug

  Background: User Story
    As a developer
    I want to fix the auth bug
    So that users can log in

  Scenario: Fix the bug
    Given the auth system is broken
    When I apply the fix
    Then users can log in
`
      );

      // When: I run "fspec show-work-unit BUG-005"
      const result = await showWorkUnit({
        workUnitId: 'BUG-005',
        cwd: tmpDir,
      });

      // Then: The output should contain a system-reminder warning about estimate > 13 points
      expect(result.systemReminders).toBeDefined();

      const largeEstimateReminder = result.systemReminders!.find(r =>
        r.includes('LARGE ESTIMATE WARNING')
      );
      expect(largeEstimateReminder).toBeDefined();
      expect(largeEstimateReminder).toContain('<system-reminder>');
      expect(largeEstimateReminder).toContain(
        'estimate is greater than 13 points'
      );
      expect(largeEstimateReminder).toContain('21 points is too large');
      expect(largeEstimateReminder).toContain('fspec create-story');
      expect(largeEstimateReminder).toContain('fspec add-dependency');

      // And: The warning should persist until estimate changes to <= 13 or status changes to done
      // (This is tested in subsequent scenarios)
    });
  });

  describe('Scenario: No warning for task type work units with large estimates', () => {
    it('should not show warning when estimating task at 21 points', async () => {
      // Given: A work unit "INFRA-001" exists with type "task"
      const workUnitsData: WorkUnitsData = {
        workUnits: {
          'INFRA-001': {
            id: 'INFRA-001',
            type: 'task',
            title: 'Infrastructure setup',
            description: 'Set up infrastructure',
            status: 'specifying',
            createdAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['INFRA-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // When: I run "fspec update-work-unit-estimate INFRA-001 21"
      const result = await updateWorkUnitEstimate({
        workUnitId: 'INFRA-001',
        estimate: 21,
        cwd: tmpDir,
      });

      // Then: The command should succeed
      expect(result.success).toBe(true);

      // And: The estimate should be set to 21 points without any system-reminder
      // (No warning in console output - will be verified in integration tests)
    });

    it('should not show warning in show-work-unit for task type', async () => {
      // Given: INFRA-001 is a task with 21 point estimate
      const workUnitsData: WorkUnitsData = {
        workUnits: {
          'INFRA-001': {
            id: 'INFRA-001',
            type: 'task',
            title: 'Infrastructure setup',
            description: 'Set up infrastructure',
            status: 'implementing',
            estimate: 21,
            createdAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['INFRA-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // When: I run "fspec show-work-unit INFRA-001"
      const result = await showWorkUnit({
        workUnitId: 'INFRA-001',
        cwd: tmpDir,
      });

      // Then: The output should NOT contain any warning about estimate size
      expect(result.systemReminders).toBeUndefined();
    });
  });

  describe('Scenario: AI follows warning guidance to break down large work unit', () => {
    it('should provide step-by-step workflow guidance in warning', async () => {
      // Given: A work unit "STORY-007" exists with type "story"
      const workUnitsData: WorkUnitsData = {
        workUnits: {
          'STORY-007': {
            id: 'STORY-007',
            type: 'story',
            title: 'User dashboard',
            description: 'Build user dashboard',
            status: 'specifying',
            createdAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['STORY-007'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // And: STORY-007 has a feature file with multiple scenario groupings
      const featureFile = join(featuresDir, 'user-dashboard.feature');
      await writeFile(
        featureFile,
        `@STORY-007
Feature: User Dashboard

  Background: User Story
    As a user
    I want a dashboard
    So that I can see my data

  Scenario: View profile
    Given I am logged in
    When I view my profile
    Then I see my data

  Scenario: Edit profile
    Given I am logged in
    When I edit my profile
    Then my changes are saved

  Scenario: View analytics
    Given I am logged in
    When I view analytics
    Then I see charts
`
      );

      // When: I set estimate to 21
      await updateWorkUnitEstimate({
        workUnitId: 'STORY-007',
        estimate: 21,
        cwd: tmpDir,
      });

      // And: I run show-work-unit
      const result = await showWorkUnit({
        workUnitId: 'STORY-007',
        cwd: tmpDir,
      });

      // Then: The output should contain step-by-step workflow guidance
      expect(result.systemReminders).toBeDefined();

      const largeEstimateReminder = result.systemReminders!.find(r =>
        r.includes('LARGE ESTIMATE WARNING')
      );
      expect(largeEstimateReminder).toBeDefined();
      expect(largeEstimateReminder).toContain('<system-reminder>');
      expect(largeEstimateReminder).toContain('REVIEW FEATURE FILE');
      expect(largeEstimateReminder).toContain('IDENTIFY BOUNDARIES');
      expect(largeEstimateReminder).toContain('CREATE CHILD WORK UNITS');
      expect(largeEstimateReminder).toContain('fspec create-story');
      expect(largeEstimateReminder).toContain('fspec add-dependency');
      expect(largeEstimateReminder).toContain('fspec create-epic');
    });
  });

  describe('Scenario: Adaptive warning guidance when feature file is missing', () => {
    it('should suggest creating feature file first when missing', async () => {
      // Given: A work unit "AUTH-008" exists with type "story"
      // And: AUTH-008 has NO feature file
      // And: AUTH-008 has estimate of 21 points (set directly to bypass validation)
      const workUnitsData: WorkUnitsData = {
        workUnits: {
          'AUTH-008': {
            id: 'AUTH-008',
            type: 'story',
            title: 'Password reset',
            description: 'Allow password reset',
            status: 'specifying',
            estimate: 21,
            createdAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-008'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      // When: I run show-work-unit
      const result = await showWorkUnit({
        workUnitId: 'AUTH-008',
        cwd: tmpDir,
      });

      // Then: The warning should suggest creating a feature file FIRST
      expect(result.systemReminders).toBeDefined();

      const largeEstimateReminder = result.systemReminders!.find(r =>
        r.includes('LARGE ESTIMATE WARNING')
      );
      expect(largeEstimateReminder).toBeDefined();
      expect(largeEstimateReminder).toContain('<system-reminder>');
      expect(largeEstimateReminder).toContain('CREATE FEATURE FILE FIRST');
      expect(largeEstimateReminder).toContain(
        'fspec generate-scenarios AUTH-008'
      );
    });
  });

  describe('Scenario: Warning stops when estimate is reduced to acceptable range', () => {
    it('should not show warning when estimate is reduced to 8 points', async () => {
      // Given: A work unit "BUG-009" with estimate of 8 points
      const workUnitsData: WorkUnitsData = {
        workUnits: {
          'BUG-009': {
            id: 'BUG-009',
            type: 'bug',
            title: 'Fix login bug',
            description: 'Login fails',
            status: 'testing',
            estimate: 8,
            rules: ['Login must work'],
            examples: ['User logs in successfully'],
            createdAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: ['BUG-009'],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      const featureFile = join(featuresDir, 'fix-login-bug.feature');
      await writeFile(
        featureFile,
        `@BUG-009
Feature: Fix login bug

  Background: User Story
    As a developer
    I want to fix login
    So that users can access

  Scenario: Fix login
    Given login is broken
    When I fix it
    Then users can log in
`
      );

      // When: I run show-work-unit
      const result = await showWorkUnit({
        workUnitId: 'BUG-009',
        cwd: tmpDir,
      });

      // Then: The output should NOT contain warning about estimate size
      expect(result.systemReminders).toBeUndefined();
    });
  });

  describe('Scenario: Warning stops when work unit is marked done', () => {
    it('should not show warning when work unit is done', async () => {
      // Given: A work unit "STORY-010" with estimate of 21 points and status done
      const workUnitsData: WorkUnitsData = {
        workUnits: {
          'STORY-010': {
            id: 'STORY-010',
            type: 'story',
            title: 'User profile',
            description: 'Build profile page',
            status: 'done',
            estimate: 21,
            createdAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: ['STORY-010'],
          blocked: [],
        },
      };

      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

      const featureFile = join(featuresDir, 'user-profile.feature');
      await writeFile(
        featureFile,
        `@STORY-010
Feature: User profile

  Background: User Story
    As a user
    I want a profile page
    So that I can manage my info

  Scenario: View profile
    Given I am logged in
    When I view profile
    Then I see my info
`
      );

      // When: I run show-work-unit
      const result = await showWorkUnit({
        workUnitId: 'STORY-010',
        cwd: tmpDir,
      });

      // Then: The output should NOT contain warning about estimate size
      expect(result.systemReminders).toBeUndefined();
    });
  });
});
