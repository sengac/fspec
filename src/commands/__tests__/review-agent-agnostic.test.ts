/**
 * Feature: spec/features/add-agent-agnostic-ultrathink-to-review-command-and-update-documentation.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { review } from '../review';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';

describe('Feature: Add agent-agnostic ultrathink to review command', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-review-test-'));

    // Create spec directory structure
    await mkdir(join(testDir, 'spec'), { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

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
    await writeFile(
      join(testDir, 'spec', 'work-units.json'),
      JSON.stringify(workUnitsData, null, 2)
    );

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
      join(testDir, 'spec', 'features', 'user-login.feature'),
      featureContent
    );
  });

  afterEach(async () => {
    if (testDir) {
      await rm(testDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Review command outputs agent-specific formatting for Claude Code', () => {
    it('should output system-reminder tags when agent is claude', async () => {
      // Given I am using Claude Code as my AI agent
      // And spec/fspec-config.json contains agent: 'claude'
      const configData = { agent: 'claude' };
      await writeFile(
        join(testDir, 'spec', 'fspec-config.json'),
        JSON.stringify(configData, null, 2)
      );

      // When I run 'fspec review AUTH-001'
      const result = await review('AUTH-001', { cwd: testDir });

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
      await writeFile(
        join(testDir, 'spec', 'fspec-config.json'),
        JSON.stringify(configData, null, 2)
      );

      // When I run 'fspec review API-005' (using AUTH-001 for test)
      const result = await review('AUTH-001', { cwd: testDir });

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
      await writeFile(
        join(testDir, 'spec', 'fspec-config.json'),
        JSON.stringify(configData, null, 2)
      );

      // When I run 'fspec review DASH-003' (using AUTH-001 for test)
      const result = await review('AUTH-001', { cwd: testDir });

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
      const result = await review('AUTH-001', { cwd: testDir });

      // Then the output should use safe default formatting
      expect(result.success).toBe(true);

      // And the output should NOT contain <system-reminder> tags
      expect(result.output).not.toContain('<system-reminder>');

      // And the output should contain plain text ACDD compliance guidance
      expect(result.output).toContain('ACDD');
    });
  });
});
