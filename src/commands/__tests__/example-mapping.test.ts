import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, readFile, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import {
  addRule,
  addExample,
  addQuestion,
  addAssumption,
  removeRule,
  removeExample,
  removeQuestion,
  answerQuestion,
  generateScenarios,
  importExampleMap,
  exportExampleMap,
  queryExampleMappingStats,
} from '../example-mapping';
import { showWorkUnit } from '../work-unit';
import { queryWorkUnit } from '../work-unit';
import { validateWorkUnits } from '../work-unit';

describe('Feature: Example Mapping Integration', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;
  let featuresDir: string;

  beforeEach(async () => {
    // Create temporary directory for each test
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');
    featuresDir = join(specDir, 'features');

    // Create spec directory structure
    await mkdir(specDir, { recursive: true });
    await mkdir(featuresDir, { recursive: true });

    // Initialize work units file
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
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Add rule to work unit during discovery', () => {
    it('should add rule to work unit in specifying state', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "specifying"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth login',
        status: 'specifying',
        rules: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec add-rule AUTH-001 'Users must authenticate before accessing protected resources'"
      await addRule(
        'AUTH-001',
        'Users must authenticate before accessing protected resources',
        { cwd: testDir }
      );

      // Then the command should succeed
      // And the work unit should have 1 rule
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].rules).toHaveLength(1);

      // And the rule should be correct
      expect(updatedWorkUnits.workUnits['AUTH-001'].rules[0]).toBe(
        'Users must authenticate before accessing protected resources'
      );
    });
  });

  describe('Scenario: Add multiple rules to work unit', () => {
    it('should accumulate rules in order', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "specifying"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        rules: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I add multiple rules
      await addRule('AUTH-001', 'OAuth tokens expire after 1 hour', {
        cwd: testDir,
      });
      await addRule('AUTH-001', 'Refresh tokens valid for 30 days', {
        cwd: testDir,
      });
      await addRule('AUTH-001', 'Only one active session per user', {
        cwd: testDir,
      });

      // Then the work unit should have 3 rules in order
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].rules).toHaveLength(3);
      expect(updatedWorkUnits.workUnits['AUTH-001'].rules[0]).toBe(
        'OAuth tokens expire after 1 hour'
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].rules[1]).toBe(
        'Refresh tokens valid for 30 days'
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].rules[2]).toBe(
        'Only one active session per user'
      );
    });
  });

  describe('Scenario: Add example that will become a scenario', () => {
    it('should add example to work unit', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "specifying"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        examples: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec add-example AUTH-001 'User logs in with Google account'"
      await addExample('AUTH-001', 'User logs in with Google account', {
        cwd: testDir,
      });

      // Then the command should succeed
      // And the work unit should have 1 example
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].examples).toHaveLength(1);
      expect(updatedWorkUnits.workUnits['AUTH-001'].examples[0]).toBe(
        'User logs in with Google account'
      );
    });
  });

  describe('Scenario: Add multiple examples for scenario candidates', () => {
    it('should accumulate multiple examples', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "specifying"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        examples: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I add multiple examples
      await addExample('AUTH-001', 'User logs in with valid credentials', {
        cwd: testDir,
      });
      await addExample('AUTH-001', 'User logs in with expired token', {
        cwd: testDir,
      });
      await addExample('AUTH-001', 'User token auto-refreshes before expiry', {
        cwd: testDir,
      });
      await addExample('AUTH-001', 'User logs out and token is invalidated', {
        cwd: testDir,
      });

      // Then the work unit should have 4 examples
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].examples).toHaveLength(4);
    });
  });

  describe('Scenario: Add question that needs human answer', () => {
    it('should add question to work unit', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "specifying"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        questions: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec add-question AUTH-001 'Should we support GitHub Enterprise?'"
      await addQuestion('AUTH-001', 'Should we support GitHub Enterprise?', {
        cwd: testDir,
      });

      // Then the command should succeed
      // And the work unit should have 1 question
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].questions).toHaveLength(1);
      expect(updatedWorkUnits.workUnits['AUTH-001'].questions[0]).toEqual({
        text: 'Should we support GitHub Enterprise?',
        selected: false,
      });
    });
  });

  describe('Scenario: Add question mentioning specific person', () => {
    it('should preserve @mention syntax for notifications', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "specifying"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        questions: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec add-question AUTH-001 '@bob: What is the token expiry policy?'"
      await addQuestion('AUTH-001', '@bob: What is the token expiry policy?', {
        cwd: testDir,
      });

      // Then the command should succeed
      // And the question should contain "@bob:"
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(
        updatedWorkUnits.workUnits['AUTH-001'].questions[0].text
      ).toContain('@bob:');

      // And the question should be parseable for notifications
      const question = updatedWorkUnits.workUnits['AUTH-001'].questions[0].text;
      const mentionMatch = question.match(/@(\w+):/);
      expect(mentionMatch).toBeTruthy();
      expect(mentionMatch![1]).toBe('bob');
    });
  });

  describe('Scenario: Add assumption about requirements', () => {
    it('should add assumption to work unit', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "specifying"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        assumptions: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec add-assumption AUTH-001 'Users have valid OAuth accounts'"
      await addAssumption('AUTH-001', 'Users have valid OAuth accounts', {
        cwd: testDir,
      });

      // Then the command should succeed
      // And the work unit should have 1 assumption
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].assumptions).toHaveLength(
        1
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].assumptions[0]).toBe(
        'Users have valid OAuth accounts'
      );
    });
  });

  describe('Scenario: Complete example mapping session with all four artifacts', () => {
    it('should support all four example mapping artifact types', async () => {
      // Given I have a project with spec directory
      // And a work unit "AUTH-001" exists with status "specifying"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        rules: [],
        examples: [],
        questions: [],
        assumptions: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I add all four types of artifacts
      await addRule('AUTH-001', 'OAuth tokens expire after 1 hour', {
        cwd: testDir,
      });
      await addRule(
        'AUTH-001',
        'Users must authenticate before accessing protected resources',
        { cwd: testDir }
      );
      await addExample('AUTH-001', 'User logs in with Google', {
        cwd: testDir,
      });
      await addExample('AUTH-001', 'User logs in with expired token', {
        cwd: testDir,
      });
      await addQuestion('AUTH-001', '@security-team: Do we need PKCE flow?', {
        cwd: testDir,
      });
      await addAssumption('AUTH-001', 'Users have valid email addresses', {
        cwd: testDir,
      });

      // Then the work unit should have all artifacts
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].rules).toHaveLength(2);
      expect(updatedWorkUnits.workUnits['AUTH-001'].examples).toHaveLength(2);
      expect(updatedWorkUnits.workUnits['AUTH-001'].questions).toHaveLength(1);
      expect(updatedWorkUnits.workUnits['AUTH-001'].assumptions).toHaveLength(
        1
      );
    });
  });

  describe('Scenario: Show work unit with example mapping data', () => {
    it('should display all four artifact types grouped', async () => {
      // Given I have a project with spec directory
      // And a work unit with example mapping data
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        rules: [
          'OAuth tokens expire after 1 hour',
          'Users must authenticate first',
        ],
        examples: [
          'User logs in with Google',
          'User logs in with expired token',
        ],
        questions: [
          { text: '@bob: Support GitHub Enterprise?', selected: false },
        ],
        assumptions: ['Users have valid OAuth accounts'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec show-work-unit AUTH-001"
      const output = await showWorkUnit('AUTH-001', { cwd: testDir });

      // Then the output should display all artifacts grouped by type
      expect(output).toContain('OAuth tokens expire after 1 hour');
      expect(output).toContain('Users must authenticate first');
      expect(output).toContain('User logs in with Google');
      expect(output).toContain('User logs in with expired token');
      expect(output).toContain('@bob: Support GitHub Enterprise?');
      expect(output).toContain('Users have valid OAuth accounts');
    });
  });

  describe('Scenario: Remove rule by index', () => {
    it('should remove rule at specified index', async () => {
      // Given I have a project with spec directory
      // And a work unit has 3 rules
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        rules: [
          'OAuth tokens expire after 1 hour',
          'Users must authenticate first',
          'Refresh tokens valid for 30 days',
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec remove-rule AUTH-001 1"
      await removeRule('AUTH-001', 1, { cwd: testDir });

      // Then the command should succeed
      // And the work unit should have 2 rules
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].rules).toHaveLength(2);

      // And the second rule should now be "Refresh tokens valid for 30 days"
      expect(updatedWorkUnits.workUnits['AUTH-001'].rules[1]).toBe(
        'Refresh tokens valid for 30 days'
      );
    });
  });

  describe('Scenario: Remove example by index', () => {
    it('should remove example at specified index', async () => {
      // Given I have a project with spec directory
      // And a work unit has 3 examples
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        examples: [
          'User logs in with Google',
          'User logs in with expired token',
          'User logs out',
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec remove-example AUTH-001 0"
      await removeExample('AUTH-001', 0, { cwd: testDir });

      // Then the command should succeed
      // And the work unit should have 2 examples
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].examples).toHaveLength(2);

      // And the first example should be "User logs in with expired token"
      expect(updatedWorkUnits.workUnits['AUTH-001'].examples[0]).toBe(
        'User logs in with expired token'
      );
    });
  });

  describe('Scenario: Remove question by index', () => {
    it('should remove question at specified index', async () => {
      // Given I have a project with spec directory
      // And a work unit has 2 questions
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        questions: [
          { text: '@bob: Support GitHub Enterprise?', selected: false },
          { text: 'What is the token expiry policy?', selected: false },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec remove-question AUTH-001 0"
      await removeQuestion('AUTH-001', 0, { cwd: testDir });

      // Then the command should succeed
      // And the work unit should have 1 question
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].questions).toHaveLength(1);
    });
  });

  describe('Scenario: Attempt to remove item with invalid index', () => {
    it('should fail with helpful error message', async () => {
      // Given I have a project with spec directory
      // And a work unit has 2 rules
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        rules: ['Rule 1', 'Rule 2'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec remove-rule AUTH-001 5"
      // Then the command should fail
      await expect(removeRule('AUTH-001', 5, { cwd: testDir })).rejects.toThrow(
        'Index 5 out of range'
      );

      // And the error should show valid range
      await expect(removeRule('AUTH-001', 5, { cwd: testDir })).rejects.toThrow(
        'Valid indices: 0-1'
      );
    });
  });

  describe('Scenario: Answer question and add to assumptions', () => {
    it('should remove question and add assumption', async () => {
      // Given I have a project with spec directory
      // And a work unit has 1 question
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        questions: [
          {
            text: '@bob: Should we support GitHub Enterprise?',
            selected: false,
          },
        ],
        assumptions: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec answer-question AUTH-001 0 'No, only GitHub.com supported' --add-to=assumptions"
      await answerQuestion('AUTH-001', 0, 'No, only GitHub.com supported', {
        cwd: testDir,
        addTo: 'assumptions',
      });

      // Then the command should succeed
      // And the question should be marked as selected
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].questions[0].selected).toBe(
        true
      );

      // And the work unit should have 1 assumption
      expect(updatedWorkUnits.workUnits['AUTH-001'].assumptions).toHaveLength(
        1
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].assumptions[0]).toContain(
        'GitHub.com'
      );
    });
  });

  describe('Scenario: Answer question and add to rules', () => {
    it('should remove question and add rule', async () => {
      // Given I have a project with spec directory
      // And a work unit has 1 question
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        questions: [
          { text: 'What is the maximum session length?', selected: false },
        ],
        rules: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec answer-question AUTH-001 0 '24 hours' --add-to=rules"
      await answerQuestion('AUTH-001', 0, '24 hours', {
        cwd: testDir,
        addTo: 'rules',
      });

      // Then the command should succeed
      // And the question should be marked as selected
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].questions[0].selected).toBe(
        true
      );

      // And the work unit should have 1 rule
      expect(updatedWorkUnits.workUnits['AUTH-001'].rules).toHaveLength(1);
      expect(updatedWorkUnits.workUnits['AUTH-001'].rules[0]).toContain(
        '24 hours'
      );
    });
  });

  describe('Scenario: Answer question without adding to rules or assumptions', () => {
    it('should just remove question', async () => {
      // Given I have a project with spec directory
      // And a work unit has 1 question
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        questions: [{ text: 'Is this feature needed?', selected: false }],
        rules: [],
        assumptions: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec answer-question AUTH-001 0 'No, descoping' --add-to=none"
      await answerQuestion('AUTH-001', 0, 'No, descoping', {
        cwd: testDir,
        addTo: 'none',
      });

      // Then the command should succeed
      // And the question should be marked as selected
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].questions[0].selected).toBe(
        true
      );

      // And no rules or assumptions added
      expect(updatedWorkUnits.workUnits['AUTH-001'].rules).toHaveLength(0);
      expect(updatedWorkUnits.workUnits['AUTH-001'].assumptions).toHaveLength(
        0
      );
    });
  });

  describe('Scenario: Generate Gherkin scenarios from examples', () => {
    it('should create scenarios from examples with auto-tagging', async () => {
      // Given I have a project with spec directory
      // And a work unit has examples
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        examples: [
          'User logs in with Google account',
          'User logs in with expired token',
          'User token auto-refreshes',
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec generate-scenarios AUTH-001"
      const result = await generateScenarios('AUTH-001', { cwd: testDir });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And a feature file should be created
      expect(result.featureFile).toBeTruthy();

      // And the feature file should contain 3 scenarios tagged with @AUTH-001
      const featureContent = await readFile(
        join(featuresDir, result.featureFile),
        'utf-8'
      );
      const scenarioMatches = featureContent.match(/Scenario:/g);
      expect(scenarioMatches).toHaveLength(3);
      expect(featureContent).toContain('@AUTH-001');

      // And the work unit examples should still be preserved
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].examples).toHaveLength(3);
    });
  });

  describe('Scenario: Generate scenarios with Given/When/Then template', () => {
    it('should create scenario with placeholder steps', async () => {
      // Given I have a project with spec directory
      // And a work unit has 1 example
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        examples: ['User logs in with valid credentials'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec generate-scenarios AUTH-001"
      const result = await generateScenarios('AUTH-001', { cwd: testDir });

      // Then the generated scenario should have Given/When/Then structure
      const featureContent = await readFile(
        join(featuresDir, result.featureFile),
        'utf-8'
      );
      expect(featureContent).toContain('@AUTH-001');
      expect(featureContent).toContain(
        'Scenario: User logs in with valid credentials'
      );
      expect(featureContent).toContain('Given');
      expect(featureContent).toContain('When');
      expect(featureContent).toContain('Then');
    });
  });

  describe('Scenario: Generate scenarios into existing feature file', () => {
    it('should append scenarios to existing feature', async () => {
      // Given I have a project with spec directory
      // And a feature file exists
      const existingFeature = `@auth\nFeature: Authentication\n\nScenario: Existing scenario\n  Given existing\n  When existing\n  Then existing`;
      await writeFile(
        join(featuresDir, 'authentication.feature'),
        existingFeature
      );

      // And a work unit has examples
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        examples: ['User logs in with Google'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec generate-scenarios AUTH-001 --feature=authentication"
      await generateScenarios('AUTH-001', {
        cwd: testDir,
        feature: 'authentication',
      });

      // Then scenarios should be appended to existing file
      const featureContent = await readFile(
        join(featuresDir, 'authentication.feature'),
        'utf-8'
      );
      expect(featureContent).toContain('Existing scenario');
      expect(featureContent).toContain('User logs in with Google');
      expect(featureContent).toContain('@AUTH-001');
    });
  });

  describe('Scenario: Generate scenarios into new feature file', () => {
    it('should create new feature file with scenarios', async () => {
      // Given I have a project with spec directory
      // And no feature file exists for "oauth-login"
      // And a work unit has examples
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        examples: ['User logs in with OAuth'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec generate-scenarios AUTH-001 --feature=oauth-login"
      const result = await generateScenarios('AUTH-001', {
        cwd: testDir,
        feature: 'oauth-login',
      });

      // Then a new feature file should be created
      const featurePath = join(featuresDir, 'oauth-login.feature');
      const featureContent = await readFile(featurePath, 'utf-8');

      // And the file should contain the scenario
      expect(featureContent).toContain('User logs in with OAuth');
      expect(featureContent).toContain('@AUTH-001');
    });
  });

  describe('Scenario: Prevent moving to testing when questions remain unanswered', () => {
    it('should block transition with unanswered questions', async () => {
      // Given I have a project with spec directory
      // And a work unit exists with unanswered questions
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        questions: [
          { text: '@bob: Should we support OAuth 2.0?', selected: false },
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // Import updateWorkUnit for state transition test
      const { updateWorkUnit } = await import('../work-unit');

      // When I run "fspec update-work-unit AUTH-001 --status=testing"
      // Then the command should fail
      await expect(
        updateWorkUnit('AUTH-001', { status: 'testing' }, { cwd: testDir })
      ).rejects.toThrow('Unanswered questions prevent state transition');

      // And the error should list the question
      await expect(
        updateWorkUnit('AUTH-001', { status: 'testing' }, { cwd: testDir })
      ).rejects.toThrow('@bob: Should we support OAuth 2.0?');

      // And the error should suggest solution
      await expect(
        updateWorkUnit('AUTH-001', { status: 'testing' }, { cwd: testDir })
      ).rejects.toThrow(
        "Answer questions with 'fspec answer-question' or remove them"
      );
    });
  });

  describe('Scenario: Warn when moving to testing with no examples', () => {
    it('should warn but allow transition without examples', async () => {
      // Given I have a project with spec directory
      // And a work unit has no examples
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        examples: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // Create a scenario to satisfy the prerequisite (or mock it)
      await writeFile(
        join(featuresDir, 'auth.feature'),
        `@auth\nFeature: Auth\n\n@AUTH-001\nScenario: Login\nGiven test\nWhen test\nThen test`
      );

      // Import updateWorkUnit
      const { updateWorkUnit } = await import('../work-unit');

      // When I run "fspec update-work-unit AUTH-001 --status=testing"
      const result = await updateWorkUnit(
        'AUTH-001',
        { status: 'testing' },
        { cwd: testDir }
      );

      // Then the transition should succeed (warnings may be in result.warnings)
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].status).toBe('testing');
    });
  });

  describe('Scenario: Bulk add multiple items from JSON', () => {
    it('should import all four artifact types from JSON', async () => {
      // Given I have a project with spec directory
      // And a work unit exists
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        rules: [],
        examples: [],
        questions: [],
        assumptions: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // And I have a JSON file with example mapping data
      const exampleMapData = {
        rules: [
          'OAuth tokens expire after 1 hour',
          'Users must authenticate first',
        ],
        examples: [
          'User logs in with Google',
          'User logs in with expired token',
        ],
        questions: ['@bob: Support GitHub Enterprise?'],
        assumptions: ['Users have valid OAuth accounts'],
      };
      const jsonPath = join(testDir, 'example-mapping.json');
      await writeFile(jsonPath, JSON.stringify(exampleMapData, null, 2));

      // When I run "fspec import-example-map AUTH-001 example-mapping.json"
      await importExampleMap('AUTH-001', jsonPath, { cwd: testDir });

      // Then the work unit should have all artifacts
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['AUTH-001'].rules).toHaveLength(2);
      expect(updatedWorkUnits.workUnits['AUTH-001'].examples).toHaveLength(2);
      expect(updatedWorkUnits.workUnits['AUTH-001'].questions).toHaveLength(1);
      expect(updatedWorkUnits.workUnits['AUTH-001'].assumptions).toHaveLength(
        1
      );
    });
  });

  describe('Scenario: Export example mapping to JSON', () => {
    it('should export all artifacts to JSON file', async () => {
      // Given I have a project with spec directory
      // And a work unit has example mapping data
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        rules: ['OAuth tokens expire after 1 hour'],
        examples: ['User logs in with Google'],
        questions: ['@bob: Support GitHub Enterprise?'],
        assumptions: ['Users have valid OAuth accounts'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec export-example-map AUTH-001 --output=auth-example-map.json"
      const outputPath = join(testDir, 'auth-example-map.json');
      await exportExampleMap('AUTH-001', { cwd: testDir, output: outputPath });

      // Then the file should contain valid JSON
      const exportedData = JSON.parse(await readFile(outputPath, 'utf-8'));

      // And the JSON should have all four arrays
      expect(exportedData.rules).toHaveLength(1);
      expect(exportedData.examples).toHaveLength(1);
      expect(exportedData.questions).toHaveLength(1);
      expect(exportedData.assumptions).toHaveLength(1);
    });
  });

  describe('Scenario: Find all work units with unanswered questions', () => {
    it('should query work units that have questions', async () => {
      // Given I have a project with spec directory
      // And work units exist with varying questions
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'Auth 1',
        status: 'specifying',
        questions: [{ text: '@bob: Support OAuth?', selected: false }],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        title: 'Auth 2',
        status: 'specifying',
        questions: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['DASH-001'] = {
        id: 'DASH-001',
        title: 'Dashboard',
        status: 'specifying',
        questions: [{ text: 'What should timeout be?', selected: false }],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['API-001'] = {
        id: 'API-001',
        title: 'API',
        status: 'specifying',
        questions: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec query work-units --has-questions --output=json"
      const result = await queryWorkUnit(null, {
        hasQuestions: true,
        output: 'json',
        cwd: testDir,
      });

      // Then the output should contain 2 work units
      const json = JSON.parse(result);
      expect(json).toHaveLength(2);

      // And they should be the ones with questions
      const ids = json.map((wu: { id: string }) => wu.id);
      expect(ids).toContain('AUTH-001');
      expect(ids).toContain('DASH-001');
    });
  });

  describe('Scenario: List work units by person mentioned in questions', () => {
    it('should filter by @mention in questions', async () => {
      // Given I have a project with spec directory
      // And work units have questions with different mentions
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'Auth 1',
        status: 'specifying',
        questions: [{ text: '@bob: Support GitHub?', selected: false }],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        title: 'Auth 2',
        status: 'specifying',
        questions: [{ text: '@alice: What is the timeout?', selected: false }],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['DASH-001'] = {
        id: 'DASH-001',
        title: 'Dashboard',
        status: 'specifying',
        questions: [{ text: '@bob: Show user metrics?', selected: false }],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec query work-units --questions-for=@bob --output=json"
      const result = await queryWorkUnit(null, {
        questionsFor: '@bob',
        output: 'json',
        cwd: testDir,
      });

      // Then the output should contain 2 work units
      const json = JSON.parse(result);
      expect(json).toHaveLength(2);

      // And they should be the ones with @bob mentions
      const ids = json.map((wu: { id: string }) => wu.id);
      expect(ids).toContain('AUTH-001');
      expect(ids).toContain('DASH-001');
    });
  });

  describe('Scenario: Validate example mapping data structure', () => {
    it('should validate all arrays contain valid strings', async () => {
      // Given I have a project with spec directory
      // And a work unit exists with example mapping data
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'OAuth',
        status: 'specifying',
        rules: ['Valid rule'],
        examples: ['Valid example'],
        questions: [{ text: 'Valid question', selected: false }],
        assumptions: ['Valid assumption'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec validate-work-units"
      const result = await validateWorkUnits({ cwd: testDir });

      // Then validation should check all artifact types
      expect(result.valid).toBe(true);
      expect(result.checks).toContain('rules are strings');
      expect(result.checks).toContain('examples are strings');
      expect(result.checks).toContain('questions are QuestionItem objects');
      expect(result.checks).toContain('assumptions are strings');
    });
  });

  describe('Scenario: Show example mapping completeness metrics', () => {
    it('should calculate statistics across work units', async () => {
      // Given I have a project with spec directory
      // And work units exist with varying example mapping data
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'Auth 1',
        status: 'specifying',
        rules: ['r1', 'r2', 'r3'],
        examples: ['e1', 'e2', 'e3', 'e4', 'e5'],
        questions: [],
        assumptions: ['a1', 'a2'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['AUTH-002'] = {
        id: 'AUTH-002',
        title: 'Auth 2',
        status: 'specifying',
        rules: [],
        examples: [],
        questions: [
          { text: 'q1', selected: false },
          { text: 'q2', selected: false },
          { text: 'q3', selected: false },
        ],
        assumptions: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.workUnits['DASH-001'] = {
        id: 'DASH-001',
        title: 'Dashboard',
        status: 'specifying',
        rules: ['r1', 'r2'],
        examples: ['e1', 'e2', 'e3', 'e4'],
        questions: [{ text: 'q1', selected: false }],
        assumptions: ['a1'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec query example-mapping-stats --output=json"
      const result = await queryExampleMappingStats({
        cwd: testDir,
        output: 'json',
      });

      // Then the output should show statistics
      const stats = JSON.parse(result);
      expect(stats.workUnitsWithRules).toBe(2);
      expect(stats.workUnitsWithExamples).toBe(2);
      expect(stats.workUnitsWithQuestions).toBe(2);
      expect(stats.workUnitsWithAssumptions).toBe(2);
    });
  });

  describe('Scenario: Attempt to add example mapping to non-existent work unit', () => {
    it('should fail with not found error', async () => {
      // Given I have a project with spec directory
      // And no work unit "AUTH-999" exists

      // When I run "fspec add-rule AUTH-999 'Some rule'"
      // Then the command should fail
      await expect(
        addRule('AUTH-999', 'Some rule', { cwd: testDir })
      ).rejects.toThrow("Work unit 'AUTH-999' does not exist");
    });
  });
});
