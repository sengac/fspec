/**
 * Feature: spec/features/add-agent-agnostic-ultrathink-to-review-command-and-update-documentation.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { review } from '../review';
import { join } from 'path';
import { writeFile } from 'fs/promises';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import { writeJsonTestFile } from '../../test-helpers/test-file-operations';

describe('Feature: Add agent-agnostic ultrathink to review command', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('review-agent-agnostic');

    // Create minimal work units file
    const workUnitsData = {
      workUnits: {
        'AUTH-001': {
          id: 'AUTH-001',
          title: 'User Login',
          description: 'Test work unit',
          status: 'done',
          type: 'story',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      },
      nextIds: { AUTH: 2 },
    };
    await writeJsonTestFile(setup.workUnitsFile, workUnitsData);

    // Create minimal feature file
    const featureContent = `@AUTH-001
Feature: User Login

  Background: User Story
    As a user
    I want to log in
    So that I can access the system

  Scenario: Login with valid credentials
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in
`;
    await writeFile(
      join(setup.featuresDir, 'user-login.feature'),
      featureContent
    );
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Review command outputs agent-specific formatting for Claude Code', () => {
    it('should output system-reminder tags when agent is claude', async () => {
      // Given I am using Claude Code as my AI agent
      // And spec/fspec-config.json contains agent: 'claude'
      const configData = { agent: 'claude' };
      await writeJsonTestFile(
        join(setup.specDir, 'fspec-config.json'),
        configData
      );

      // When I run 'fspec review AUTH-001'
      const result = await review('AUTH-001', { cwd: setup.testDir });

      // Then the output should contain <system-reminder> tags
      expect(result.output).toContain('<system-reminder>');

      // And the system-reminder should contain ACDD compliance guidance
      expect(result.output).toMatch(
        /<system-reminder>[\s\S]*ACDD[\s\S]*<\/system-reminder>/
      );
    });
  });

  describe('Scenario: Review command outputs agent-specific formatting for Cursor IDE', () => {
    it('should output warning emoji prefix when agent is cursor', async () => {
      // Given I am using Cursor IDE as my AI agent
      // And spec/fspec-config.json contains agent: 'cursor'
      const configData = { agent: 'cursor' };
      await writeJsonTestFile(
        join(setup.specDir, 'fspec-config.json'),
        configData
      );

      // When I run 'fspec review API-005' (using AUTH-001 for test)
      const result = await review('AUTH-001', { cwd: setup.testDir });

      // Then the output should contain **⚠️ IMPORTANT:** prefix
      expect(result.output).toContain('**⚠️ IMPORTANT:**');

      // And the message should contain ACDD compliance guidance
      expect(result.output).toContain('ACDD');
    });
  });

  describe('Scenario: Review command outputs agent-specific formatting for Aider CLI', () => {
    it('should output plain bold prefix when agent is aider', async () => {
      // Given I am using Aider CLI as my AI agent
      // And spec/fspec-config.json contains agent: 'aider'
      const configData = { agent: 'aider' };
      await writeJsonTestFile(
        join(setup.specDir, 'fspec-config.json'),
        configData
      );

      // When I run 'fspec review DASH-003' (using AUTH-001 for test)
      const result = await review('AUTH-001', { cwd: setup.testDir });

      // Then the output should contain **IMPORTANT:** prefix
      expect(result.output).toContain('**IMPORTANT:**');

      // And the message should contain ACDD compliance guidance
      expect(result.output).toContain('ACDD');
    });
  });

  describe('Scenario: Review command uses agent detection with fallback to safe default', () => {
    it('should use safe default formatting when no agent configured', async () => {
      // Given no agent is configured in spec/fspec-config.json
      // (don't create fspec-config.json file)

      // And FSPEC_AGENT environment variable is not set
      delete process.env.FSPEC_AGENT;

      // When I run 'fspec review TEST-001' (using AUTH-001 for test)
      const result = await review('AUTH-001', { cwd: setup.testDir });

      // Then the output should use safe default formatting
      expect(result.success).toBe(true);

      // And the output should NOT contain <system-reminder> tags
      expect(result.output).not.toContain('<system-reminder>');

      // And the output should contain plain text ACDD compliance guidance
      expect(result.output).toContain('ACDD');
    });
  });
});
