import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData, PrefixesData, EpicsData } from '../../types';

// Import commands
import { createStory } from '../create-story';
import { updateWorkUnit } from '../update-work-unit';
import { showWorkUnit } from '../show-work-unit';
import { listWorkUnits } from '../list-work-units';
import { deleteWorkUnit } from '../delete-work-unit';
import { validateWorkUnits } from '../validate-work-units';
import { createMinimalFoundation } from '../../test-helpers/foundation-helper';

import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';
describe('Feature: Work Unit Management', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('work-unit-management');

    // Create foundation.json for all tests (required by create-work-unit)
    await createMinimalFoundation(testDir);
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
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
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec create-story AUTH 'Implement OAuth login'"
      const result = await createStory({
        prefix: 'AUTH',
        title: 'Implement OAuth login',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And a work unit "AUTH-001" should be created in spec/work-units.json
      const updatedContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-001']).toBeDefined();

      // And the work unit should have title "Implement OAuth login"
      expect(updatedData.workUnits['AUTH-001'].title).toBe(
        'Implement OAuth login'
      );

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
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec create-story AUTH 'Add password reset'"
      const result = await createStory({
        prefix: 'AUTH',
        title: 'Add password reset',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And a work unit "AUTH-002" should be created
      const updatedContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
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
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/epics.json'),
        JSON.stringify(epics, null, 2)
      );

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

      // When I run "fspec create-story AUTH 'OAuth integration' --epic=epic-user-management"
      const result = await createStory({
        prefix: 'AUTH',
        title: 'OAuth integration',
        epic: 'epic-user-management',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the work unit "AUTH-001" should have epic "epic-user-management"
      const updatedContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-001'].epic).toBe(
        'epic-user-management'
      );

      // And the epic should reference work unit "AUTH-001"
      const epicsContent = await readFile(
        join(testDir, 'spec/epics.json'),
        'utf-8'
      );
      const epicsData: EpicsData = JSON.parse(epicsContent);

      expect(epicsData.epics['epic-user-management'].workUnits).toContain(
        'AUTH-001'
      );
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
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

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

      // When I run "fspec create-story AUTH 'OAuth login' --description='Add OAuth 2.0 with Google and GitHub'"
      const result = await createStory({
        prefix: 'AUTH',
        title: 'OAuth login',
        description: 'Add OAuth 2.0 with Google and GitHub',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the work unit should have description "Add OAuth 2.0 with Google and GitHub"
      const updatedContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-001'].description).toBe(
        'Add OAuth 2.0 with Google and GitHub'
      );
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
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec create-story AUTH 'Google provider' --parent=AUTH-001"
      const result = await createStory({
        prefix: 'AUTH',
        title: 'Google provider',
        parent: 'AUTH-001',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the work unit "AUTH-002" should be created
      const updatedContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
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
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

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

      // When I run "fspec create-story INVALID 'Some work'"
      // Then the command should fail
      await expect(
        createStory({
          prefix: 'INVALID',
          title: 'Some work',
          cwd: testDir,
        })
      ).rejects.toThrow("Prefix 'INVALID' is not registered");

      // And the error should suggest "Run 'fspec create-prefix INVALID' first"
      try {
        await createStory({
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
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

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

      // When I run "fspec create-story AUTH"
      // Then the command should fail
      await expect(
        createStory({
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
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec create-story AUTH 'Child work' --parent=AUTH-999"
      // Then the command should fail
      await expect(
        createStory({
          prefix: 'AUTH',
          title: 'Child work',
          parent: 'AUTH-999',
          cwd: testDir,
        })
      ).rejects.toThrow("Parent story 'AUTH-999' does not exist");
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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec update-work-unit AUTH-001 --title='New title'"
      const result = await updateWorkUnit({
        workUnitId: 'AUTH-001',
        title: 'New title',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the work unit "AUTH-001" should have title "New title"
      const updatedContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-001'].title).toBe('New title');

      // And the updatedAt timestamp should be updated
      expect(updatedData.workUnits['AUTH-001'].updatedAt).not.toBe(
        '2025-01-01T00:00:00.000Z'
      );
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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec update-work-unit AUTH-001 --description='Updated description'"
      const result = await updateWorkUnit({
        workUnitId: 'AUTH-001',
        description: 'Updated description',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the work unit should have description "Updated description"
      const updatedContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-001'].description).toBe(
        'Updated description'
      );
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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/epics.json'),
        JSON.stringify(epics, null, 2)
      );

      // When I run "fspec update-work-unit AUTH-001 --epic=epic-security"
      const result = await updateWorkUnit({
        workUnitId: 'AUTH-001',
        epic: 'epic-security',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the work unit should have epic "epic-security"
      const updatedContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const updatedData: WorkUnitsData = JSON.parse(updatedContent);

      expect(updatedData.workUnits['AUTH-001'].epic).toBe('epic-security');

      // And the epic should reference work unit "AUTH-001"
      const epicsContent = await readFile(
        join(testDir, 'spec/epics.json'),
        'utf-8'
      );
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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // And no epic "epic-nonexistent" exists
      const epics: EpicsData = {
        epics: {},
      };
      await writeFile(
        join(testDir, 'spec/epics.json'),
        JSON.stringify(epics, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

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
      const updatedContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec delete-work-unit AUTH-001 --force"
      const result = await deleteWorkUnit({
        workUnitId: 'AUTH-001',
        force: true,
        cwd: testDir,
      });

      // Then the command should succeed without prompting
      expect(result.success).toBe(true);

      // And the work unit "AUTH-001" should not exist
      const updatedContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

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
      expect(
        result.warnings?.some((w: string) => w.includes('blocks 1 work unit'))
      ).toBe(true);

      // And if we try to delete without cascade flag, it should fail
      // First re-create the work unit for testing error case
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

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

      // When I run "fspec create-story AUTH 'OAuth 2.0 implementation'"
      await createStory({
        prefix: 'AUTH',
        title: 'OAuth 2.0 implementation',
        cwd: testDir,
      });

      // And I run "fspec create-story AUTH 'Google provider' --parent=AUTH-001"
      await createStory({
        prefix: 'AUTH',
        title: 'Google provider',
        parent: 'AUTH-001',
        cwd: testDir,
      });

      // And I run "fspec create-story AUTH 'GitHub provider' --parent=AUTH-001"
      await createStory({
        prefix: 'AUTH',
        title: 'GitHub provider',
        parent: 'AUTH-001',
        cwd: testDir,
      });

      // And I run "fspec create-story AUTH 'Token storage' --parent=AUTH-001"
      await createStory({
        prefix: 'AUTH',
        title: 'Token storage',
        parent: 'AUTH-001',
        cwd: testDir,
      });

      // Then work unit "AUTH-001" should have 3 children
      const updatedContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // When I run "fspec create-story AUTH 'Too deep' --parent=AUTH-003"
      // Then the command should fail
      await expect(
        createStory({
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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

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

  describe('Scenario: Auto-create work-units.json when missing', () => {
    it('should automatically create work-units.json with initial structure', async () => {
      // Given I have a project with spec directory
      // And the file "spec/work-units.json" does not exist
      // NOTE: We don't create work-units.json

      // And the prefix "HOOK" is registered in spec/prefixes.json
      const prefixes: PrefixesData = {
        prefixes: {
          HOOK: {
            prefix: 'HOOK',
            description: 'Hook features',
            createdAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

      // When I run "fspec create-story HOOK 'Hook Handler'"
      const result = await createStory({
        prefix: 'HOOK',
        title: 'Hook Handler',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And the file "spec/work-units.json" should be created with initial structure
      const content = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const data: WorkUnitsData = JSON.parse(content);

      // And the structure should include meta section with version and lastUpdated
      expect(data.meta).toBeDefined();
      expect(data.meta?.version).toBeDefined();
      expect(data.meta?.lastUpdated).toBeDefined();

      // And the structure should include states
      expect(data.states).toBeDefined();
      expect(data.states.backlog).toBeDefined();
      expect(data.states.specifying).toBeDefined();
      expect(data.states.testing).toBeDefined();
      expect(data.states.implementing).toBeDefined();
      expect(data.states.validating).toBeDefined();
      expect(data.states.done).toBeDefined();
      expect(data.states.blocked).toBeDefined();

      // And the structure should include workUnits object
      expect(data.workUnits).toBeDefined();

      // And the work unit "HOOK-001" should be created successfully
      expect(data.workUnits['HOOK-001']).toBeDefined();

      // And "HOOK-001" should be in the backlog state array
      expect(data.states.backlog).toContain('HOOK-001');
    });
  });

  describe('Scenario: Auto-create prefixes.json when reading work units', () => {
    it('should automatically create prefixes.json when listing work units', async () => {
      // Given I have a project with spec directory
      // And the file "spec/prefixes.json" does not exist
      // NOTE: We don't create prefixes.json

      // And the file "spec/work-units.json" exists with work unit data
      const workUnits: WorkUnitsData = {
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            title: 'Test work',
            status: 'backlog',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: ['TEST-001'],
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

      // When I run "fspec list-work-units"
      const result = await listWorkUnits({
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result).toBeDefined();

      // And the file "spec/prefixes.json" should be created with empty structure
      const content = await readFile(
        join(testDir, 'spec/prefixes.json'),
        'utf-8'
      );
      const data: PrefixesData = JSON.parse(content);
      expect(data.prefixes).toBeDefined();

      // And the command should list all work units
      expect(result.workUnits.some(wu => wu.id === 'TEST-001')).toBe(true);
    });
  });

  describe('Scenario: Auto-create epics.json when needed for work unit operations', () => {
    it('should automatically create epics.json but still fail if epic does not exist', async () => {
      // Given I have a project with spec directory
      // And the file "spec/epics.json" does not exist
      // NOTE: We don't create epics.json

      // And the prefix "AUTH" is registered
      const prefixes: PrefixesData = {
        prefixes: {
          AUTH: {
            prefix: 'AUTH',
            description: 'Auth features',
            createdAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

      // And spec/work-units.json exists
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

      // When I run "fspec create-story AUTH 'Login feature' --epic=epic-auth"
      // Then the command should fail
      await expect(
        createStory({
          prefix: 'AUTH',
          title: 'Login feature',
          epic: 'epic-auth',
          cwd: testDir,
        })
      ).rejects.toThrow("Epic 'epic-auth' does not exist");

      // And the file "spec/epics.json" should be created with empty structure
      const content = await readFile(join(testDir, 'spec/epics.json'), 'utf-8');
      const data: EpicsData = JSON.parse(content);
      expect(data.epics).toBeDefined();
    });
  });

  // Work Unit Linking Tests
  describe('Scenario: Show work unit with linked feature files and scenarios', () => {
    it('should display linked feature files and scenarios with line numbers', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec/features'), { recursive: true });

      // And a work unit "AUTH-001" exists with title "OAuth Login Implementation"
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth Login Implementation',
            status: 'implementing',
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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // And a feature file "oauth-login.feature" is tagged with "@AUTH-001"
      const featureContent = `@AUTH-001
@critical
@authentication
Feature: OAuth Login

  Scenario: Login with Google
    Given I am on login page
    When I click Google
    Then I am logged in

  Scenario: Login with GitHub
    Given I am on login page
    When I click GitHub
    Then I am logged in

  Scenario: Handle OAuth errors
    Given invalid credentials
    When I attempt login
    Then I see error`;

      await writeFile(
        join(testDir, 'spec/features/oauth-login.feature'),
        featureContent
      );

      // When I run "fspec show-work-unit AUTH-001"
      const result = await showWorkUnit({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result).toBeDefined();

      // And the output should show "Linked Features & Scenarios:"
      expect(result.linkedFeatures).toBeDefined();

      // And the output should show "oauth-login.feature:6 - Login with Google"
      const scenarios =
        result.linkedFeatures?.flatMap((f: any) => f.scenarios) || [];
      expect(
        scenarios.some(
          (s: any) => s.name === 'Login with Google' && s.line === 6
        )
      ).toBe(true);

      // And the output should show "oauth-login.feature:11 - Login with GitHub"
      expect(
        scenarios.some(
          (s: any) => s.name === 'Login with GitHub' && s.line === 11
        )
      ).toBe(true);

      // And the output should show "oauth-login.feature:16 - Handle OAuth errors"
      expect(
        scenarios.some(
          (s: any) => s.name === 'Handle OAuth errors' && s.line === 16
        )
      ).toBe(true);

      // And the output should show "Total: 3 scenarios"
      expect(scenarios.length).toBe(3);
    });
  });

  describe('Scenario: Show work unit with scenario-level tags', () => {
    it('should only show scenarios with matching work unit tag', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec/features'), { recursive: true });

      // And work units exist
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth Login',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Token Refresh',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-002'],
          testing: [],
          implementing: ['AUTH-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // And a feature file has feature-level tag "@AUTH-001" and scenario with "@AUTH-002"
      const featureContent = `@AUTH-001
@critical
Feature: OAuth

  Scenario: Login
    Given I login
    When I authenticate
    Then I am logged in

  @AUTH-002
  Scenario: Refresh tokens
    Given I have a token
    When it expires
    Then refresh it`;

      await writeFile(
        join(testDir, 'spec/features/oauth.feature'),
        featureContent
      );

      // When I run "fspec show-work-unit AUTH-001"
      const result1 = await showWorkUnit({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then the output should show scenario "Login" under AUTH-001
      const scenarios1 =
        result1.linkedFeatures?.flatMap((f: any) => f.scenarios) || [];
      expect(scenarios1.some((s: any) => s.name === 'Login')).toBe(true);

      // And the output should not show scenario "Refresh tokens"
      expect(scenarios1.some((s: any) => s.name === 'Refresh tokens')).toBe(
        false
      );

      // When I run "fspec show-work-unit AUTH-002"
      const result2 = await showWorkUnit({
        workUnitId: 'AUTH-002',
        cwd: testDir,
      });

      // Then the output should show only scenario "Refresh tokens"
      const scenarios2 =
        result2.linkedFeatures?.flatMap((f: any) => f.scenarios) || [];
      expect(scenarios2.some((s: any) => s.name === 'Refresh tokens')).toBe(
        true
      );
      expect(scenarios2.some((s: any) => s.name === 'Login')).toBe(false);
    });
  });

  describe('Scenario: Show work unit with no linked features', () => {
    it('should show "Linked Features & Scenarios: None"', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec/features'), { recursive: true });

      // And a work unit "AUTH-001" exists
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth Login',
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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // And no feature files are tagged with "@AUTH-001"
      // (don't create any feature files)

      // When I run "fspec show-work-unit AUTH-001"
      const result = await showWorkUnit({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result).toBeDefined();

      // And the output should show "Linked Features & Scenarios: None"
      expect(result.linkedFeatures).toBeDefined();
      expect(result.linkedFeatures?.length).toBe(0);
    });
  });

  describe('Scenario: Show work unit with JSON output including linked features', () => {
    it('should include linkedFeatures in JSON output', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec/features'), { recursive: true });

      // And a work unit "AUTH-001" exists
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth Work',
            status: 'testing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: ['AUTH-001'],
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

      // And a feature file "oauth.feature" is tagged with "@AUTH-001" and has 2 scenarios
      const featureContent = `@AUTH-001
@critical
Feature: OAuth

  Scenario: First scenario
    Given a step
    When another step
    Then result

  Scenario: Second scenario
    Given a step
    When another step
    Then result`;

      await writeFile(
        join(testDir, 'spec/features/oauth.feature'),
        featureContent
      );

      // When I run "fspec show-work-unit AUTH-001 --output=json"
      const result = await showWorkUnit({
        workUnitId: 'AUTH-001',
        output: 'json',
        cwd: testDir,
      });

      // Then the JSON should have "linkedFeatures" array
      expect(result.linkedFeatures).toBeDefined();
      expect(Array.isArray(result.linkedFeatures)).toBe(true);

      // And the linkedFeatures array should contain "oauth.feature"
      expect(
        result.linkedFeatures?.some((f: any) =>
          f.file.includes('oauth.feature')
        )
      ).toBe(true);

      // And the JSON should have "linkedScenarios" array with 2 items
      const scenarios =
        result.linkedFeatures?.flatMap((f: any) => f.scenarios) || [];
      expect(scenarios.length).toBe(2);

      // And each scenario should have "file", "line", and "name" fields
      expect(scenarios[0].file).toBeDefined();
      expect(scenarios[0].line).toBeDefined();
      expect(scenarios[0].name).toBeDefined();
    });
  });

  describe('Scenario: Show work unit with multiple feature files', () => {
    it('should show all linked feature files grouped', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec/features'), { recursive: true });

      // And a work unit "AUTH-001" exists
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth System',
            status: 'implementing',
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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // And feature file "oauth-login.feature" is tagged with "@AUTH-001"
      const feature1 = `@AUTH-001
Feature: OAuth Login
  Scenario: Login with Google
    Given I login
    When I use Google
    Then I am authenticated`;
      await writeFile(
        join(testDir, 'spec/features/oauth-login.feature'),
        feature1
      );

      // And feature file "oauth-refresh.feature" is tagged with "@AUTH-001"
      const feature2 = `@AUTH-001
Feature: OAuth Refresh
  Scenario: Refresh expired tokens
    Given I have expired token
    When I refresh
    Then I get new token`;
      await writeFile(
        join(testDir, 'spec/features/oauth-refresh.feature'),
        feature2
      );

      // When I run "fspec show-work-unit AUTH-001"
      const result = await showWorkUnit({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then the output should show both feature files
      expect(result.linkedFeatures?.length).toBe(2);

      // And scenarios should be grouped by feature file
      const loginFeature = result.linkedFeatures?.find((f: any) =>
        f.file.includes('oauth-login.feature')
      );
      const refreshFeature = result.linkedFeatures?.find((f: any) =>
        f.file.includes('oauth-refresh.feature')
      );

      expect(loginFeature).toBeDefined();
      expect(refreshFeature).toBeDefined();

      expect(
        loginFeature?.scenarios.some((s: any) => s.name === 'Login with Google')
      ).toBe(true);
      expect(
        refreshFeature?.scenarios.some(
          (s: any) => s.name === 'Refresh expired tokens'
        )
      ).toBe(true);
    });
  });

  describe('Scenario: Complete end-to-end work unit to feature linking workflow', () => {
    it('should create work unit, link to feature, and validate bidirectional linking', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec/features'), { recursive: true });

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
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

      // Initialize work-units.json
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

      // Initialize tags.json with required tags
      const tagsJson = {
        $schema: '../src/schemas/tags.schema.json',
        categories: [
          {
            name: 'Phase Tags',
            description: 'Development phase tags',
            required: false,
            tags: [{ name: '@critical', description: 'Phase 1' }],
          },
          {
            name: 'Component Tags',
            description: 'Architectural components',
            required: false,
            tags: [{ name: '@authentication', description: 'Authentication' }],
          },
          {
            name: 'Feature Group Tags',
            description: 'Functional areas',
            required: false,
            tags: [
              {
                name: '@feature-management',
                description: 'Feature Management',
              },
            ],
          },
        ],
        combinationExamples: [],
        usageGuidelines: {
          minimumTagsPerFeature: 1,
          recommendedTagsPerFeature: 3,
          tagNamingConvention: 'kebab-case with @ prefix',
        },
      };
      await writeFile(
        join(testDir, 'spec/tags.json'),
        JSON.stringify(tagsJson, null, 2)
      );

      // And a feature file "oauth-login.feature" exists without work unit tags
      const featureContent = `@critical
@authentication
@feature-management
Feature: OAuth Login

  Scenario: Login with Google
    Given I am on login page
    When I click Google
    Then I am logged in

  Scenario: Login with GitHub
    Given I am on login page
    When I click GitHub
    Then I am logged in`;

      await writeFile(
        join(testDir, 'spec/features/oauth-login.feature'),
        featureContent
      );

      // When I run "fspec create-story AUTH 'OAuth Login Implementation'"
      const createResult = await createStory({
        prefix: 'AUTH',
        title: 'OAuth Login Implementation',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(createResult.success).toBe(true);

      // And a work unit "AUTH-001" should be created with title "OAuth Login Implementation"
      const workUnitsContent = await readFile(
        join(testDir, 'spec/work-units.json'),
        'utf-8'
      );
      const workUnitsData: WorkUnitsData = JSON.parse(workUnitsContent);
      expect(workUnitsData.workUnits['AUTH-001']).toBeDefined();
      expect(workUnitsData.workUnits['AUTH-001'].title).toBe(
        'OAuth Login Implementation'
      );

      // When I run "fspec add-tag-to-feature oauth-login.feature @AUTH-001"
      // Import the add-tag-to-feature command
      const { addTagToFeature } = await import('../add-tag-to-feature');

      await addTagToFeature(
        'spec/features/oauth-login.feature',
        ['@AUTH-001'],
        { cwd: testDir }
      );

      // Then the command should succeed
      // And the feature file should contain tag "@AUTH-001"
      const updatedFeatureContent = await readFile(
        join(testDir, 'spec/features/oauth-login.feature'),
        'utf-8'
      );
      expect(updatedFeatureContent).toContain('@AUTH-001');

      // When I run "fspec show-work-unit AUTH-001"
      const showWorkUnitResult = await showWorkUnit({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(showWorkUnitResult).toBeDefined();

      // And the output should show linked feature "oauth-login.feature"
      expect(showWorkUnitResult.linkedFeatures).toBeDefined();
      expect(
        showWorkUnitResult.linkedFeatures?.some((f: any) =>
          f.file.includes('oauth-login.feature')
        )
      ).toBe(true);

      // And the output should list all scenarios from the feature
      const scenarios =
        showWorkUnitResult.linkedFeatures?.flatMap((f: any) => f.scenarios) ||
        [];
      expect(scenarios.some((s: any) => s.name === 'Login with Google')).toBe(
        true
      );
      expect(scenarios.some((s: any) => s.name === 'Login with GitHub')).toBe(
        true
      );

      // When I run "fspec show-feature oauth-login.feature"
      const { showFeature } = await import('../show-feature');
      const showFeatureResult = await showFeature({
        feature: 'spec/features/oauth-login.feature',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(showFeatureResult).toBeDefined();

      // And the output should show work unit "AUTH-001" linked to this feature
      expect(showFeatureResult.workUnits).toBeDefined();
      expect(
        showFeatureResult.workUnits?.some((wu: any) => wu.id === 'AUTH-001')
      ).toBe(true);

      // When I run "fspec validate-tags oauth-login.feature"
      const { validateTags } = await import('../validate-tags');
      const validateResult = await validateTags({
        file: 'spec/features/oauth-login.feature',
        cwd: testDir,
      });

      // Then the command should succeed
      // And work unit tag "@AUTH-001" should be validated against spec/work-units.json
      expect(validateResult.validCount).toBe(1);
      expect(validateResult.invalidCount).toBe(0);
    });
  });

  describe('Scenario: Remove work unit tag from feature and verify unlinking', () => {
    it('should remove work unit tag and verify feature is no longer linked', async () => {
      // Given I have a project with spec directory
      await mkdir(join(testDir, 'spec/features'), { recursive: true });

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
      await writeFile(
        join(testDir, 'spec/prefixes.json'),
        JSON.stringify(prefixes, null, 2)
      );

      // And a work unit "AUTH-001" exists with title "OAuth Login Implementation"
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth Login Implementation',
            status: 'implementing',
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
      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnits, null, 2)
      );

      // And a feature file "oauth-login.feature" is tagged with "@AUTH-001"
      const featureContent = `@AUTH-001
@critical
@authentication
Feature: OAuth Login

  Scenario: Login with Google
    Given I am on login page
    When I click Google
    Then I am logged in`;

      await writeFile(
        join(testDir, 'spec/features/oauth-login.feature'),
        featureContent
      );

      // When I run "fspec show-work-unit AUTH-001"
      const showResult1 = await showWorkUnit({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then the command should succeed
      // And the output should show linked feature "oauth-login.feature"
      expect(showResult1.linkedFeatures).toBeDefined();
      expect(
        showResult1.linkedFeatures?.some((f: any) =>
          f.file.includes('oauth-login.feature')
        )
      ).toBe(true);

      // When I run "fspec remove-tag-from-feature oauth-login.feature @AUTH-001"
      const { removeTagFromFeature } = await import(
        '../remove-tag-from-feature'
      );
      const removeResult = await removeTagFromFeature(
        'spec/features/oauth-login.feature',
        ['@AUTH-001'],
        { cwd: testDir }
      );

      // Then the command should succeed
      expect(removeResult.success).toBe(true);

      // And the feature file should not contain tag "@AUTH-001"
      const updatedFeatureContent = await readFile(
        join(testDir, 'spec/features/oauth-login.feature'),
        'utf-8'
      );
      expect(updatedFeatureContent).not.toContain('@AUTH-001');

      // When I run "fspec show-work-unit AUTH-001"
      const showResult2 = await showWorkUnit({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // Then the command should succeed
      // And the output should show "Linked Features & Scenarios: None"
      expect(showResult2.linkedFeatures).toBeDefined();
      expect(showResult2.linkedFeatures?.length).toBe(0);

      // When I run "fspec show-feature oauth-login.feature"
      const { showFeature } = await import('../show-feature');
      const showFeatureResult = await showFeature({
        feature: 'spec/features/oauth-login.feature',
        cwd: testDir,
      });

      // Then the command should succeed
      // And the output should not show work unit "AUTH-001"
      expect(showFeatureResult.workUnits).toBeDefined();
      expect(
        showFeatureResult.workUnits?.some((wu: any) => wu.id === 'AUTH-001')
      ).toBe(false);
    });
  });
});
