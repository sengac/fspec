import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { createEpic } from '../create-epic';
import { createPrefix } from '../create-prefix';
import { updatePrefix } from '../update-prefix';
import { showEpic } from '../show-epic';
import { listEpics } from '../list-epics';
import { deleteEpic } from '../delete-epic';

describe('Feature: Epic and Prefix Management', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-epic-and-prefix-management');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Create epic', () => {
    it('should create epic with ID and description', async () => {
      // Given I have a project with spec directory
      // (created in beforeEach)

      // When I run "fspec create-epic epic-user-management 'User Management Epic'"
      const result = await createEpic({
        epicId: 'epic-user-management',
        title: 'User Management Epic',
        description: 'All features related to user management',
        cwd: testDir,
      });

      // Then the epic should be created successfully
      expect(result.success).toBe(true);

      // And spec/epics.json should contain the new epic
      const epicsFile = join(testDir, 'spec', 'epics.json');
      const content = await readFile(epicsFile, 'utf-8');
      const data = JSON.parse(content);

      expect(data.epics['epic-user-management']).toBeDefined();
      expect(data.epics['epic-user-management'].id).toBe('epic-user-management');
      expect(data.epics['epic-user-management'].title).toBe('User Management Epic');
      expect(data.epics['epic-user-management'].description).toBe('All features related to user management');
    });
  });

  describe('Scenario: Create prefix for work unit IDs', () => {
    it('should register prefix for work unit IDs', async () => {
      // Given I have a project with spec directory
      // (created in beforeEach)

      // When I run "fspec create-prefix AUTH 'Authentication work units'"
      const result = await createPrefix({
        prefix: 'AUTH',
        description: 'Authentication work units',
        cwd: testDir,
      });

      // Then the prefix should be created successfully
      expect(result.success).toBe(true);

      // And spec/prefixes.json should contain the new prefix
      const prefixesFile = join(testDir, 'spec', 'prefixes.json');
      const content = await readFile(prefixesFile, 'utf-8');
      const data = JSON.parse(content);

      expect(data.prefixes['AUTH']).toBeDefined();
      expect(data.prefixes['AUTH'].prefix).toBe('AUTH');
      expect(data.prefixes['AUTH'].description).toBe('Authentication work units');
    });
  });

  describe('Scenario: Link prefix to epic', () => {
    it('should associate prefix with epic', async () => {
      // Given I have an existing epic
      await createEpic({
        epicId: 'epic-user-management',
        title: 'User Management',
        cwd: testDir,
      });

      // And I have an existing prefix
      await createPrefix({
        prefix: 'AUTH',
        description: 'Authentication',
        cwd: testDir,
      });

      // When I run "fspec update-prefix AUTH --epic=epic-user-management"
      const result = await updatePrefix({
        prefix: 'AUTH',
        epicId: 'epic-user-management',
        cwd: testDir,
      });

      // Then the prefix should be linked to the epic
      expect(result.success).toBe(true);

      // And spec/prefixes.json should show the association
      const prefixesFile = join(testDir, 'spec', 'prefixes.json');
      const content = await readFile(prefixesFile, 'utf-8');
      const data = JSON.parse(content);

      expect(data.prefixes['AUTH'].epicId).toBe('epic-user-management');
    });
  });

  describe('Scenario: Show epic progress', () => {
    it('should calculate epic completion percentage', async () => {
      // Given I have an epic with linked work units
      const epicsFile = join(testDir, 'spec', 'epics.json');
      await writeFile(
        epicsFile,
        JSON.stringify({
          epics: {
            'epic-auth': {
              id: 'epic-auth',
              title: 'Authentication',
              description: 'Auth features',
            },
          },
        }, null, 2)
      );

      const prefixesFile = join(testDir, 'spec', 'prefixes.json');
      await writeFile(
        prefixesFile,
        JSON.stringify({
          prefixes: {
            'AUTH': {
              prefix: 'AUTH',
              description: 'Auth work units',
              epicId: 'epic-auth',
            },
          },
        }, null, 2)
      );

      const workUnitsFile = join(testDir, 'spec', 'work-units.json');
      await writeFile(
        workUnitsFile,
        JSON.stringify({
          workUnits: {
            'AUTH-001': {
              id: 'AUTH-001',
              title: 'Login',
              status: 'done',
              epic: 'epic-auth',
            },
            'AUTH-002': {
              id: 'AUTH-002',
              title: 'Logout',
              status: 'implementing',
              epic: 'epic-auth',
            },
            'AUTH-003': {
              id: 'AUTH-003',
              title: 'Register',
              status: 'done',
              epic: 'epic-auth',
            },
          },
          states: {
            done: ['AUTH-001', 'AUTH-003'],
            implementing: ['AUTH-002'],
          },
        }, null, 2)
      );

      // When I run "fspec show-epic epic-auth"
      const result = await showEpic({
        epicId: 'epic-auth',
        cwd: testDir,
      });

      // Then the output should show epic details
      expect(result.epic).toBeDefined();
      expect(result.epic.id).toBe('epic-auth');

      // And the output should show total work units
      expect(result.totalWorkUnits).toBe(3);

      // And the output should show completed work units
      expect(result.completedWorkUnits).toBe(2);

      // And the output should show completion percentage
      expect(result.completionPercentage).toBeCloseTo(66.67, 1);
    });
  });

  describe('Scenario: Reject invalid epic ID format', () => {
    it('should validate epic ID format', async () => {
      // Given I have a project with spec directory
      // (created in beforeEach)

      // When I run "fspec create-epic InvalidEpicID 'Bad Format'"
      // Then the command should fail with validation error
      await expect(
        createEpic({
          epicId: 'InvalidEpicID',
          title: 'Bad Format',
          cwd: testDir,
        })
      ).rejects.toThrow('Epic ID must be lowercase-with-hyphens format');

      // When I run "fspec create-epic epic_underscore 'Bad Format'"
      // Then the command should fail with validation error
      await expect(
        createEpic({
          epicId: 'epic_underscore',
          title: 'Bad Format',
          cwd: testDir,
        })
      ).rejects.toThrow('Epic ID must be lowercase-with-hyphens format');

      // When I run "fspec create-epic epic-valid 'Good Format'"
      // Then the command should succeed
      const result = await createEpic({
        epicId: 'epic-valid',
        title: 'Good Format',
        cwd: testDir,
      });
      expect(result.success).toBe(true);
    });
  });

  describe('Scenario: Reject invalid prefix format', () => {
    it('should validate prefix format', async () => {
      // Given I have a project with spec directory
      // (created in beforeEach)

      // When I run "fspec create-prefix lowercase 'Bad Format'"
      // Then the command should fail with validation error
      await expect(
        createPrefix({
          prefix: 'lowercase',
          description: 'Bad Format',
          cwd: testDir,
        })
      ).rejects.toThrow('Prefix must be 2-6 uppercase letters');

      // When I run "fspec create-prefix A 'Too Short'"
      // Then the command should fail with validation error
      await expect(
        createPrefix({
          prefix: 'A',
          description: 'Too Short',
          cwd: testDir,
        })
      ).rejects.toThrow('Prefix must be 2-6 uppercase letters');

      // When I run "fspec create-prefix TOOLONG 'Too Long'"
      // Then the command should fail with validation error
      await expect(
        createPrefix({
          prefix: 'TOOLONG',
          description: 'Too Long',
          cwd: testDir,
        })
      ).rejects.toThrow('Prefix must be 2-6 uppercase letters');

      // When I run "fspec create-prefix AUTH 'Good Format'"
      // Then the command should succeed
      const result = await createPrefix({
        prefix: 'AUTH',
        description: 'Good Format',
        cwd: testDir,
      });
      expect(result.success).toBe(true);
    });
  });

  describe('Scenario: List all epics with progress', () => {
    it('should list all epics with progress stats', async () => {
      // Given I have multiple epics with work units
      const epicsFile = join(testDir, 'spec', 'epics.json');
      await writeFile(
        epicsFile,
        JSON.stringify({
          epics: {
            'epic-auth': {
              id: 'epic-auth',
              title: 'Authentication',
            },
            'epic-dashboard': {
              id: 'epic-dashboard',
              title: 'Dashboard',
            },
          },
        }, null, 2)
      );

      const workUnitsFile = join(testDir, 'spec', 'work-units.json');
      await writeFile(
        workUnitsFile,
        JSON.stringify({
          workUnits: {
            'AUTH-001': {
              id: 'AUTH-001',
              status: 'done',
              epic: 'epic-auth',
            },
            'AUTH-002': {
              id: 'AUTH-002',
              status: 'implementing',
              epic: 'epic-auth',
            },
            'DASH-001': {
              id: 'DASH-001',
              status: 'done',
              epic: 'epic-dashboard',
            },
          },
          states: {
            done: ['AUTH-001', 'DASH-001'],
            implementing: ['AUTH-002'],
          },
        }, null, 2)
      );

      // When I run "fspec list-epics"
      const result = await listEpics({ cwd: testDir });

      // Then the output should list all epics
      expect(result.epics).toHaveLength(2);

      // And each epic should show progress
      const authEpic = result.epics.find(e => e.id === 'epic-auth');
      expect(authEpic).toBeDefined();
      expect(authEpic?.totalWorkUnits).toBe(2);
      expect(authEpic?.completedWorkUnits).toBe(1);
      expect(authEpic?.completionPercentage).toBe(50);

      const dashEpic = result.epics.find(e => e.id === 'epic-dashboard');
      expect(dashEpic).toBeDefined();
      expect(dashEpic?.totalWorkUnits).toBe(1);
      expect(dashEpic?.completedWorkUnits).toBe(1);
      expect(dashEpic?.completionPercentage).toBe(100);
    });
  });

  describe('Scenario: List epics when epics.json does not exist', () => {
    it('should return empty list when epics.json does not exist', async () => {
      // Given I have a project with spec directory
      // (created in beforeEach)

      // And the file "spec/epics.json" does not exist
      // (it doesn't exist by default)

      // When I run "fspec list-epics"
      const result = await listEpics({ cwd: testDir });

      // Then the command should succeed
      expect(result).toBeDefined();

      // And the output should indicate no epics found
      expect(result.epics).toHaveLength(0);

      // And the file "spec/epics.json" should NOT be created
      const { access } = await import('fs/promises');
      const epicsFile = join(testDir, 'spec', 'epics.json');
      await expect(access(epicsFile)).rejects.toThrow();
    });
  });

  describe('Scenario: Delete epic and unlink work units', () => {
    it('should delete epic and clear work unit references', async () => {
      // Given I have an epic with linked work units
      const epicsFile = join(testDir, 'spec', 'epics.json');
      await writeFile(
        epicsFile,
        JSON.stringify({
          epics: {
            'epic-auth': {
              id: 'epic-auth',
              title: 'Authentication',
            },
          },
        }, null, 2)
      );

      const prefixesFile = join(testDir, 'spec', 'prefixes.json');
      await writeFile(
        prefixesFile,
        JSON.stringify({
          prefixes: {
            'AUTH': {
              prefix: 'AUTH',
              description: 'Auth',
              epicId: 'epic-auth',
            },
          },
        }, null, 2)
      );

      const workUnitsFile = join(testDir, 'spec', 'work-units.json');
      await writeFile(
        workUnitsFile,
        JSON.stringify({
          workUnits: {
            'AUTH-001': {
              id: 'AUTH-001',
              epic: 'epic-auth',
            },
          },
          states: {
            implementing: ['AUTH-001'],
          },
        }, null, 2)
      );

      // When I run "fspec delete-epic epic-auth"
      const result = await deleteEpic({
        epicId: 'epic-auth',
        cwd: testDir,
      });

      // Then the epic should be deleted
      expect(result.success).toBe(true);

      // And spec/epics.json should not contain the epic
      const epicsContent = await readFile(epicsFile, 'utf-8');
      const epicsData = JSON.parse(epicsContent);
      expect(epicsData.epics['epic-auth']).toBeUndefined();

      // And prefix should have epic reference removed
      const prefixesContent = await readFile(prefixesFile, 'utf-8');
      const prefixesData = JSON.parse(prefixesContent);
      expect(prefixesData.prefixes['AUTH'].epicId).toBeUndefined();

      // And work units should have epic reference removed
      const workUnitsContent = await readFile(workUnitsFile, 'utf-8');
      const workUnitsData = JSON.parse(workUnitsContent);
      expect(workUnitsData.workUnits['AUTH-001'].epic).toBeUndefined();
    });
  });
});
