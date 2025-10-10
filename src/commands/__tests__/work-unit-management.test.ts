import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData, PrefixesData, EpicsData } from '../../types';

// Import commands (to be created)
import { createWorkUnit } from '../create-work-unit';
import { updateWorkUnit } from '../update-work-unit';
import { showWorkUnit } from '../show-work-unit';
import { listWorkUnits } from '../list-work-units';
import { deleteWorkUnit } from '../delete-work-unit';
import { validateWorkUnits } from '../validate-work-units';

describe('Feature: Work Unit Management', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-work-unit-management');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Create work unit with auto-incrementing ID', () => {
    it('should create work unit with auto-incremented ID', async () => {
      // Given I have a project with spec directory
      // And the prefix "AUTH" is registered in spec/prefixes.json
      const prefixes: PrefixesData = {
        prefixes: {
          AUTH: {
            prefix: 'AUTH',
            description: 'Authentication features',
            createdAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(join(testDir, 'spec/prefixes.json'), JSON.stringify(prefixes, null, 2));

      // And no work units exist with prefix "AUTH"
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
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec create-work-unit AUTH 'Implement OAuth login'"
      const result = await createWorkUnit({
        prefix: 'AUTH',
        title: 'Implement OAuth login',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And a work unit "AUTH-001" should be created in spec/work-units.json
      const updatedContent = await readFile(join(testDir, 'spec/work-units.json'), 'utf-8');
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-001']).toBeDefined();

      // And the work unit should have title "Implement OAuth login"
      expect(updatedData.workUnits['AUTH-001'].title).toBe('Implement OAuth login');

      // And the work unit should have status "backlog"
      expect(updatedData.workUnits['AUTH-001'].status).toBe('backlog');

      // And the work unit should have createdAt timestamp
      expect(updatedData.workUnits['AUTH-001'].createdAt).toBeDefined();

      // And the work unit should have updatedAt timestamp
      expect(updatedData.workUnits['AUTH-001'].updatedAt).toBeDefined();

      // And the states.backlog array should contain "AUTH-001"
      expect(updatedData.states.backlog).toContain('AUTH-001');
    });
  });

  describe('Scenario: Create second work unit with incremented ID', () => {
    it('should increment ID for second work unit', async () => {
      // Given I have a project with spec directory
      // And the prefix "AUTH" is registered
      const prefixes: PrefixesData = {
        prefixes: {
          AUTH: {
            prefix: 'AUTH',
            description: 'Authentication features',
            createdAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(join(testDir, 'spec/prefixes.json'), JSON.stringify(prefixes, null, 2));

      // And a work unit "AUTH-001" exists
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'First work unit',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec create-work-unit AUTH 'Add password reset'"
      const result = await createWorkUnit({
        prefix: 'AUTH',
        title: 'Add password reset',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And a work unit "AUTH-002" should be created
      const updatedContent = await readFile(join(testDir, 'spec/work-units.json'), 'utf-8');
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-002']).toBeDefined();

      // And the work unit should have status "backlog"
      expect(updatedData.workUnits['AUTH-002'].status).toBe('backlog');

      // And the states.backlog array should contain "AUTH-002"
      expect(updatedData.states.backlog).toContain('AUTH-002');
    });
  });

  describe('Scenario: Create work unit with epic assignment', () => {
    it('should assign work unit to epic', async () => {
      // Given I have a project with spec directory
      // And the prefix "AUTH" is registered
      const prefixes: PrefixesData = {
        prefixes: {
          AUTH: {
            prefix: 'AUTH',
            description: 'Authentication features',
            createdAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(join(testDir, 'spec/prefixes.json'), JSON.stringify(prefixes, null, 2));

      // And an epic "epic-user-management" exists
      const epics: EpicsData = {
        epics: {
          'epic-user-management': {
            id: 'epic-user-management',
            title: 'User Management',
            createdAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(join(testDir, 'spec/epics.json'), JSON.stringify(epics, null, 2));

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
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec create-work-unit AUTH 'OAuth integration' --epic=epic-user-management"
      const result = await createWorkUnit({
        prefix: 'AUTH',
        title: 'OAuth integration',
        epic: 'epic-user-management',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the work unit "AUTH-001" should have epic "epic-user-management"
      const updatedContent = await readFile(join(testDir, 'spec/work-units.json'), 'utf-8');
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-001'].epic).toBe('epic-user-management');

      // And the epic should reference work unit "AUTH-001"
      const epicsContent = await readFile(join(testDir, 'spec/epics.json'), 'utf-8');
      const epicsData: EpicsData = JSON.parse(epicsContent);

      expect(epicsData.epics['epic-user-management'].workUnits).toContain('AUTH-001');
    });
  });

  describe('Scenario: Create work unit with description', () => {
    it('should create work unit with description field', async () => {
      // Given I have a project with spec directory
      // And the prefix "AUTH" is registered
      const prefixes: PrefixesData = {
        prefixes: {
          AUTH: {
            prefix: 'AUTH',
            description: 'Authentication features',
            createdAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(join(testDir, 'spec/prefixes.json'), JSON.stringify(prefixes, null, 2));

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
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec create-work-unit AUTH 'OAuth login' --description='Add OAuth 2.0 with Google and GitHub'"
      const result = await createWorkUnit({
        prefix: 'AUTH',
        title: 'OAuth login',
        description: 'Add OAuth 2.0 with Google and GitHub',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the work unit should have description "Add OAuth 2.0 with Google and GitHub"
      const updatedContent = await readFile(join(testDir, 'spec/work-units.json'), 'utf-8');
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-001'].description).toBe('Add OAuth 2.0 with Google and GitHub');
    });
  });

  describe('Scenario: Create child work unit with parent', () => {
    it('should create child work unit with parent relationship', async () => {
      // Given I have a project with spec directory
      // And the prefix "AUTH" is registered
      const prefixes: PrefixesData = {
        prefixes: {
          AUTH: {
            prefix: 'AUTH',
            description: 'Authentication features',
            createdAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(join(testDir, 'spec/prefixes.json'), JSON.stringify(prefixes, null, 2));

      // And a work unit "AUTH-001" exists with title "OAuth integration"
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth integration',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec create-work-unit AUTH 'Google provider' --parent=AUTH-001"
      const result = await createWorkUnit({
        prefix: 'AUTH',
        title: 'Google provider',
        parent: 'AUTH-001',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the work unit "AUTH-002" should be created
      const updatedContent = await readFile(join(testDir, 'spec/work-units.json'), 'utf-8');
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-002']).toBeDefined();

      // And the work unit "AUTH-002" should have parent "AUTH-001"
      expect(updatedData.workUnits['AUTH-002'].parent).toBe('AUTH-001');

      // And the work unit "AUTH-001" children array should contain "AUTH-002"
      expect(updatedData.workUnits['AUTH-001'].children).toContain('AUTH-002');
    });
  });

  describe('Scenario: Attempt to create work unit with unregistered prefix', () => {
    it('should fail when prefix is not registered', async () => {
      // Given I have a project with spec directory
      // And the prefix "INVALID" is not registered
      const prefixes: PrefixesData = {
        prefixes: {},
      };
      await writeFile(join(testDir, 'spec/prefixes.json'), JSON.stringify(prefixes, null, 2));

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
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec create-work-unit INVALID 'Some work'"
      // Then the command should fail
      await expect(
        createWorkUnit({
          prefix: 'INVALID',
          title: 'Some work',
          cwd: testDir,
        })
      ).rejects.toThrow("Prefix 'INVALID' is not registered");

      // And the error should suggest "Run 'fspec create-prefix INVALID' first"
      try {
        await createWorkUnit({
          prefix: 'INVALID',
          title: 'Some work',
          cwd: testDir,
        });
      } catch (error: unknown) {
        if (error instanceof Error) {
          expect(error.message).toContain("Run 'fspec create-prefix");
        }
      }
    });
  });

  describe('Scenario: Attempt to create work unit with missing title', () => {
    it('should fail when title is missing', async () => {
      // Given I have a project with spec directory
      // And the prefix "AUTH" is registered
      const prefixes: PrefixesData = {
        prefixes: {
          AUTH: {
            prefix: 'AUTH',
            description: 'Authentication features',
            createdAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(join(testDir, 'spec/prefixes.json'), JSON.stringify(prefixes, null, 2));

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
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec create-work-unit AUTH"
      // Then the command should fail
      await expect(
        createWorkUnit({
          prefix: 'AUTH',
          title: '',
          cwd: testDir,
        })
      ).rejects.toThrow('Title is required');
    });
  });

  describe('Scenario: Attempt to create child with non-existent parent', () => {
    it('should fail when parent does not exist', async () => {
      // Given I have a project with spec directory
      // And the prefix "AUTH" is registered
      const prefixes: PrefixesData = {
        prefixes: {
          AUTH: {
            prefix: 'AUTH',
            description: 'Authentication features',
            createdAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(join(testDir, 'spec/prefixes.json'), JSON.stringify(prefixes, null, 2));

      // And no work unit "AUTH-999" exists
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
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec create-work-unit AUTH 'Child work' --parent=AUTH-999"
      // Then the command should fail
      await expect(
        createWorkUnit({
          prefix: 'AUTH',
          title: 'Child work',
          parent: 'AUTH-999',
          cwd: testDir,
        })
      ).rejects.toThrow("Parent work unit 'AUTH-999' does not exist");
    });
  });

  describe('Scenario: Update work unit title', () => {
    it('should update title of existing work unit', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with title "Old title"
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Old title',
            status: 'backlog',
            createdAt: '2025-01-01T00:00:00.000Z',
            updatedAt: '2025-01-01T00:00:00.000Z',
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec update-work-unit AUTH-001 --title='New title'"
      const result = await updateWorkUnit({
        workUnitId: 'AUTH-001',
        title: 'New title',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the work unit "AUTH-001" should have title "New title"
      const updatedContent = await readFile(join(testDir, 'spec/work-units.json'), 'utf-8');
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-001'].title).toBe('New title');

      // And the updatedAt timestamp should be updated
      expect(updatedData.workUnits['AUTH-001'].updatedAt).not.toBe('2025-01-01T00:00:00.000Z');
    });
  });

  describe('Scenario: Update work unit description', () => {
    it('should update description of existing work unit', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Some work',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec update-work-unit AUTH-001 --description='Updated description'"
      const result = await updateWorkUnit({
        workUnitId: 'AUTH-001',
        description: 'Updated description',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the work unit should have description "Updated description"
      const updatedContent = await readFile(join(testDir, 'spec/work-units.json'), 'utf-8');
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-001'].description).toBe('Updated description');
    });
  });

  describe('Scenario: Update work unit epic', () => {
    it('should update epic assignment', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Some work',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // And an epic "epic-security" exists
      const epics: EpicsData = {
        epics: {
          'epic-security': {
            id: 'epic-security',
            title: 'Security Features',
            createdAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(join(testDir, 'spec/epics.json'), JSON.stringify(epics, null, 2));

      // When I run "fspec update-work-unit AUTH-001 --epic=epic-security"
      const result = await updateWorkUnit({
        workUnitId: 'AUTH-001',
        epic: 'epic-security',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the work unit should have epic "epic-security"
      const updatedContent = await readFile(join(testDir, 'spec/work-units.json'), 'utf-8');
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-001'].epic).toBe('epic-security');

      // And the epic should reference work unit "AUTH-001"
      const epicsContent = await readFile(join(testDir, 'spec/epics.json'), 'utf-8');
      const epicsData: EpicsData = JSON.parse(epicsContent);

      expect(epicsData.epics['epic-security'].workUnits).toContain('AUTH-001');
    });
  });

  describe('Scenario: Attempt to update work unit with invalid epic', () => {
    it('should fail when epic does not exist', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Some work',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // And no epic "epic-nonexistent" exists
      const epics: EpicsData = {
        epics: {},
      };
      await writeFile(join(testDir, 'spec/epics.json'), JSON.stringify(epics, null, 2));

      // When I run "fspec update-work-unit AUTH-001 --epic=epic-nonexistent"
      // Then the command should fail
      await expect(
        updateWorkUnit({
          workUnitId: 'AUTH-001',
          epic: 'epic-nonexistent',
          cwd: testDir,
        })
      ).rejects.toThrow("Epic 'epic-nonexistent' does not exist");
    });
  });

  describe('Scenario: Show single work unit details', () => {
    it('should display all details of a work unit', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with details
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth integration',
            description: 'Add OAuth 2.0 support',
            status: 'implementing',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec show-work-unit AUTH-001"
      const result = await showWorkUnit({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result).toBeDefined();

      // And the output should contain "AUTH-001"
      expect(result.id).toBe('AUTH-001');

      // And the output should contain "OAuth integration"
      expect(result.title).toBe('OAuth integration');

      // And the output should contain "implementing"
      expect(result.status).toBe('implementing');
    });
  });

  describe('Scenario: Show work unit with JSON output', () => {
    it('should output work unit as JSON', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Some work',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec show-work-unit AUTH-001 --output=json"
      const result = await showWorkUnit({
        workUnitId: 'AUTH-001',
        output: 'json',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result).toBeDefined();

      // And the JSON should have field "id" with value "AUTH-001"
      expect(result.id).toBe('AUTH-001');

      // And the JSON should have field "title"
      expect(result.title).toBeDefined();

      // And the JSON should have field "status"
      expect(result.status).toBeDefined();
    });
  });

  describe('Scenario: List all work units', () => {
    it('should list all work units', async () => {
      // Given I have a project with spec directory
      // And work units exist
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'done',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Password reset',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'User dashboard',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['DASH-001'],
          specifying: [],
          testing: [],
          implementing: ['AUTH-002'],
          validating: [],
          done: ['AUTH-001'],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec list-work-units"
      const result = await listWorkUnits({
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result).toBeDefined();

      // And the output should contain "AUTH-001"
      expect(result.workUnits.some(wu => wu.id === 'AUTH-001')).toBe(true);

      // And the output should contain "AUTH-002"
      expect(result.workUnits.some(wu => wu.id === 'AUTH-002')).toBe(true);

      // And the output should contain "DASH-001"
      expect(result.workUnits.some(wu => wu.id === 'DASH-001')).toBe(true);
    });
  });

  describe('Scenario: List work units filtered by status', () => {
    it('should filter work units by status', async () => {
      // Given I have a project with spec directory
      // And work units exist
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work 1',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Work 2',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Work 3',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001', 'DASH-001'],
          specifying: [],
          testing: [],
          implementing: ['AUTH-002'],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec list-work-units --status=backlog"
      const result = await listWorkUnits({
        status: 'backlog',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result).toBeDefined();

      // And the output should contain "AUTH-001"
      expect(result.workUnits.some(wu => wu.id === 'AUTH-001')).toBe(true);

      // And the output should contain "DASH-001"
      expect(result.workUnits.some(wu => wu.id === 'DASH-001')).toBe(true);

      // And the output should not contain "AUTH-002"
      expect(result.workUnits.some(wu => wu.id === 'AUTH-002')).toBe(false);
    });
  });

  describe('Scenario: List work units filtered by prefix', () => {
    it('should filter work units by prefix', async () => {
      // Given I have a project with spec directory
      // And work units exist
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth work',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'More auth',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Dashboard',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001', 'AUTH-002', 'DASH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec list-work-units --prefix=AUTH"
      const result = await listWorkUnits({
        prefix: 'AUTH',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result).toBeDefined();

      // And the output should contain "AUTH-001"
      expect(result.workUnits.some(wu => wu.id === 'AUTH-001')).toBe(true);

      // And the output should contain "AUTH-002"
      expect(result.workUnits.some(wu => wu.id === 'AUTH-002')).toBe(true);

      // And the output should not contain "DASH-001"
      expect(result.workUnits.some(wu => wu.id === 'DASH-001')).toBe(false);
    });
  });

  describe('Scenario: List work units filtered by epic', () => {
    it('should filter work units by epic', async () => {
      // Given I have a project with spec directory
      // And an epic "epic-user-management" exists
      // And work units exist
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work 1',
            status: 'backlog',
            epic: 'epic-user-management',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Work 2',
            status: 'backlog',
            epic: 'epic-user-management',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'SEC-001': {
            id: 'SEC-001',
            title: 'Work 3',
            status: 'backlog',
            epic: 'epic-security',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001', 'AUTH-002', 'SEC-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec list-work-units --epic=epic-user-management"
      const result = await listWorkUnits({
        epic: 'epic-user-management',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result).toBeDefined();

      // And the output should contain "AUTH-001"
      expect(result.workUnits.some(wu => wu.id === 'AUTH-001')).toBe(true);

      // And the output should contain "AUTH-002"
      expect(result.workUnits.some(wu => wu.id === 'AUTH-002')).toBe(true);

      // And the output should not contain "SEC-001"
      expect(result.workUnits.some(wu => wu.id === 'SEC-001')).toBe(false);
    });
  });

  describe('Scenario: Delete work unit with no dependencies', () => {
    it('should delete work unit without dependencies', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      // And the work unit has no children
      // And the work unit is not blocking other work
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Some work',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec delete-work-unit AUTH-001"
      // For testing, we'll skip the confirmation prompt
      const result = await deleteWorkUnit({
        workUnitId: 'AUTH-001',
        skipConfirmation: true,
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the work unit "AUTH-001" should not exist in spec/work-units.json
      const updatedContent = await readFile(join(testDir, 'spec/work-units.json'), 'utf-8');
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-001']).toBeUndefined();

      // And the work unit should be removed from states index
      expect(updatedData.states.backlog).not.toContain('AUTH-001');
    });
  });

  describe('Scenario: Force delete work unit without confirmation', () => {
    it('should force delete with --force flag', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Some work',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec delete-work-unit AUTH-001 --force"
      const result = await deleteWorkUnit({
        workUnitId: 'AUTH-001',
        force: true,
        cwd: testDir,
      });

      // Then the command should succeed without prompting
      expect(result.success).toBe(true);

      // And the work unit "AUTH-001" should not exist
      const updatedContent = await readFile(join(testDir, 'spec/work-units.json'), 'utf-8');
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-001']).toBeUndefined();
    });
  });

  describe('Scenario: Attempt to delete work unit with children', () => {
    it('should fail when work unit has children', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      // And a work unit "AUTH-002" exists with parent "AUTH-001"
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Parent work',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: ['AUTH-002'],
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Child work',
            status: 'backlog',
            parent: 'AUTH-001',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001', 'AUTH-002'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec delete-work-unit AUTH-001 --force"
      // Then the command should fail
      await expect(
        deleteWorkUnit({
          workUnitId: 'AUTH-001',
          force: true,
          cwd: testDir,
        })
      ).rejects.toThrow('Cannot delete work unit with children');

      // And the error should list child "AUTH-002"
      try {
        await deleteWorkUnit({
          workUnitId: 'AUTH-001',
          force: true,
          cwd: testDir,
        });
      } catch (error: unknown) {
        if (error instanceof Error) {
          expect(error.message).toContain('AUTH-002');
          // And the error should suggest "Delete children first or remove parent relationship"
          expect(error.message).toContain('Delete children first');
        }
      }
    });
  });

  describe('Scenario: Attempt to delete work unit that blocks other work', () => {
    it('should fail when work unit blocks others', async () => {
      // Given I have a project with spec directory
      // And a work unit "API-001" exists
      // And a work unit "AUTH-001" exists with blockedBy "API-001"
      const workUnits: WorkUnitsData = {
        workUnits: {
          'API-001': {
            id: 'API-001',
            title: 'API work',
            status: 'backlog',
            blocks: ['AUTH-001'], // API-001 blocks AUTH-001
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth work',
            status: 'blocked',
            blockedBy: ['API-001'], // AUTH-001 is blocked by API-001
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['API-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: ['AUTH-001'],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec delete-work-unit API-001 --force --cascade-dependencies"
      const result = await deleteWorkUnit({
        workUnitId: 'API-001',
        force: true,
        cascadeDependencies: true,
        cwd: testDir,
      });

      // Then the command should succeed with warning
      expect(result.success).toBe(true);
      expect(result.warnings).toBeDefined();
      expect(result.warnings?.some((w: string) => w.includes('blocks 1 work unit'))).toBe(true);

      // And if we try to delete without cascade flag, it should fail
      // First re-create the work unit for testing error case
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      try {
        await deleteWorkUnit({
          workUnitId: 'API-001',
          force: true,
          cwd: testDir,
        });
      } catch (error: unknown) {
        if (error instanceof Error) {
          // The error should mention dependencies and need for cascade flag
          expect(error.message).toContain('dependencies');
          expect(error.message).toContain('cascade');
        }
      }
    });
  });

  describe('Scenario: Create nested work unit hierarchy', () => {
    it('should create multi-level parent-child hierarchy', async () => {
      // Given I have a project with spec directory
      // And the prefix "AUTH" is registered
      const prefixes: PrefixesData = {
        prefixes: {
          AUTH: {
            prefix: 'AUTH',
            description: 'Authentication features',
            createdAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(join(testDir, 'spec/prefixes.json'), JSON.stringify(prefixes, null, 2));

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
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec create-work-unit AUTH 'OAuth 2.0 implementation'"
      await createWorkUnit({
        prefix: 'AUTH',
        title: 'OAuth 2.0 implementation',
        cwd: testDir,
      });

      // And I run "fspec create-work-unit AUTH 'Google provider' --parent=AUTH-001"
      await createWorkUnit({
        prefix: 'AUTH',
        title: 'Google provider',
        parent: 'AUTH-001',
        cwd: testDir,
      });

      // And I run "fspec create-work-unit AUTH 'GitHub provider' --parent=AUTH-001"
      await createWorkUnit({
        prefix: 'AUTH',
        title: 'GitHub provider',
        parent: 'AUTH-001',
        cwd: testDir,
      });

      // And I run "fspec create-work-unit AUTH 'Token storage' --parent=AUTH-001"
      await createWorkUnit({
        prefix: 'AUTH',
        title: 'Token storage',
        parent: 'AUTH-001',
        cwd: testDir,
      });

      // Then work unit "AUTH-001" should have 3 children
      const updatedContent = await readFile(join(testDir, 'spec/work-units.json'), 'utf-8');
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-001'].children?.length).toBe(3);

      // And the children should be "AUTH-002", "AUTH-003", "AUTH-004"
      expect(updatedData.workUnits['AUTH-001'].children).toContain('AUTH-002');
      expect(updatedData.workUnits['AUTH-001'].children).toContain('AUTH-003');
      expect(updatedData.workUnits['AUTH-001'].children).toContain('AUTH-004');

      // And each child should have parent "AUTH-001"
      expect(updatedData.workUnits['AUTH-002'].parent).toBe('AUTH-001');
      expect(updatedData.workUnits['AUTH-003'].parent).toBe('AUTH-001');
      expect(updatedData.workUnits['AUTH-004'].parent).toBe('AUTH-001');
    });
  });

  describe('Scenario: Attempt to create circular parent relationship', () => {
    it('should prevent circular parent relationships', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      // And a work unit "AUTH-002" exists with parent "AUTH-001"
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work 1',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: ['AUTH-002'],
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Work 2',
            status: 'backlog',
            parent: 'AUTH-001',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001', 'AUTH-002'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec update-work-unit AUTH-001 --parent=AUTH-002"
      // Then the command should fail
      await expect(
        updateWorkUnit({
          workUnitId: 'AUTH-001',
          parent: 'AUTH-002',
          cwd: testDir,
        })
      ).rejects.toThrow('Circular parent relationship detected');
    });
  });

  describe('Scenario: Attempt to exceed maximum nesting depth', () => {
    it('should enforce maximum nesting depth', async () => {
      // Given I have a project with spec directory
      // And work units exist with nesting
      const prefixes: PrefixesData = {
        prefixes: {
          AUTH: {
            prefix: 'AUTH',
            description: 'Authentication features',
            createdAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(join(testDir, 'spec/prefixes.json'), JSON.stringify(prefixes, null, 2));

      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Level 1',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: ['AUTH-002'],
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Level 2',
            status: 'backlog',
            parent: 'AUTH-001',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: ['AUTH-003'],
          },
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Level 3',
            status: 'backlog',
            parent: 'AUTH-002',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001', 'AUTH-002', 'AUTH-003'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec create-work-unit AUTH 'Too deep' --parent=AUTH-003"
      // Then the command should fail
      await expect(
        createWorkUnit({
          prefix: 'AUTH',
          title: 'Too deep',
          parent: 'AUTH-003',
          cwd: testDir,
        })
      ).rejects.toThrow('Maximum nesting depth (3) exceeded');
    });
  });

  describe('Scenario: Validate work unit data structure', () => {
    it('should validate work unit JSON schema', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Some work',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec validate-work-units"
      const result = await validateWorkUnits({
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.valid).toBe(true);

      // And the validation should check JSON schema compliance
      expect(result.checks).toContain('schema');

      // And the validation should check parent/child consistency
      expect(result.checks).toContain('parentChild');

      // And the validation should check work unit IDs are unique
      expect(result.checks).toContain('uniqueIds');
    });
  });

  describe('Scenario: Attempt to perform operations on non-existent work unit', () => {
    it('should fail when work unit does not exist', async () => {
      // Given I have a project with spec directory
      // And no work unit "AUTH-999" exists
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
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec show-work-unit AUTH-999"
      // Then the command should fail
      await expect(
        showWorkUnit({
          workUnitId: 'AUTH-999',
          cwd: testDir,
        })
      ).rejects.toThrow("Work unit 'AUTH-999' does not exist");
    });
  });
});
