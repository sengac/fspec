import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../../types';

// Import commands (to be created)
import { addRule } from '../add-rule';
import { addExample } from '../add-example';
import { addQuestion } from '../add-question';
import { addAssumption } from '../add-assumption';
import { removeRule } from '../remove-rule';
import { removeExample } from '../remove-example';
import { removeQuestion } from '../remove-question';
import { answerQuestion } from '../answer-question';
import { generateScenarios } from '../generate-scenarios';
import { importExampleMap } from '../import-example-map';
import { exportExampleMap } from '../export-example-map';
import { queryExampleMappingStats } from '../query-example-mapping-stats';

describe('Feature: Example Mapping Integration', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-example-mapping-integration');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
    await mkdir(join(testDir, 'spec/features'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Add rule to work unit during discovery', () => {
    it('should add rule to example mapping', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "specifying"
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // When I run "fspec add-rule AUTH-001 'Users must authenticate before accessing protected resources'"
      const result = await addRule({
        workUnitId: 'AUTH-001',
        rule: 'Users must authenticate before accessing protected resources',
        cwd: testDir,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      // And the work unit should have 1 rule
      expect(updated.workUnits['AUTH-001'].rules).toHaveLength(1);

      // And the rule should be "Users must authenticate before accessing protected resources"
      expect(updated.workUnits['AUTH-001'].rules[0]).toBe('Users must authenticate before accessing protected resources');
    });
  });

  describe('Scenario: Add multiple rules to work unit', () => {
    it('should add multiple rules', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      await addRule({ workUnitId: 'AUTH-001', rule: 'OAuth tokens expire after 1 hour', cwd: testDir });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Refresh tokens valid for 30 days', cwd: testDir });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Only one active session per user', cwd: testDir });

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      // Then the work unit should have 3 rules
      expect(updated.workUnits['AUTH-001'].rules).toHaveLength(3);

      // And the rules should be in order
      expect(updated.workUnits['AUTH-001'].rules[0]).toBe('OAuth tokens expire after 1 hour');
      expect(updated.workUnits['AUTH-001'].rules[1]).toBe('Refresh tokens valid for 30 days');
      expect(updated.workUnits['AUTH-001'].rules[2]).toBe('Only one active session per user');
    });
  });

  describe('Scenario: Add example that will become a scenario', () => {
    it('should add example for scenario generation', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await addExample({
        workUnitId: 'AUTH-001',
        example: 'User logs in with Google account',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      expect(updated.workUnits['AUTH-001'].examples).toHaveLength(1);
      expect(updated.workUnits['AUTH-001'].examples[0]).toBe('User logs in with Google account');
    });
  });

  describe('Scenario: Add multiple examples for scenario candidates', () => {
    it('should add multiple examples', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      await addExample({ workUnitId: 'AUTH-001', example: 'User logs in with valid credentials', cwd: testDir });
      await addExample({ workUnitId: 'AUTH-001', example: 'User logs in with expired token', cwd: testDir });
      await addExample({ workUnitId: 'AUTH-001', example: 'User token auto-refreshes before expiry', cwd: testDir });
      await addExample({ workUnitId: 'AUTH-001', example: 'User logs out and token is invalidated', cwd: testDir });

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      expect(updated.workUnits['AUTH-001'].examples).toHaveLength(4);
    });
  });

  describe('Scenario: Add question that needs human answer', () => {
    it('should add unanswered question', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await addQuestion({
        workUnitId: 'AUTH-001',
        question: 'Should we support GitHub Enterprise?',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      expect(updated.workUnits['AUTH-001'].questions).toHaveLength(1);
      expect(updated.workUnits['AUTH-001'].questions[0]).toBe('Should we support GitHub Enterprise?');
    });
  });

  describe('Scenario: Add question mentioning specific person', () => {
    it('should track person in question', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await addQuestion({
        workUnitId: 'AUTH-001',
        question: '@bob: What is the token expiry policy?',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      expect(updated.workUnits['AUTH-001'].questions[0]).toContain('@bob:');
      expect(updated.workUnits['AUTH-001'].questions[0]).toBe('@bob: What is the token expiry policy?');
    });
  });

  describe('Scenario: Add assumption about requirements', () => {
    it('should record assumption', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await addAssumption({
        workUnitId: 'AUTH-001',
        assumption: 'Users have valid OAuth accounts',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      expect(updated.workUnits['AUTH-001'].assumptions).toHaveLength(1);
      expect(updated.workUnits['AUTH-001'].assumptions[0]).toBe('Users have valid OAuth accounts');
    });
  });

  describe('Scenario: Complete example mapping session with all four artifacts', () => {
    it('should have rules, examples, questions, and assumptions', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      await addRule({ workUnitId: 'AUTH-001', rule: 'OAuth tokens expire after 1 hour', cwd: testDir });
      await addRule({ workUnitId: 'AUTH-001', rule: 'Users must authenticate before accessing protected resources', cwd: testDir });
      await addExample({ workUnitId: 'AUTH-001', example: 'User logs in with Google', cwd: testDir });
      await addExample({ workUnitId: 'AUTH-001', example: 'User logs in with expired token', cwd: testDir });
      await addQuestion({ workUnitId: 'AUTH-001', question: '@security-team: Do we need PKCE flow?', cwd: testDir });
      await addAssumption({ workUnitId: 'AUTH-001', assumption: 'Users have valid email addresses', cwd: testDir });

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      expect(updated.workUnits['AUTH-001'].rules).toHaveLength(2);
      expect(updated.workUnits['AUTH-001'].examples).toHaveLength(2);
      expect(updated.workUnits['AUTH-001'].questions).toHaveLength(1);
      expect(updated.workUnits['AUTH-001'].assumptions).toHaveLength(1);
    });
  });

  describe('Scenario: Show work unit with example mapping data', () => {
    it('should display example mapping artifacts', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            rules: ['OAuth tokens expire after 1 hour', 'Users must authenticate first'],
            examples: ['User logs in with Google', 'User logs in with expired token'],
            questions: ['@bob: Support GitHub Enterprise?'],
            assumptions: ['Users have valid OAuth accounts'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const { showWorkUnit } = await import('../show-work-unit');
      const result = await showWorkUnit({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      expect(result.rules).toBeDefined();
      expect(result.examples).toBeDefined();
      expect(result.questions).toBeDefined();
      expect(result.assumptions).toBeDefined();
      expect(result.rules).toHaveLength(2);
      expect(result.examples).toHaveLength(2);
    });
  });

  describe('Scenario: Remove rule by index', () => {
    it('should remove rule at specific index', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            rules: [
              'OAuth tokens expire after 1 hour',
              'Users must authenticate first',
              'Refresh tokens valid for 30 days',
            ],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await removeRule({
        workUnitId: 'AUTH-001',
        index: 1,
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      expect(updated.workUnits['AUTH-001'].rules).toHaveLength(2);
      expect(updated.workUnits['AUTH-001'].rules[1]).toBe('Refresh tokens valid for 30 days');
    });
  });

  describe('Scenario: Remove example by index', () => {
    it('should remove example at specific index', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            examples: [
              'User logs in with Google',
              'User logs in with expired token',
              'User logs out',
            ],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await removeExample({
        workUnitId: 'AUTH-001',
        index: 0,
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      expect(updated.workUnits['AUTH-001'].examples).toHaveLength(2);
      expect(updated.workUnits['AUTH-001'].examples[0]).toBe('User logs in with expired token');
    });
  });

  describe('Scenario: Remove question by index', () => {
    it('should remove question at specific index', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            questions: [
              '@bob: Support GitHub Enterprise?',
              'What is the token expiry policy?',
            ],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await removeQuestion({
        workUnitId: 'AUTH-001',
        index: 0,
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      expect(updated.workUnits['AUTH-001'].questions).toHaveLength(1);
    });
  });

  describe('Scenario: Attempt to remove item with invalid index', () => {
    it('should fail when index out of range', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            rules: ['Rule 1', 'Rule 2'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const error = await removeRule({
        workUnitId: 'AUTH-001',
        index: 5,
        cwd: testDir,
      }).catch((e: Error) => e);

      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain('Index 5 out of range');
        expect(error.message).toContain('Valid indices: 0-1');
      }
    });
  });

  describe('Scenario: Answer question and add to assumptions', () => {
    it('should move answered question to assumptions', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            questions: ['@bob: Should we support GitHub Enterprise?'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await answerQuestion({
        workUnitId: 'AUTH-001',
        index: 0,
        answer: 'No, only GitHub.com supported',
        addTo: 'assumptions',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      expect(updated.workUnits['AUTH-001'].questions || []).toHaveLength(0);
      expect(updated.workUnits['AUTH-001'].assumptions).toHaveLength(1);
      expect(updated.workUnits['AUTH-001'].assumptions[0]).toContain('GitHub.com supported');
    });
  });

  describe('Scenario: Answer question and add to rules', () => {
    it('should move answered question to rules', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            questions: ['What is the maximum session length?'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await answerQuestion({
        workUnitId: 'AUTH-001',
        index: 0,
        answer: '24 hours',
        addTo: 'rules',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      expect(updated.workUnits['AUTH-001'].questions || []).toHaveLength(0);
      expect(updated.workUnits['AUTH-001'].rules).toHaveLength(1);
      expect(updated.workUnits['AUTH-001'].rules[0]).toContain('24 hours');
    });
  });

  describe('Scenario: Answer question without adding to rules or assumptions', () => {
    it('should remove question without adding anywhere', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            questions: ['Is this feature needed?'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await answerQuestion({
        workUnitId: 'AUTH-001',
        index: 0,
        answer: 'No, descoping',
        addTo: 'none',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      expect(updated.workUnits['AUTH-001'].questions || []).toHaveLength(0);
      expect(updated.workUnits['AUTH-001'].rules || []).toHaveLength(0);
      expect(updated.workUnits['AUTH-001'].assumptions || []).toHaveLength(0);
    });
  });

  describe('Scenario: Generate Gherkin scenarios from examples', () => {
    it('should convert examples to Gherkin scenarios', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            examples: [
              'User logs in with Google account',
              'User logs in with expired token',
              'User token auto-refreshes',
            ],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await generateScenarios({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      expect(result.success).toBe(true);
      expect(result.scenariosGenerated).toBe(3);

      // Check feature file was created
      const featureFiles = await readFile(join(testDir, 'spec/features/auth-001.feature'), 'utf-8');
      expect(featureFiles).toContain('@AUTH-001');
      expect(featureFiles).toContain('User logs in with Google account');
      expect(featureFiles).toContain('User logs in with expired token');
      expect(featureFiles).toContain('User token auto-refreshes');

      // Check examples are preserved
      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));
      expect(updated.workUnits['AUTH-001'].examples).toHaveLength(3);
    });
  });

  describe('Scenario: Generate scenarios with Given/When/Then template', () => {
    it('should use proper Gherkin structure', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            examples: ['User logs in with valid credentials'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await generateScenarios({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const featureContent = await readFile(join(testDir, 'spec/features/auth-001.feature'), 'utf-8');
      expect(featureContent).toContain('@AUTH-001');
      expect(featureContent).toContain('Scenario: User logs in with valid credentials');
      expect(featureContent).toContain('Given');
      expect(featureContent).toContain('When');
      expect(featureContent).toContain('Then');
    });
  });

  describe('Scenario: Generate scenarios into existing feature file', () => {
    it('should append scenarios to existing feature', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            examples: ['User logs in with Google'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // Create existing feature file
      const existingFeature = `Feature: Authentication

Scenario: Existing scenario
  Given something
  When something happens
  Then result occurs
`;
      await writeFile(join(testDir, 'spec/features/authentication.feature'), existingFeature);

      const result = await generateScenarios({
        workUnitId: 'AUTH-001',
        feature: 'authentication',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const featureContent = await readFile(join(testDir, 'spec/features/authentication.feature'), 'utf-8');
      expect(featureContent).toContain('Existing scenario');
      expect(featureContent).toContain('@AUTH-001');
      expect(featureContent).toContain('User logs in with Google');
    });
  });

  describe('Scenario: Generate scenarios into new feature file', () => {
    it('should create new feature file with scenarios', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            examples: ['User logs in with OAuth'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await generateScenarios({
        workUnitId: 'AUTH-001',
        feature: 'oauth-login',
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const featureContent = await readFile(join(testDir, 'spec/features/oauth-login.feature'), 'utf-8');
      expect(featureContent).toContain('Feature:');
      expect(featureContent).toContain('@AUTH-001');
      expect(featureContent).toContain('User logs in with OAuth');
    });
  });

  describe('Scenario: Prevent moving to testing when questions remain unanswered', () => {
    it('should block testing state when questions unanswered', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            questions: ['@bob: Should we support OAuth 2.0?'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // Create feature file so scenario check passes
      const featureContent = '@AUTH-001\nFeature: Auth\nScenario: Login\n  Given x';
      await writeFile(join(testDir, 'spec/features/auth.feature'), featureContent);

      const { updateWorkUnitStatus } = await import('../update-work-unit-status');
      const error = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'testing',
        cwd: testDir,
      }).catch((e: Error) => e);

      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain('Unanswered questions prevent state transition');
        expect(error.message).toContain('@bob: Should we support OAuth 2.0?');
        expect(error.message).toContain("Answer questions with 'fspec answer-question AUTH-001");
      }
    });
  });

  describe('Scenario: Warn when moving to testing with no examples', () => {
    it('should warn when no examples defined', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // Create feature file
      const featureContent = '@AUTH-001\nFeature: Auth\nScenario: Login\n  Given x';
      await writeFile(join(testDir, 'spec/features/auth.feature'), featureContent);

      const { updateWorkUnitStatus } = await import('../update-work-unit-status');
      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'testing',
        cwd: testDir,
      });

      expect(result.success).toBe(true);
      expect(result.warnings).toBeDefined();
      expect(result.warnings?.some(w => w.includes('No examples captured'))).toBe(true);
    });
  });

  describe('Scenario: Bulk add multiple items from JSON', () => {
    it('should import example mapping from JSON', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const exampleMapping = {
        rules: [
          'OAuth tokens expire after 1 hour',
          'Users must authenticate first',
        ],
        examples: [
          'User logs in with Google',
          'User logs in with expired token',
        ],
        questions: [
          '@bob: Support GitHub Enterprise?',
        ],
        assumptions: [
          'Users have valid OAuth accounts',
        ],
      };
      await writeFile(join(testDir, 'example-mapping.json'), JSON.stringify(exampleMapping, null, 2));

      const result = await importExampleMap({
        workUnitId: 'AUTH-001',
        file: join(testDir, 'example-mapping.json'),
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const updated = JSON.parse(await readFile(join(testDir, 'spec/work-units.json'), 'utf-8'));

      expect(updated.workUnits['AUTH-001'].rules).toHaveLength(2);
      expect(updated.workUnits['AUTH-001'].examples).toHaveLength(2);
      expect(updated.workUnits['AUTH-001'].questions).toHaveLength(1);
      expect(updated.workUnits['AUTH-001'].assumptions).toHaveLength(1);
    });
  });

  describe('Scenario: Export example mapping to JSON', () => {
    it('should export example mapping as JSON', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            rules: ['OAuth tokens expire after 1 hour'],
            examples: ['User logs in with Google'],
            questions: ['@bob: Support GitHub Enterprise?'],
            assumptions: ['Users have valid OAuth accounts'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await exportExampleMap({
        workUnitId: 'AUTH-001',
        output: join(testDir, 'auth-example-map.json'),
        cwd: testDir,
      });

      expect(result.success).toBe(true);

      const exported = JSON.parse(await readFile(join(testDir, 'auth-example-map.json'), 'utf-8'));

      expect(exported.rules).toBeDefined();
      expect(exported.examples).toBeDefined();
      expect(exported.questions).toBeDefined();
      expect(exported.assumptions).toBeDefined();
    });
  });

  describe('Scenario: Find all work units with unanswered questions', () => {
    it('should query work units with open questions', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work 1',
            status: 'specifying',
            questions: ['@bob: Support OAuth?'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Work 2',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Work 3',
            status: 'specifying',
            questions: ['What should timeout be?'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'API-001': {
            id: 'API-001',
            title: 'Work 4',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001', 'AUTH-002', 'DASH-001', 'API-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const { queryWorkUnits } = await import('../query-work-units');
      const result = await queryWorkUnits({
        hasQuestions: true,
        output: 'json',
        cwd: testDir,
      });

      expect(result.workUnits).toHaveLength(2);
      expect(result.workUnits?.some(wu => wu.id === 'AUTH-001')).toBe(true);
      expect(result.workUnits?.some(wu => wu.id === 'DASH-001')).toBe(true);
    });
  });

  describe('Scenario: List work units by person mentioned in questions', () => {
    it('should filter by person in questions', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work 1',
            status: 'specifying',
            questions: ['@bob: Support GitHub?'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Work 2',
            status: 'specifying',
            questions: ['@alice: What is the timeout?'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Work 3',
            status: 'specifying',
            questions: ['@bob: Show user metrics?'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001', 'AUTH-002', 'DASH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const { queryWorkUnits } = await import('../query-work-units');
      const result = await queryWorkUnits({
        questionsFor: '@bob',
        output: 'json',
        cwd: testDir,
      });

      expect(result.workUnits).toHaveLength(2);
      expect(result.workUnits?.some(wu => wu.id === 'AUTH-001')).toBe(true);
      expect(result.workUnits?.some(wu => wu.id === 'DASH-001')).toBe(true);
    });
  });

  describe('Scenario: Validate example mapping data structure', () => {
    it('should validate example mapping JSON schema', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work',
            status: 'specifying',
            rules: ['Rule 1', ''],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const { validateWorkUnits } = await import('../validate-work-units');
      const result = await validateWorkUnits({ cwd: testDir });

      expect(result.valid).toBe(false);
      expect(result.errors?.some(e => e.includes('empty strings'))).toBe(true);
    });
  });

  describe('Scenario: AI refines generated scenario with proper Given/When/Then', () => {
    it('should allow AI to refine scenario structure', async () => {
      // This scenario is more about workflow than testing
      // The test validates that scenarios can be edited after generation
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'OAuth login',
            status: 'specifying',
            examples: ['User logs in with Google'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      // Generate initial scenario
      await generateScenarios({
        workUnitId: 'AUTH-001',
        cwd: testDir,
      });

      // AI can now edit the file manually or programmatically
      // Just verify the tag is preserved
      const featureContent = await readFile(join(testDir, 'spec/features/auth-001.feature'), 'utf-8');
      expect(featureContent).toContain('@AUTH-001');
    });
  });

  describe('Scenario: Show example mapping completeness metrics', () => {
    it('should calculate example mapping completeness', async () => {
      const workUnits: WorkUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Work 1',
            status: 'specifying',
            rules: ['R1', 'R2', 'R3'],
            examples: ['E1', 'E2', 'E3', 'E4', 'E5'],
            assumptions: ['A1', 'A2'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Work 2',
            status: 'specifying',
            questions: ['Q1', 'Q2', 'Q3'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          'DASH-001': {
            id: 'DASH-001',
            title: 'Work 3',
            status: 'specifying',
            rules: ['R1', 'R2'],
            examples: ['E1', 'E2', 'E3', 'E4'],
            questions: ['Q1'],
            assumptions: ['A1'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001', 'AUTH-002', 'DASH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      await writeFile(join(testDir, 'spec/work-units.json'), JSON.stringify(workUnits, null, 2));

      const result = await queryExampleMappingStats({
        output: 'json',
        cwd: testDir,
      });

      expect(result.workUnitsWithRules).toBe(2);
      expect(result.workUnitsWithExamples).toBe(2);
      expect(result.workUnitsWithQuestions).toBe(2);
      expect(result.workUnitsWithAssumptions).toBe(2);
      expect(result.avgRulesPerWorkUnit).toBeCloseTo(1.67, 1);
      expect(result.avgExamplesPerWorkUnit).toBe(3.0);
    });
  });

  describe('Scenario: Attempt to add example mapping to non-existent work unit', () => {
    it('should fail when work unit does not exist', async () => {
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

      const error = await addRule({
        workUnitId: 'AUTH-999',
        rule: 'Some rule',
        cwd: testDir,
      }).catch((e: Error) => e);

      expect(error).toBeInstanceOf(Error);
      if (error instanceof Error) {
        expect(error.message).toContain("Work unit 'AUTH-999' does not exist");
      }
    });
  });
});
