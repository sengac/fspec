/**
 * Tests for virtual hooks system reminders in update-work-unit-status
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { updateWorkUnitStatus } from '../update-work-unit-status';

describe('Virtual hooks system reminders', () => {
  let testDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-virtual-hooks-reminders-'));
    workUnitsFile = join(testDir, 'spec', 'work-units.json');
    await mkdir(join(testDir, 'spec'), { recursive: true });

    // Create initial work units file
    const workUnitsData = {
      states: {
        backlog: [],
        specifying: ['AUTH-001'],
        testing: [],
        implementing: ['BUG-001'],
        validating: [],
        done: [],
        blocked: [],
      },
      workUnits: {
        'AUTH-001': {
          id: 'AUTH-001',
          title: 'User Login',
          type: 'story',
          status: 'specifying',
          createdAt: '2025-01-01T00:00:00.000Z',
          updatedAt: '2025-01-01T00:00:00.000Z',
          stateHistory: [
            {
              state: 'backlog',
              timestamp: '2025-01-01T00:00:00.000Z',
            },
            {
              state: 'specifying',
              timestamp: '2025-01-01T01:00:00.000Z',
            },
          ],
          rules: ['User must authenticate with valid credentials'],
          examples: [
            'User successfully logs in with valid username and password',
          ],
          architectureNotes: [
            'Implementation: Use secure authentication with password hashing',
          ],
          attachments: ['spec/attachments/AUTH-001/ast-research.json'],
        },
        'BUG-001': {
          id: 'BUG-001',
          title: 'Fix login redirect',
          type: 'bug',
          status: 'implementing',
          createdAt: '2025-01-01T00:00:00.000Z',
          updatedAt: '2025-01-01T00:00:00.000Z',
          stateHistory: [
            {
              state: 'backlog',
              timestamp: '2025-01-01T00:00:00.000Z',
            },
            {
              state: 'implementing',
              timestamp: '2025-01-01T02:00:00.000Z',
            },
          ],
          virtualHooks: [
            {
              name: 'lint',
              event: 'pre-implementing',
              command: 'npm run lint',
              blocking: true,
            },
            {
              name: 'test',
              event: 'post-implementing',
              command: 'npm test',
              blocking: false,
            },
          ],
        },
      },
    };

    await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

    // Create a feature file for AUTH-001
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
    await writeFile(
      join(testDir, 'spec', 'features', 'user-login.feature'),
      '@AUTH-001\nFeature: User Login\n\nScenario: Login success\n  Given valid credentials\n  When user logs in\n  Then redirect to dashboard'
    );

    // Create coverage file for user-login.feature
    const coverageContent = {
      scenarios: [
        {
          name: 'Login success',
          testMappings: [
            {
              file: 'src/__tests__/user-login.test.ts',
              lines: '1-10',
              implMappings: [
                {
                  file: 'src/auth/login.ts',
                  lines: '1-20',
                },
              ],
            },
          ],
        },
      ],
    };
    await writeFile(
      join(testDir, 'spec', 'features', 'user-login.feature.coverage'),
      JSON.stringify(coverageContent, null, 2)
    );

    // Create the test file referenced in coverage
    await mkdir(join(testDir, 'src', '__tests__'), { recursive: true });
    const testFileContent = `// @step Given  valid credentials
// @step When  user logs in
// @step Then  redirect to dashboard
describe('Login success', () => {
  it('should redirect', () => {
    expect(true).toBe(true);
  });
});
`;
    await writeFile(
      join(testDir, 'src', '__tests__', 'user-login.test.ts'),
      testFileContent
    );

    // Create the implementation file referenced in coverage
    await mkdir(join(testDir, 'src', 'auth'), { recursive: true });
    const implFileContent = `export function login(username: string, password: string) {
  // Authentication logic
  return true;
}
`;
    await writeFile(join(testDir, 'src', 'auth', 'login.ts'), implFileContent);
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Virtual hooks reminder when transitioning from specifying to testing', () => {
    it('should emit virtual hooks reminder', async () => {
      // When I transition AUTH-001 from specifying → testing
      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'testing',
        cwd: testDir,
      });

      // Then the result should include system reminder
      expect(result.systemReminder).toBeDefined();

      // And the reminder should mention virtual hooks
      expect(result.systemReminder).toContain('VIRTUAL HOOKS');
      expect(result.systemReminder).toContain('Consider quality checks');

      // And the reminder should list available events
      expect(result.systemReminder).toContain('pre-testing');
      expect(result.systemReminder).toContain('post-testing');
      expect(result.systemReminder).toContain('pre-implementing');
      expect(result.systemReminder).toContain('post-implementing');

      // And the reminder should include command examples
      expect(result.systemReminder).toContain(
        'fspec add-virtual-hook AUTH-001'
      );
      expect(result.systemReminder).toContain('fspec list-virtual-hooks');
      expect(result.systemReminder).toContain('fspec remove-virtual-hook');
      expect(result.systemReminder).toContain('fspec clear-virtual-hooks');

      // And the reminder should be wrapped in system-reminder tags
      expect(result.systemReminder).toContain('<system-reminder>');
      expect(result.systemReminder).toContain('</system-reminder>');
    });

    it('should combine virtual hooks reminder with status change reminder', async () => {
      // When I transition AUTH-001 from specifying → testing
      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'testing',
        cwd: testDir,
      });

      // Then both reminders should be present
      expect(result.systemReminder).toContain('TESTING status');
      expect(result.systemReminder).toContain('VIRTUAL HOOKS');

      // And they should be in a consolidated reminder block (as of BUG-077 consolidation)
      const reminderCount = (
        result.systemReminder?.match(/<system-reminder>/g) || []
      ).length;
      expect(reminderCount).toBeGreaterThanOrEqual(1);
    });
  });

  describe('Scenario: Cleanup reminder when transitioning to done with virtual hooks', () => {
    it('should emit cleanup reminder when work unit has virtual hooks', async () => {
      // First move through validating
      await updateWorkUnitStatus({
        workUnitId: 'BUG-001',
        status: 'validating',
        cwd: testDir,
      });

      // When I transition BUG-001 to done (it has 2 virtual hooks)
      const result = await updateWorkUnitStatus({
        workUnitId: 'BUG-001',
        status: 'done',
        cwd: testDir,
      });

      // Then the result should include cleanup reminder
      expect(result.systemReminder).toBeDefined();

      // And the reminder should mention virtual hooks count
      expect(result.systemReminder).toContain('2 virtual hooks');

      // And the reminder should ask about cleanup
      expect(result.systemReminder).toContain('CLEANUP DECISION');
      expect(result.systemReminder).toContain('keep or remove');

      // And the reminder should provide options
      expect(result.systemReminder).toContain('KEEP hooks');
      expect(result.systemReminder).toContain('REMOVE hooks');

      // And the reminder should include cleanup command
      expect(result.systemReminder).toContain(
        'fspec clear-virtual-hooks BUG-001'
      );

      // And the reminder should ask user
      expect(result.systemReminder).toContain('ASK USER');
    });

    it('should not emit cleanup reminder when work unit has no virtual hooks', async () => {
      // When I transition AUTH-001 to done (it has no virtual hooks)
      // First need to move through the workflow
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'testing',
        cwd: testDir,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'implementing',
        skipTemporalValidation: true,
        cwd: testDir,
      });
      await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'validating',
        skipTemporalValidation: true,
        cwd: testDir,
      });

      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'done',
        skipTemporalValidation: true,
        cwd: testDir,
      });

      // Then the cleanup reminder should not mention virtual hooks cleanup
      if (result.systemReminder) {
        expect(result.systemReminder).not.toContain('CLEANUP DECISION');
        expect(result.systemReminder).not.toContain('virtual hooks');
      }
    });
  });

  describe('Scenario: No virtual hooks reminder for non-transitioning statuses', () => {
    it('should not emit virtual hooks reminder when not transitioning from specifying → testing', async () => {
      // When I transition BUG-001 from implementing → validating
      const result = await updateWorkUnitStatus({
        workUnitId: 'BUG-001',
        status: 'validating',
        cwd: testDir,
      });

      // Then the virtual hooks reminder should not be present
      expect(result.systemReminder).toBeDefined(); // Status reminder exists
      expect(result.systemReminder).toContain('VALIDATING status'); // Status reminder
      expect(result.systemReminder).not.toContain('VIRTUAL HOOKS'); // No virtual hooks reminder
    });
  });
});
