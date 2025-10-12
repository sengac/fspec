import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, readFile, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { createWorkUnit } from '../work-unit';
import { updateWorkUnit } from '../work-unit';
import { deleteWorkUnit } from '../work-unit';
import { showWorkUnit } from '../work-unit';
import { listWorkUnits } from '../work-unit';
import { validateWorkUnits } from '../work-unit';

describe('Feature: Work Unit Management', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;
  let prefixesFile: string;
  let epicsFile: string;

  beforeEach(async () => {
    // Create temporary directory for each test
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');
    prefixesFile = join(specDir, 'prefixes.json');
    epicsFile = join(specDir, 'epics.json');

    // Create spec directory
    await mkdir(specDir, { recursive: true });

    // Initialize empty work units file
    await writeFile(
      workUnitsFile,
      JSON.stringify(
        {
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
        },
        null,
        2
      )
    );

    // Initialize prefixes file
    await writeFile(
      prefixesFile,
      JSON.stringify(
        {
          prefixes: {},
        },
        null,
        2
      )
    );

    // Initialize epics file
    await writeFile(
      epicsFile,
      JSON.stringify(
        {
          epics: {},
        },
        null,
        2
      )
    );
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Create work unit with auto-incrementing ID', () => {
    it('should create work unit AUTH-001 with correct defaults', async () => {
      // Given I have a project with spec directory
      // And the prefix "AUTH" is registered in spec/prefixes.json
      const prefixes = JSON.parse(await readFile(prefixesFile, 'utf-8'));
      prefixes.prefixes.AUTH = { description: 'Authentication features' };
      await writeFile(prefixesFile, JSON.stringify(prefixes, null, 2));

      // And no work units exist with prefix "AUTH"
      // (already initialized empty in beforeEach)

      // When I run "fspec create-work-unit AUTH 'Implement OAuth login'"
      await createWorkUnit('AUTH', 'Implement OAuth login', { cwd: testDir });

      // Then the command should succeed
      // And a work unit "AUTH-001" should be created in spec/work-units.json
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(workUnits.workUnits['AUTH-001']).toBeDefined();

      // And the work unit should have title "Implement OAuth login"
      expect(workUnits.workUnits['AUTH-001'].title).toBe(
        'Implement OAuth login'
      );

      // And the work unit should have status "backlog"
      expect(workUnits.workUnits['AUTH-001'].status).toBe('backlog');

      // And the work unit should have createdAt timestamp
      expect(workUnits.workUnits['AUTH-001'].createdAt).toBeDefined();
      expect(
        new Date(workUnits.workUnits['AUTH-001'].createdAt).getTime()
      ).toBeGreaterThan(0);

      // And the work unit should have updatedAt timestamp
      expect(workUnits.workUnits['AUTH-001'].updatedAt).toBeDefined();

      // And the states.backlog array should contain "AUTH-001"
      expect(workUnits.states.backlog).toContain('AUTH-001');
    });
  });

  describe('Scenario: Create second work unit with incremented ID', () => {
    it('should create AUTH-002 after AUTH-001 exists', async () => {
      // Given I have a project with spec directory
      // And the prefix "AUTH" is registered
      const prefixes = JSON.parse(await readFile(prefixesFile, 'utf-8'));
      prefixes.prefixes.AUTH = { description: 'Authentication features' };
      await writeFile(prefixesFile, JSON.stringify(prefixes, null, 2));

      // And a work unit "AUTH-001" exists
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'First work unit',
        status: 'backlog',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec create-work-unit AUTH 'Add password reset'"
      await createWorkUnit('AUTH', 'Add password reset', { cwd: testDir });

      // Then the command should succeed
      // And a work unit "AUTH-002" should be created
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-002']).toBeDefined();

      // And the work unit should have status "backlog"
      expect(updatedWorkUnits.workUnits['AUTH-002'].status).toBe('backlog');

      // And the states.backlog array should contain "AUTH-002"
      expect(updatedWorkUnits.states.backlog).toContain('AUTH-002');
    });
  });

  describe('Scenario: Create work unit with epic assignment', () => {
    it('should create work unit with epic reference', async () => {
      // Given I have a project with spec directory
      // And the prefix "AUTH" is registered
      const prefixes = JSON.parse(await readFile(prefixesFile, 'utf-8'));
      prefixes.prefixes.AUTH = { description: 'Authentication features' };
      await writeFile(prefixesFile, JSON.stringify(prefixes, null, 2));

      // And an epic "epic-user-management" exists
      const epics = JSON.parse(await readFile(epicsFile, 'utf-8'));
      epics.epics['epic-user-management'] = {
        id: 'epic-user-management',
        title: 'User Management',
        workUnits: [],
      };
      await writeFile(epicsFile, JSON.stringify(epics, null, 2));

      // When I run "fspec create-work-unit AUTH 'OAuth integration' --epic=epic-user-management"
      await createWorkUnit('AUTH', 'OAuth integration', {
        cwd: testDir,
        epic: 'epic-user-management',
      });

      // Then the command should succeed
      // And the work unit "AUTH-001" should have epic "epic-user-management"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(workUnits.workUnits['AUTH-001'].epic).toBe('epic-user-management');

      // And the epic should reference work unit "AUTH-001"
      const updatedEpics = JSON.parse(await readFile(epicsFile, 'utf-8'));
      expect(updatedEpics.epics['epic-user-management'].workUnits).toContain(
        'AUTH-001'
      );
    });
  });

  describe('Scenario: Create work unit with description', () => {
    it('should create work unit with description field', async () => {
      // Given I have a project with spec directory
      // And the prefix "AUTH" is registered
      const prefixes = JSON.parse(await readFile(prefixesFile, 'utf-8'));
      prefixes.prefixes.AUTH = { description: 'Authentication features' };
      await writeFile(prefixesFile, JSON.stringify(prefixes, null, 2));

      // When I run "fspec create-work-unit AUTH 'OAuth login' --description='Add OAuth 2.0 with Google and GitHub'"
      await createWorkUnit('AUTH', 'OAuth login', {
        cwd: testDir,
        description: 'Add OAuth 2.0 with Google and GitHub',
      });

      // Then the command should succeed
      // And the work unit should have description "Add OAuth 2.0 with Google and GitHub"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(workUnits.workUnits['AUTH-001'].description).toBe(
        'Add OAuth 2.0 with Google and GitHub'
      );
    });
  });

  describe('Scenario: Create child work unit with parent', () => {
    it('should create child work unit and update parent', async () => {
      // Given I have a project with spec directory
      // And the prefix "AUTH" is registered
      const prefixes = JSON.parse(await readFile(prefixesFile, 'utf-8'));
      prefixes.prefixes.AUTH = { description: 'Authentication features' };
      await writeFile(prefixesFile, JSON.stringify(prefixes, null, 2));

      // And a work unit "AUTH-001" exists with title "OAuth integration"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth integration',
        status: 'backlog',
        children: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec create-work-unit AUTH 'Google provider' --parent=AUTH-001"
      await createWorkUnit('AUTH', 'Google provider', {
        cwd: testDir,
        parent: 'AUTH-001',
      });

      // Then the command should succeed
      // And the work unit "AUTH-002" should be created
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-002']).toBeDefined();

      // And the work unit "AUTH-002" should have parent "AUTH-001"
      expect(updatedWorkUnits.workUnits['AUTH-002'].parent).toBe('AUTH-001');

      // And the work unit "AUTH-001" children array should contain "AUTH-002"
      expect(updatedWorkUnits.workUnits['AUTH-001'].children).toContain(
        'AUTH-002'
      );
    });
  });

  describe('Scenario: Attempt to create work unit with unregistered prefix', () => {
    it('should fail with helpful error message', async () => {
      // Given I have a project with spec directory
      // And the prefix "INVALID" is not registered
      // (prefixes.json initialized empty in beforeEach)

      // When I run "fspec create-work-unit INVALID 'Some work'"
      // Then the command should fail
      await expect(
        createWorkUnit('INVALID', 'Some work', { cwd: testDir })
      ).rejects.toThrow("Prefix 'INVALID' is not registered");

      // And the error should suggest "Run 'fspec create-prefix INVALID' first"
      await expect(
        createWorkUnit('INVALID', 'Some work', { cwd: testDir })
      ).rejects.toThrow("Run 'fspec create-prefix INVALID' first");
    });
  });

  describe('Scenario: Attempt to create work unit with missing title', () => {
    it('should fail with validation error', async () => {
      // Given I have a project with spec directory
      // And the prefix "AUTH" is registered
      const prefixes = JSON.parse(await readFile(prefixesFile, 'utf-8'));
      prefixes.prefixes.AUTH = { description: 'Authentication features' };
      await writeFile(prefixesFile, JSON.stringify(prefixes, null, 2));

      // When I run "fspec create-work-unit AUTH"
      // Then the command should fail
      // And the error should contain "Title is required"
      await expect(
        createWorkUnit('AUTH', '', { cwd: testDir })
      ).rejects.toThrow('Title is required');
    });
  });

  describe('Scenario: Attempt to create child with non-existent parent', () => {
    it('should fail with parent validation error', async () => {
      // Given I have a project with spec directory
      // And the prefix "AUTH" is registered
      const prefixes = JSON.parse(await readFile(prefixesFile, 'utf-8'));
      prefixes.prefixes.AUTH = { description: 'Authentication features' };
      await writeFile(prefixesFile, JSON.stringify(prefixes, null, 2));

      // And no work unit "AUTH-999" exists
      // (work-units.json initialized empty in beforeEach)

      // When I run "fspec create-work-unit AUTH 'Child work' --parent=AUTH-999"
      // Then the command should fail
      // And the error should contain "Parent work unit 'AUTH-999' does not exist"
      await expect(
        createWorkUnit('AUTH', 'Child work', {
          cwd: testDir,
          parent: 'AUTH-999',
        })
      ).rejects.toThrow("Parent work unit 'AUTH-999' does not exist");
    });
  });

  describe('Scenario: Update work unit title', () => {
    it('should update title and timestamp', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with title "Old title"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      const oldTimestamp = new Date('2024-01-01T00:00:00Z').toISOString();
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'Old title',
        status: 'backlog',
        createdAt: oldTimestamp,
        updatedAt: oldTimestamp,
      };
      workUnits.states.backlog.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec update-work-unit AUTH-001 --title='New title'"
      await updateWorkUnit(
        'AUTH-001',
        { title: 'New title' },
        { cwd: testDir }
      );

      // Then the command should succeed
      // And the work unit "AUTH-001" should have title "New title"
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].title).toBe('New title');

      // And the updatedAt timestamp should be updated
      expect(updatedWorkUnits.workUnits['AUTH-001'].updatedAt).not.toBe(
        oldTimestamp
      );
      expect(
        new Date(updatedWorkUnits.workUnits['AUTH-001'].updatedAt).getTime()
      ).toBeGreaterThan(new Date(oldTimestamp).getTime());
    });
  });

  describe('Scenario: Update work unit description', () => {
    it('should update description field', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'backlog',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec update-work-unit AUTH-001 --description='Updated description'"
      await updateWorkUnit(
        'AUTH-001',
        { description: 'Updated description' },
        { cwd: testDir }
      );

      // Then the command should succeed
      // And the work unit should have description "Updated description"
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].description).toBe(
        'Updated description'
      );
    });
  });

  describe('Scenario: Update work unit epic', () => {
    it('should update epic reference and bidirectional link', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'backlog',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // And an epic "epic-security" exists
      const epics = JSON.parse(await readFile(epicsFile, 'utf-8'));
      epics.epics['epic-security'] = {
        id: 'epic-security',
        title: 'Security Improvements',
        workUnits: [],
      };
      await writeFile(epicsFile, JSON.stringify(epics, null, 2));

      // When I run "fspec update-work-unit AUTH-001 --epic=epic-security"
      await updateWorkUnit(
        'AUTH-001',
        { epic: 'epic-security' },
        { cwd: testDir }
      );

      // Then the command should succeed
      // And the work unit should have epic "epic-security"
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].epic).toBe('epic-security');

      // And the epic should reference work unit "AUTH-001"
      const updatedEpics = JSON.parse(await readFile(epicsFile, 'utf-8'));
      expect(updatedEpics.epics['epic-security'].workUnits).toContain(
        'AUTH-001'
      );
    });
  });

  describe('Scenario: Attempt to update work unit with invalid epic', () => {
    it('should fail with epic validation error', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'backlog',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // And no epic "epic-nonexistent" exists
      // (epics.json initialized empty in beforeEach)

      // When I run "fspec update-work-unit AUTH-001 --epic=epic-nonexistent"
      // Then the command should fail
      // And the error should contain "Epic 'epic-nonexistent' does not exist"
      await expect(
        updateWorkUnit(
          'AUTH-001',
          { epic: 'epic-nonexistent' },
          { cwd: testDir }
        )
      ).rejects.toThrow("Epic 'epic-nonexistent' does not exist");
    });
  });

  describe('Scenario: Show single work unit details', () => {
    it('should display work unit with all fields', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with detailed data
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth integration',
        description: 'Add OAuth 2.0 support',
        status: 'implementing',
        estimate: 5,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.implementing.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec show-work-unit AUTH-001"
      const output = await showWorkUnit('AUTH-001', { cwd: testDir });

      // Then the command should succeed
      // And the output should display work unit details
      expect(output).toContain('AUTH-001');
      expect(output).toContain('OAuth integration');
      expect(output).toContain('implementing');
    });
  });

  describe('Scenario: Show work unit with JSON output', () => {
    it('should output valid JSON with all fields', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'backlog',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec show-work-unit AUTH-001 --output=json"
      const output = await showWorkUnit('AUTH-001', {
        cwd: testDir,
        output: 'json',
      });

      // Then the command should succeed
      // And the output should be valid JSON
      const json = JSON.parse(output);
      expect(json.id).toBe('AUTH-001');
      expect(json.title).toBe('OAuth login');
      expect(json.status).toBe('backlog');
    });
  });

  describe('Scenario: List all work units', () => {
    it('should display all work units', async () => {
      // Given I have a project with spec directory
      // And work units exist
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'done',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        title: 'Password reset',
        status: 'implementing',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['DASH-001'] = {
        id: 'DASH-001',
        title: 'User dashboard',
        status: 'backlog',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.done.push('AUTH-001');
      workUnits.states.implementing.push('AUTH-002');
      workUnits.states.backlog.push('DASH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec list-work-units"
      const output = await listWorkUnits({ cwd: testDir });

      // Then the command should succeed
      // And the output should contain all work units
      expect(output).toContain('AUTH-001');
      expect(output).toContain('AUTH-002');
      expect(output).toContain('DASH-001');
    });
  });

  describe('Scenario: List work units filtered by status', () => {
    it('should only display work units with matching status', async () => {
      // Given I have a project with spec directory
      // And work units exist with different statuses
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        status: 'backlog',
        title: 'Auth work',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        status: 'implementing',
        title: 'More auth',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['DASH-001'] = {
        id: 'DASH-001',
        status: 'backlog',
        title: 'Dashboard',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001', 'DASH-001');
      workUnits.states.implementing.push('AUTH-002');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec list-work-units --status=backlog"
      const output = await listWorkUnits({ cwd: testDir, status: 'backlog' });

      // Then the command should succeed
      // And the output should contain work units in backlog
      expect(output).toContain('AUTH-001');
      expect(output).toContain('DASH-001');

      // And the output should not contain implementing work units
      expect(output).not.toContain('AUTH-002');
    });
  });

  describe('Scenario: List work units filtered by prefix', () => {
    it('should only display work units with matching prefix', async () => {
      // Given I have a project with spec directory
      // And work units exist with different prefixes
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'Auth work',
        status: 'backlog',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        title: 'More auth',
        status: 'backlog',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['DASH-001'] = {
        id: 'DASH-001',
        title: 'Dashboard',
        status: 'backlog',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001', 'AUTH-002', 'DASH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec list-work-units --prefix=AUTH"
      const output = await listWorkUnits({ cwd: testDir, prefix: 'AUTH' });

      // Then the command should succeed
      // And the output should contain AUTH work units
      expect(output).toContain('AUTH-001');
      expect(output).toContain('AUTH-002');

      // And the output should not contain DASH work units
      expect(output).not.toContain('DASH-001');
    });
  });

  describe('Scenario: List work units filtered by epic', () => {
    it('should only display work units with matching epic', async () => {
      // Given I have a project with spec directory
      // And an epic exists with work units
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'Auth 1',
        epic: 'epic-user-management',
        status: 'backlog',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        title: 'Auth 2',
        epic: 'epic-user-management',
        status: 'backlog',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['SEC-001'] = {
        id: 'SEC-001',
        title: 'Security',
        epic: 'epic-security',
        status: 'backlog',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001', 'AUTH-002', 'SEC-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec list-work-units --epic=epic-user-management"
      const output = await listWorkUnits({
        cwd: testDir,
        epic: 'epic-user-management',
      });

      // Then the command should succeed
      // And the output should contain epic-user-management work units
      expect(output).toContain('AUTH-001');
      expect(output).toContain('AUTH-002');

      // And the output should not contain epic-security work units
      expect(output).not.toContain('SEC-001');
    });
  });

  describe('Scenario: Delete work unit with no dependencies', () => {
    it('should delete work unit after confirmation', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with no dependencies
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'backlog',
        children: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec delete-work-unit AUTH-001" and confirm
      await deleteWorkUnit('AUTH-001', { cwd: testDir, force: true });

      // Then the command should succeed
      // And the work unit "AUTH-001" should not exist
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001']).toBeUndefined();

      // And the work unit should be removed from states index
      expect(updatedWorkUnits.states.backlog).not.toContain('AUTH-001');
    });
  });

  describe('Scenario: Force delete work unit without confirmation', () => {
    it('should delete without prompting', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'backlog',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec delete-work-unit AUTH-001 --force"
      await deleteWorkUnit('AUTH-001', { cwd: testDir, force: true });

      // Then the command should succeed without prompting
      // And the work unit "AUTH-001" should not exist
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001']).toBeUndefined();
    });
  });

  describe('Scenario: Attempt to delete work unit with children', () => {
    it('should fail with helpful error message', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" has child "AUTH-002"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'Parent',
        status: 'backlog',
        children: ['AUTH-002'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        title: 'Child',
        status: 'backlog',
        parent: 'AUTH-001',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001', 'AUTH-002');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec delete-work-unit AUTH-001 --force"
      // Then the command should fail
      await expect(
        deleteWorkUnit('AUTH-001', { cwd: testDir, force: true })
      ).rejects.toThrow('Cannot delete work unit with children');

      // And the error should list child "AUTH-002"
      await expect(
        deleteWorkUnit('AUTH-001', { cwd: testDir, force: true })
      ).rejects.toThrow('AUTH-002');

      // And the error should suggest removing children
      await expect(
        deleteWorkUnit('AUTH-001', { cwd: testDir, force: true })
      ).rejects.toThrow('Delete children first or remove parent relationship');
    });
  });

  describe('Scenario: Attempt to delete work unit that blocks other work', () => {
    it('should fail with blocking relationship error', async () => {
      // Given I have a project with spec directory
      // And work unit "AUTH-001" is blockedBy "API-001"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        title: 'API work',
        status: 'backlog',
        relationships: {
          blocks: ['AUTH-001'],
        },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'Auth work',
        status: 'blocked',
        relationships: {
          blockedBy: ['API-001'],
        },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('API-001');
      workUnits.states.blocked.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec delete-work-unit API-001 --force"
      // Then the command should fail
      await expect(
        deleteWorkUnit('API-001', { cwd: testDir, force: true })
      ).rejects.toThrow('Cannot delete work unit that blocks other work');

      // And the error should list blocked work unit
      await expect(
        deleteWorkUnit('API-001', { cwd: testDir, force: true })
      ).rejects.toThrow('AUTH-001');

      // And the error should suggest removing relationships
      await expect(
        deleteWorkUnit('API-001', { cwd: testDir, force: true })
      ).rejects.toThrow('Remove blocking relationships first');
    });
  });

  describe('Scenario: Create nested work unit hierarchy', () => {
    it('should create parent with multiple children', async () => {
      // Given I have a project with spec directory
      // And the prefix "AUTH" is registered
      const prefixes = JSON.parse(await readFile(prefixesFile, 'utf-8'));
      prefixes.prefixes.AUTH = { description: 'Authentication features' };
      await writeFile(prefixesFile, JSON.stringify(prefixes, null, 2));

      // When I create parent and children
      await createWorkUnit('AUTH', 'OAuth 2.0 implementation', {
        cwd: testDir,
      });
      await createWorkUnit('AUTH', 'Google provider', {
        cwd: testDir,
        parent: 'AUTH-001',
      });
      await createWorkUnit('AUTH', 'GitHub provider', {
        cwd: testDir,
        parent: 'AUTH-001',
      });
      await createWorkUnit('AUTH', 'Token storage', {
        cwd: testDir,
        parent: 'AUTH-001',
      });

      // Then work unit "AUTH-001" should have 3 children
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      expect(workUnits.workUnits['AUTH-001'].children).toHaveLength(3);
      expect(workUnits.workUnits['AUTH-001'].children).toEqual([
        'AUTH-002',
        'AUTH-003',
        'AUTH-004',
      ]);

      // And each child should have parent "AUTH-001"
      expect(workUnits.workUnits['AUTH-002'].parent).toBe('AUTH-001');
      expect(workUnits.workUnits['AUTH-003'].parent).toBe('AUTH-001');
      expect(workUnits.workUnits['AUTH-004'].parent).toBe('AUTH-001');
    });
  });

  describe('Scenario: Attempt to create circular parent relationship', () => {
    it('should detect and prevent circular relationships', async () => {
      // Given I have a project with spec directory
      // And work unit "AUTH-002" has parent "AUTH-001"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'Parent',
        status: 'backlog',
        children: ['AUTH-002'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        title: 'Child',
        status: 'backlog',
        parent: 'AUTH-001',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001', 'AUTH-002');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec update-work-unit AUTH-001 --parent=AUTH-002"
      // Then the command should fail with circular relationship error
      await expect(
        updateWorkUnit('AUTH-001', { parent: 'AUTH-002' }, { cwd: testDir })
      ).rejects.toThrow('Circular parent relationship detected');
    });
  });

  describe('Scenario: Attempt to exceed maximum nesting depth', () => {
    it('should enforce maximum depth of 3 levels', async () => {
      // Given I have a project with spec directory
      // And work units exist with 3 levels of nesting
      const prefixes = JSON.parse(await readFile(prefixesFile, 'utf-8'));
      prefixes.prefixes.AUTH = { description: 'Authentication features' };
      await writeFile(prefixesFile, JSON.stringify(prefixes, null, 2));

      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'Level 1',
        status: 'backlog',
        children: ['AUTH-002'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        title: 'Level 2',
        status: 'backlog',
        parent: 'AUTH-001',
        children: ['AUTH-003'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-003'] = {
        id: 'AUTH-003',
        title: 'Level 3',
        status: 'backlog',
        parent: 'AUTH-002',
        children: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001', 'AUTH-002', 'AUTH-003');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec create-work-unit AUTH 'Too deep' --parent=AUTH-003"
      // Then the command should fail
      await expect(
        createWorkUnit('AUTH', 'Too deep', { cwd: testDir, parent: 'AUTH-003' })
      ).rejects.toThrow('Maximum nesting depth (3) exceeded');
    });
  });

  describe('Scenario: Validate work unit data structure', () => {
    it('should validate JSON schema and consistency', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'backlog',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.backlog.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec validate-work-units"
      const result = await validateWorkUnits({ cwd: testDir });

      // Then the command should succeed
      expect(result.valid).toBe(true);

      // And validation checks should pass
      expect(result.checks).toContain('JSON schema compliance');
      expect(result.checks).toContain('parent/child consistency');
      expect(result.checks).toContain('unique work unit IDs');
    });
  });

  describe('Scenario: Attempt to perform operations on non-existent work unit', () => {
    it('should fail with not found error', async () => {
      // Given I have a project with spec directory
      // And no work unit "AUTH-999" exists

      // When I run "fspec show-work-unit AUTH-999"
      // Then the command should fail
      await expect(showWorkUnit('AUTH-999', { cwd: testDir })).rejects.toThrow(
        "Work unit 'AUTH-999' does not exist"
      );
    });
  });
});
