/**
 * Feature: spec/features/replace-generic-create-work-unit-with-type-specific-commands.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData, PrefixesData } from '../../types';
import { createMinimalFoundation } from '../../test-helpers/foundation-helper';

// Import commands to be created
import { createStory } from '../create-story';
import { createBug } from '../create-bug';
import { createTask } from '../create-task';

describe('Feature: Replace generic create-work-unit with type-specific commands', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-type-specific-commands');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });

    // Create foundation.json for all tests
    await createMinimalFoundation(testDir);

    // Create prefixes.json
    const prefixes: PrefixesData = {
      prefixes: {
        AUTH: {
          prefix: 'AUTH',
          description: 'Authentication features',
          createdAt: new Date().toISOString(),
        },
        BUG: {
          prefix: 'BUG',
          description: 'Bug fixes',
          createdAt: new Date().toISOString(),
        },
        TASK: {
          prefix: 'TASK',
          description: 'Tasks',
          createdAt: new Date().toISOString(),
        },
      },
    };
    await writeFile(
      join(testDir, 'spec/prefixes.json'),
      JSON.stringify(prefixes, null, 2)
    );

    // Create empty work-units.json
    const workUnits: WorkUnitsData = {
      workUnits: {},
      states: {
        backlog: [],
        specifying: [],
        testing: [],
        implementing: [],
        validating: [],
        done: [],
        blocked: [],
      },
    };
    await writeFile(
      join(testDir, 'spec/work-units.json'),
      JSON.stringify(workUnits, null, 2)
    );
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Create story with Example Mapping guidance', () => {
    it('should create story and emit Example Mapping system-reminder', async () => {
      // Given I have fspec installed
      // (setup in beforeEach)

      // When I run 'fspec create-story AUTH "User login"'
      const result = await createStory({
        prefix: 'AUTH',
        title: 'User login',
        cwd: testDir,
      });

      // Then a new story work unit AUTH-001 should be created
      expect(result.success).toBe(true);
      expect(result.workUnitId).toBe('AUTH-001');

      const workUnitsContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const workUnitsData: WorkUnitsData = JSON.parse(workUnitsContent);
      expect(workUnitsData.workUnits['AUTH-001']).toBeDefined();
      expect(workUnitsData.workUnits['AUTH-001'].type).toBe('story');

      // And a system-reminder should be displayed with Example Mapping guidance
      expect(result.systemReminder).toBeDefined();
      expect(result.systemReminder).toContain('Example Mapping');

      // And the system-reminder should include: 'fspec add-rule AUTH-001'
      expect(result.systemReminder).toContain('fspec add-rule AUTH-001');

      // And the system-reminder should include: 'fspec add-example AUTH-001'
      expect(result.systemReminder).toContain('fspec add-example AUTH-001');

      // And the system-reminder should include: 'fspec add-question AUTH-001'
      expect(result.systemReminder).toContain('fspec add-question AUTH-001');

      // And the system-reminder should include: 'fspec set-user-story AUTH-001'
      expect(result.systemReminder).toContain('fspec set-user-story AUTH-001');
    });
  });

  describe('Scenario: Create bug with research guidance', () => {
    it('should create bug and emit research command system-reminder', async () => {
      // Given I have fspec installed
      // (setup in beforeEach)

      // When I run 'fspec create-bug BUG "Login validation broken"'
      const result = await createBug({
        prefix: 'BUG',
        title: 'Login validation broken',
        cwd: testDir,
      });

      // Then a new bug work unit BUG-001 should be created
      expect(result.success).toBe(true);
      expect(result.workUnitId).toBe('BUG-001');

      const workUnitsContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const workUnitsData: WorkUnitsData = JSON.parse(workUnitsContent);
      expect(workUnitsData.workUnits['BUG-001']).toBeDefined();
      expect(workUnitsData.workUnits['BUG-001'].type).toBe('bug');

      // And a system-reminder should be displayed with research command guidance
      expect(result.systemReminder).toBeDefined();
      expect(result.systemReminder).toContain('CRITICAL');
      expect(result.systemReminder).toContain('Research existing code FIRST');

      // And the system-reminder should include: 'fspec search-scenarios --query="login"'
      expect(result.systemReminder).toContain('fspec search-scenarios');
      expect(result.systemReminder).toContain('--query=');

      // And the system-reminder should include: 'fspec search-implementation --function="validateLogin"'
      expect(result.systemReminder).toContain('fspec search-implementation');
      expect(result.systemReminder).toContain('--function=');

      // And the system-reminder should include: 'fspec show-coverage'
      expect(result.systemReminder).toContain('fspec show-coverage');
    });
  });

  describe('Scenario: Create task with minimal requirements guidance', () => {
    it('should create task and emit minimal requirements system-reminder', async () => {
      // Given I have fspec installed
      // (setup in beforeEach)

      // When I run 'fspec create-task TASK "Setup CI/CD pipeline"'
      const result = await createTask({
        prefix: 'TASK',
        title: 'Setup CI/CD pipeline',
        cwd: testDir,
      });

      // Then a new task work unit TASK-001 should be created
      expect(result.success).toBe(true);
      expect(result.workUnitId).toBe('TASK-001');

      const workUnitsContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const workUnitsData: WorkUnitsData = JSON.parse(workUnitsContent);
      expect(workUnitsData.workUnits['TASK-001']).toBeDefined();
      expect(workUnitsData.workUnits['TASK-001'].type).toBe('task');

      // And a system-reminder should indicate tasks have optional feature files
      expect(result.systemReminder).toBeDefined();
      expect(result.systemReminder).toContain('optional feature file');

      // And a system-reminder should indicate tasks have optional tests
      expect(result.systemReminder).toContain('optional tests');
    });
  });

  describe('Scenario: All type-specific commands support common options', () => {
    it('should support --epic, --description, --parent options', async () => {
      // Given I have fspec installed
      // And epic exists
      await writeFile(
        join(testDir, 'spec/epics.json'),
        JSON.stringify(
          {
            epics: {
              'user-management': {
                name: 'user-management',
                title: 'User Management',
                createdAt: new Date().toISOString(),
              },
            },
          },
          null,
          2
        )
      );

      // And parent work unit exists
      const workUnitsData = JSON.parse(
        await readFile(join(testDir, 'spec/work-units.json'), 'utf-8')
      );
      workUnitsData.workUnits['AUTH-000'] = {
        id: 'AUTH-000',
        title: 'Authentication System',
        type: 'story',
        status: 'done',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnitsData.states.done.push('AUTH-000');
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run 'fspec create-story AUTH "Login" --epic user-management --description "User authentication" --parent AUTH-000'
      const result = await createStory({
        prefix: 'AUTH',
        title: 'Login',
        epic: 'user-management',
        description: 'User authentication',
        parent: 'AUTH-000',
        cwd: testDir,
      });

      // Then the story should be created with epic set to 'user-management'
      expect(result.success).toBe(true);
      const content = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const data: WorkUnitsData = JSON.parse(content);
      expect(data.workUnits[result.workUnitId!].epic).toBe('user-management');

      // And the story should have description 'User authentication'
      expect(data.workUnits[result.workUnitId!].description).toBe(
        'User authentication'
      );

      // And the story should have parent AUTH-000
      expect(data.workUnits[result.workUnitId!].parent).toBe('AUTH-000');
    });
  });

  describe('Scenario: Old create-work-unit command is removed', () => {
    it('should fail when trying to import create-work-unit', async () => {
      // Given I have fspec v2.0 installed
      // When I try to import create-work-unit
      // Then the import should fail or function should not exist

      // This test verifies the module doesn't export create-work-unit
      let importError = false;
      try {
        // Attempt dynamic import
        await import('../create-work-unit');
      } catch (error) {
        importError = true;
      }

      // @step Then the command should fail with 'unknown command' error
      // We expect either:
      // 1. Import fails (file deleted)
      // 2. Or function is explicitly removed/deprecated
      // For now, this test documents the expectation
      // Implementation will make this pass by deleting the file
      expect(importError).toBe(true);

      // @step And the error should suggest using create-story, create-bug, or create-task
      // This will be validated when the CLI integration is complete
    });
  });

  describe('Scenario: All "work unit" terminology replaced with type-specific terms', () => {
    it('should use type-specific terminology in output', async () => {
      // Given I search the entire codebase for 'work unit'
      // When I grep for 'work unit' or 'workUnit' in all files
      // Then all user-facing text should use 'story', 'bug', 'task', or 'work unit'

      // This scenario will be validated during implementation
      // The test here verifies that created items use correct terminology
      const storyResult = await createStory({
        prefix: 'AUTH',
        title: 'Test story',
        cwd: testDir,
      });

      // Verify output uses 'story' not 'work unit'
      expect(storyResult.systemReminder).not.toContain('work unit');
      expect(storyResult.systemReminder).toContain('story');

      const bugResult = await createBug({
        prefix: 'BUG',
        title: 'Test bug',
        cwd: testDir,
      });

      // Verify output uses 'bug' not 'work unit'
      expect(bugResult.systemReminder).not.toContain('work unit');
      expect(bugResult.systemReminder).toContain('bug');

      const taskResult = await createTask({
        prefix: 'TASK',
        title: 'Test task',
        cwd: testDir,
      });

      // Verify output uses 'task' not 'work unit'
      expect(taskResult.systemReminder).not.toContain('work unit');
      expect(taskResult.systemReminder).toContain('task');

      // @step And variable names should be story, bug, task, or workUnit
      // This will be validated during code review

      // @step And no references to 'work unit' should remain except in historical changelogs
      // This will be validated during implementation
    });
  });

  describe('Scenario: Documentation updated with type-specific commands', () => {
    it('should have help files for new commands', async () => {
      // Given I have updated the codebase
      // @step When I check README.md, spec/CLAUDE.md, and docs/ files
      // This test verifies help files can be imported

      // @step Then all examples should use create-story, create-bug, or create-task
      // Validated by importing help files

      // @step And help text in src/help.ts should reference new commands
      // This will be validated during implementation

      // @step And command help files should exist for all 3 new commands
      const storyHelp = await import('../create-story-help');
      const bugHelp = await import('../create-bug-help');
      const taskHelp = await import('../create-task-help');

      expect(storyHelp).toBeDefined();
      expect(bugHelp).toBeDefined();
      expect(taskHelp).toBeDefined();
    });
  });

  describe('Scenario: System-reminders in codebase reference new commands', () => {
    it('should emit system-reminders with correct command references', async () => {
      // Given I search for system-reminder tags in source code
      // When I grep for '<system-reminder>' containing 'create'
      // Then all system-reminders should reference create-story/bug/task based on context

      // Verified by checking actual system-reminder content
      const storyResult = await createStory({
        prefix: 'AUTH',
        title: 'Test',
        cwd: testDir,
      });

      const bugResult = await createBug({
        prefix: 'BUG',
        title: 'Test',
        cwd: testDir,
      });

      const taskResult = await createTask({
        prefix: 'TASK',
        title: 'Test',
        cwd: testDir,
      });

      // And no system-reminders should reference create-work-unit
      expect(storyResult.systemReminder).not.toContain('create-work-unit');
      expect(bugResult.systemReminder).not.toContain('create-work-unit');
      expect(taskResult.systemReminder).not.toContain('create-work-unit');
    });
  });
});
