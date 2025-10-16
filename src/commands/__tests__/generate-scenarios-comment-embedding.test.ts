/**
 * Feature: spec/features/preserve-example-mapping-context-as-comments-in-generated-feature-files.feature
 *
 * This test file validates that generate-scenarios embeds example mapping context as comments.
 * Scenarios tested:
 * - Generate context-only feature file with no scenarios
 * - User story embedded in both comments and Background section
 * - System-reminder guides AI to write scenarios from examples
 * - Business rules, questions, assumptions documented in comments
 * - Comment block has visual borders
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, readFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { generateScenarios } from '../generate-scenarios';
import type { WorkUnitsData } from '../../types';

describe('Feature: Preserve example mapping context as comments', () => {
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    await mkdir(join(tmpDir, 'spec', 'features'), { recursive: true });
    await writeFile(join(tmpDir, 'spec', 'features', '.gitkeep'), '');
  });

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true });
  });

  describe('Scenario: Generate context-only feature file with no scenarios', () => {
    it('should create feature file with comment context and zero scenarios', async () => {
      // Given I have a work unit EXMAP-002 with example mapping data
      // And the work unit has 3 rules and 5 examples
      const workUnitsData: WorkUnitsData = {
        meta: {
          lastId: 2,
          lastUpdated: new Date().toISOString(),
        },
        prefixes: {
          EXMAP: {
            name: 'Example Mapping',
            nextId: 3,
          },
        },
        workUnits: {
          'EXMAP-002': {
            id: 'EXMAP-002',
            prefix: 'EXMAP',
            title: 'Test Feature',
            description: 'Test description',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            userStory: {
              role: 'developer',
              action: 'test feature',
              benefit: 'ensure quality',
            },
            rules: [
              'Passwords must be 8+ characters',
              'Account locks after 5 failed attempts',
              'Sessions expire after 24 hours',
            ],
            examples: [
              'User logs in with valid credentials',
              'User enters wrong password 5 times',
              'User session expires after 24 hours',
              'User resets password successfully',
              'User receives email notification',
            ],
            questions: [],
          },
        },
      };

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec generate-scenarios EXMAP-002"
      const result = await generateScenarios({
        workUnitId: 'EXMAP-002',
        cwd: tmpDir,
      });

      // Then a feature file should be created
      expect(result.success).toBe(true);
      expect(result.featureFile).toBeDefined();

      const content = await readFile(result.featureFile, 'utf-8');

      // And the file should contain # comment block with all rules and examples
      expect(content).toContain('# EXAMPLE MAPPING CONTEXT');
      expect(content).toContain('# BUSINESS RULES:');
      expect(content).toContain('#   1. Passwords must be 8+ characters');
      expect(content).toContain('#   2. Account locks after 5 failed attempts');
      expect(content).toContain('#   3. Sessions expire after 24 hours');
      expect(content).toContain('# EXAMPLES');
      expect(content).toContain('#   1. User logs in with valid credentials');
      expect(content).toContain('#   5. User receives email notification');

      // And the file should contain Background with user story
      expect(content).toContain('Background: User Story');
      expect(content).toContain('As a developer');
      expect(content).toContain('I want to test feature');
      expect(content).toContain('So that ensure quality');

      // And the file should contain ZERO scenarios
      const scenarioMatches = content.match(/^\s*Scenario:/gm);
      expect(scenarioMatches).toBeNull(); // No scenarios

      // And a system-reminder should tell AI to write scenarios
      expect(result.systemReminders).toBeDefined();
      expect(result.systemReminders!.length).toBeGreaterThan(0);
      const reminder = result.systemReminders!.find(r =>
        r.includes('write scenarios') || r.includes('EXAMPLES')
      );
      expect(reminder).toBeDefined();
    });
  });

  describe('Scenario: User story only in Background section, not in comments', () => {
    it('should include user story in Background section only (FEAT-012)', async () => {
      // Given I set user story for work unit using "fspec set-user-story"
      const workUnitsData: WorkUnitsData = {
        meta: { lastId: 1, lastUpdated: new Date().toISOString() },
        prefixes: { TEST: { name: 'Test', nextId: 2 } },
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            prefix: 'TEST',
            title: 'Login Feature',
            description: 'Test',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            userStory: {
              role: 'user',
              action: 'log in securely',
              benefit: 'access my account',
            },
            rules: ['Password required'],
            examples: ['User logs in'],
            questions: [],
          },
        },
      };

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec generate-scenarios" on the work unit
      const result = await generateScenarios({
        workUnitId: 'TEST-001',
        cwd: tmpDir,
      });

      const content = await readFile(result.featureFile, 'utf-8');

      // Then the feature file # comments should NOT contain the user story
      expect(content).not.toContain('# USER STORY:');

      // And the Background section should contain the user story
      expect(content).toContain('Background: User Story');
      expect(content).toContain('As a user');
      expect(content).toContain('I want to log in securely');
      expect(content).toContain('So that access my account');

      // And the user story should only appear in Background, not comments
      const hasUserStoryInComments = content.includes('# USER STORY:');
      expect(hasUserStoryInComments).toBe(false);
    });
  });

  describe('Scenario: Comment block has visual borders for easy identification', () => {
    it('should add visual separator lines at top and bottom of comment block', async () => {
      // Given I generate scenarios from a work unit
      const workUnitsData: WorkUnitsData = {
        meta: { lastId: 1, lastUpdated: new Date().toISOString() },
        prefixes: { TEST: { name: 'Test', nextId: 2 } },
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            prefix: 'TEST',
            title: 'Test',
            description: 'Test',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            userStory: { role: 'user', action: 'test', benefit: 'quality' },
            rules: ['Rule 1'],
            examples: ['Example 1'],
            questions: [],
          },
        },
      };

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      const result = await generateScenarios({
        workUnitId: 'TEST-001',
        cwd: tmpDir,
      });

      // When I view the generated feature file
      const content = await readFile(result.featureFile, 'utf-8');

      // Then the comment block should have "# ===" separator lines at top and bottom
      expect(content).toMatch(/# ={3,}/); // At least 3 equals signs

      // And the block should be visually distinct when scrolling
      const lines = content.split('\n');
      const separatorLines = lines.filter(line => /^#\s*={3,}/.test(line.trim()));
      expect(separatorLines.length).toBeGreaterThanOrEqual(2); // Top and bottom borders
    });
  });

  describe('Scenario: Answered questions preserved in comments', () => {
    it('should include answered questions in comment context', async () => {
      // Given I answer question during example mapping
      // And the answer says "OAuth support deferred to Phase 2"
      const workUnitsData: WorkUnitsData = {
        meta: { lastId: 1, lastUpdated: new Date().toISOString() },
        prefixes: { TEST: { name: 'Test', nextId: 2 } },
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            prefix: 'TEST',
            title: 'Auth Feature',
            description: 'Test',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            userStory: { role: 'user', action: 'authenticate', benefit: 'security' },
            rules: ['OAuth considered'],
            examples: ['User logs in with password'],
            questions: [
              {
                text: '@human: Should we support OAuth?',
                selected: 'Yes, but deferred to Phase 2',
              },
            ],
          },
        },
      };

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I generate scenarios from the work unit
      const result = await generateScenarios({
        workUnitId: 'TEST-001',
        cwd: tmpDir,
      });

      const content = await readFile(result.featureFile, 'utf-8');

      // Then the feature file should contain "# QUESTIONS (ANSWERED):" section
      expect(content).toContain('# QUESTIONS (ANSWERED');

      // And the section should show "Q: OAuth support? A: Phase 2"
      expect(content).toContain('Q: Should we support OAuth?');
      expect(content).toContain('A: Yes, but deferred to Phase 2');

      // And developers reading the file understand deferred decisions
      expect(content).toMatch(/# QUESTIONS[\s\S]*OAuth[\s\S]*Phase 2/);
    });
  });

  describe('Scenario: Assumptions documented in comments', () => {
    it('should include assumptions in comment context', async () => {
      // Given I add assumption "email verification handled by external service"
      const workUnitsData: WorkUnitsData = {
        meta: { lastId: 1, lastUpdated: new Date().toISOString() },
        prefixes: { TEST: { name: 'Test', nextId: 2 } },
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            prefix: 'TEST',
            title: 'Registration',
            description: 'Test',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            userStory: { role: 'user', action: 'register', benefit: 'create account' },
            rules: ['Email required'],
            examples: ['User registers with email'],
            questions: [],
            assumptions: [
              'Email verification handled by external service',
              'Password strength validation uses external library',
            ],
          },
        },
      };

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I generate scenarios from the work unit
      const result = await generateScenarios({
        workUnitId: 'TEST-001',
        cwd: tmpDir,
      });

      const content = await readFile(result.featureFile, 'utf-8');

      // Then the feature file should contain "# ASSUMPTIONS:" section
      expect(content).toContain('# ASSUMPTIONS:');

      // And the section should list the email verification assumption
      expect(content).toContain('Email verification handled by external service');

      // And AI agents know not to write scenarios for email verification
      expect(content).toContain('# ASSUMPTIONS:');
      expect(content).toMatch(/# ASSUMPTIONS:[\s\S]*Email verification/);
    });
  });

  describe('Scenario: Empty example map creates minimal comment block', () => {
    it('should create minimal comment block when no example mapping data exists', async () => {
      // Given a work unit has no rules, examples, or questions
      const workUnitsData: WorkUnitsData = {
        meta: { lastId: 1, lastUpdated: new Date().toISOString() },
        prefixes: { TEST: { name: 'Test', nextId: 2 } },
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            prefix: 'TEST',
            title: 'Empty Feature',
            description: 'Test',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            userStory: { role: 'user', action: 'test', benefit: 'quality' },
            rules: [],
            examples: ['At least one example required'], // Need at least one
            questions: [],
          },
        },
      };

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I run "fspec generate-scenarios" on it
      const result = await generateScenarios({
        workUnitId: 'TEST-001',
        cwd: tmpDir,
      });

      const content = await readFile(result.featureFile, 'utf-8');

      // Then the feature file should contain a comment block
      expect(content).toContain('# EXAMPLE MAPPING CONTEXT');

      // And the file should still contain Background section
      expect(content).toContain('Background: User Story');
    });
  });

  describe('Scenario: System-reminder guides AI to write scenarios from examples', () => {
    it('should emit system-reminder instructing AI to write scenarios', async () => {
      // Given I generate scenarios for a work unit
      // And the feature file contains "# EXAMPLES: 1. User logs in with valid creds"
      const workUnitsData: WorkUnitsData = {
        meta: { lastId: 1, lastUpdated: new Date().toISOString() },
        prefixes: { TEST: { name: 'Test', nextId: 2 } },
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            prefix: 'TEST',
            title: 'Login',
            description: 'Test',
            type: 'story',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            userStory: { role: 'user', action: 'log in', benefit: 'access' },
            rules: ['Valid credentials required'],
            examples: [
              'User logs in with valid credentials and sees dashboard',
              'User logs in with invalid credentials and sees error',
            ],
            questions: [],
          },
        },
      };

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When the command completes
      const result = await generateScenarios({
        workUnitId: 'TEST-001',
        cwd: tmpDir,
      });

      // Then a system-reminder should be emitted
      expect(result.systemReminders).toBeDefined();
      expect(result.systemReminders!.length).toBeGreaterThan(0);

      const reminder = result.systemReminders!.join('\n');

      // And the reminder should say "write scenarios based on # EXAMPLES section"
      expect(reminder).toMatch(/write.*scenarios/i);
      expect(reminder).toMatch(/EXAMPLES/i);

      // And the reminder should list all examples found
      expect(reminder).toContain('2'); // Two examples

      // And the reminder should instruct using Edit tool
      expect(reminder).toMatch(/Edit tool/i);
    });
  });
});
