/**
 * Feature: spec/features/no-support-for-ultrathink-in-discovery-driven-feedback-loop.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 *
 * Tests validate that discover-foundation command adapts its guidance based on
 * the detected AI agent, using ULTRATHINK for Claude Code and generic language
 * for other agents (Cursor, Aider, etc.).
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { mkdir, writeFile, rm } from 'fs/promises';
import { tmpdir } from 'os';
import { discoverFoundation } from '../discover-foundation';

describe('Feature: No support for ULTRATHINK in discovery-driven feedback loop', () => {
  let tmpDir: string;
  let specDir: string;

  beforeEach(async () => {
    // Create temporary directory for tests
    tmpDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    specDir = join(tmpDir, 'spec');
    await mkdir(specDir, { recursive: true });

    // Create minimal foundation.json.draft for testing
    const draftPath = join(specDir, 'foundation.json.draft');
    await writeFile(
      draftPath,
      JSON.stringify({
        project: {
          name: '[QUESTION: What is the project name?]',
          vision: '[QUESTION: What is the elevator pitch?]',
        },
      })
    );
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(tmpDir, { recursive: true, force: true });
    // Clean up environment variable
    delete process.env.FSPEC_AGENT;
  });

  describe('Scenario: Claude Code agent receives ULTRATHINK guidance in system-reminder', () => {
    it('should include ULTRATHINK terminology for Claude Code', async () => {
      // Given I am using Claude Code (FSPEC_AGENT=claude)
      process.env.FSPEC_AGENT = 'claude';

      // When I run fspec discover-foundation
      const result = await discoverFoundation({ cwd: tmpDir });

      // Then I should see "ULTRATHINK" in the initial draft guidance
      expect(result.systemReminder).toContain('ULTRATHINK');
      expect(result.systemReminder).toContain(
        'you must ULTRATHINK the entire codebase'
      );

      // And the output should contain system-reminder tags (handled by formatAgentOutput)
      // Note: formatAgentOutput wraps the message, so we check the raw content here
      expect(result.systemReminder).toBeTruthy();
    });
  });

  describe('Scenario: Cursor IDE agent receives generic analysis guidance with emoji warning', () => {
    it('should use generic language without ULTRATHINK for Cursor', async () => {
      // Given I am using Cursor (FSPEC_AGENT=cursor)
      process.env.FSPEC_AGENT = 'cursor';

      // When I run fspec discover-foundation
      const result = await discoverFoundation({ cwd: tmpDir });

      // Then I should see "Carefully analyze the entire codebase" in the output
      expect(result.systemReminder).toContain('analyze');
      expect(result.systemReminder).toContain('codebase');

      // And I should NOT see "ULTRATHINK" in the output
      expect(result.systemReminder).not.toContain('ULTRATHINK');

      // And the output should NOT contain system-reminder tags
      // (formatAgentOutput will add **⚠️ IMPORTANT:** instead)
      expect(result.systemReminder).toBeTruthy();
    });
  });

  describe('Scenario: Aider CLI agent receives plain text analysis guidance', () => {
    it('should use generic language without ULTRATHINK for Aider', async () => {
      // Given I am using Aider (FSPEC_AGENT=aider)
      process.env.FSPEC_AGENT = 'aider';

      // When I run fspec discover-foundation
      const result = await discoverFoundation({ cwd: tmpDir });

      // Then I should see "Thoroughly examine the codebase" or similar in the output
      expect(result.systemReminder).toMatch(/analyze|examine/i);
      expect(result.systemReminder).toContain('codebase');

      // And I should NOT see "ULTRATHINK" in the output
      expect(result.systemReminder).not.toContain('ULTRATHINK');

      // And the output should contain "**IMPORTANT:**" format without emoji
      // (formatAgentOutput handles this for CLI agents)
      expect(result.systemReminder).toBeTruthy();
    });
  });

  describe('Scenario: Unknown agent receives safe default guidance without system-reminders', () => {
    it('should use generic language for unknown/default agent', async () => {
      // Given no agent is configured (no FSPEC_AGENT env var or config file)
      // (We don't set FSPEC_AGENT, so it defaults to safe default)

      // When I run fspec discover-foundation
      const result = await discoverFoundation({ cwd: tmpDir });

      // Then I should see generic analysis language in the output
      expect(result.systemReminder).toMatch(/analyze|examine/i);

      // And I should NOT see "ULTRATHINK" in the output
      expect(result.systemReminder).not.toContain('ULTRATHINK');

      // And the guidance should use safe default plain text format
      expect(result.systemReminder).toBeTruthy();
    });
  });

  describe('Scenario: Initial draft creation uses agent capability detection for ULTRATHINK', () => {
    it('should conditionally include ULTRATHINK based on agent.supportsMetaCognition', async () => {
      // Given I am implementing the discover-foundation command
      // When the initial draft system-reminder is generated

      // Test with Claude (supportsMetaCognition = true)
      process.env.FSPEC_AGENT = 'claude';
      const claudeResult = await discoverFoundation({ cwd: tmpDir });

      // Then the reminder should include "you must ULTRATHINK the entire codebase"
      expect(claudeResult.systemReminder).toContain('ULTRATHINK');
      expect(claudeResult.systemReminder).toContain('entire codebase');

      // Test with Cursor (supportsMetaCognition = false)
      process.env.FSPEC_AGENT = 'cursor';
      const cursorResult = await discoverFoundation({ cwd: tmpDir });

      // Then the reminder should include "you must thoroughly analyze the entire codebase"
      expect(cursorResult.systemReminder).not.toContain('ULTRATHINK');
      expect(cursorResult.systemReminder).toMatch(/analyze|examine/i);
      expect(cursorResult.systemReminder).toContain('codebase');
    });
  });

  describe('Scenario: Project vision field guidance checks agent capabilities for ULTRATHINK', () => {
    it('should conditionally include ULTRATHINK in project.vision field guidance', async () => {
      // Given I am implementing the project.vision field guidance
      // When the field-specific system-reminder is generated

      // Test with Claude (supportsMetaCognition = true)
      process.env.FSPEC_AGENT = 'claude';
      const claudeResult = await discoverFoundation({ cwd: tmpDir });

      // Then the guidance should include "ULTRATHINK"
      expect(claudeResult.systemReminder).toContain('ULTRATHINK');
      expect(claudeResult.systemReminder).toContain(
        'you must ULTRATHINK the entire codebase'
      );

      // Test with Aider (supportsMetaCognition = false)
      process.env.FSPEC_AGENT = 'aider';
      const aiderResult = await discoverFoundation({ cwd: tmpDir });

      // Then the guidance should include "Carefully analyze the codebase to understand its purpose"
      expect(aiderResult.systemReminder).not.toContain('ULTRATHINK');
      expect(aiderResult.systemReminder).toMatch(/analyze|examine/i);
      expect(aiderResult.systemReminder).toMatch(/purpose|understand/i);
    });
  });
});
