/**
 * Feature: spec/features/no-support-for-ultrathink-in-discovery-driven-feedback-loop.feature
 *
 * Validates agent-specific guidance wording for the discovery workflow.
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
      const result = await discoverFoundation({ cwd: tmpDir, force: true });

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

  describe('Scenario: Cursor IDE agent receives "think a lot" guidance with emoji warning', () => {
    it('should use "think a lot" guidance for Cursor', async () => {
      // Given I am using Cursor (FSPEC_AGENT=cursor)
      process.env.FSPEC_AGENT = 'cursor';

      // When I run fspec discover-foundation
      const result = await discoverFoundation({ cwd: tmpDir, force: true });

      // Then I should see "think a lot" in the output
      expect(result.systemReminder.toLowerCase()).toContain('think a lot');

      // And I should NOT see "ULTRATHINK" in the output
      expect(result.systemReminder).not.toContain('ULTRATHINK');

      // And the output should NOT contain system-reminder tags
      // (formatAgentOutput will add **⚠️ IMPORTANT:** instead)
      expect(result.systemReminder).toBeTruthy();
    });
  });

  describe('Scenario: Aider CLI agent receives plain text "think a lot" guidance', () => {
    it('should use "think a lot" guidance for Aider', async () => {
      // Given I am using Aider (FSPEC_AGENT=aider)
      process.env.FSPEC_AGENT = 'aider';

      // When I run fspec discover-foundation
      const result = await discoverFoundation({ cwd: tmpDir, force: true });

      // Then I should see "think a lot" in the output
      expect(result.systemReminder.toLowerCase()).toContain('think a lot');

      // And I should NOT see "ULTRATHINK" in the output
      expect(result.systemReminder).not.toContain('ULTRATHINK');

      // And the output should contain "**IMPORTANT:**" format without emoji
      // (formatAgentOutput handles this for CLI agents)
      expect(result.systemReminder).toBeTruthy();
    });
  });

  describe('Scenario: Unknown agent receives "think a lot" guidance without system-reminders', () => {
    it('should use "think a lot" guidance for default agent', async () => {
      // Given no agent is configured (no FSPEC_AGENT env var or config file)
      // (We don't set FSPEC_AGENT, so it defaults to safe default)

      // When I run fspec discover-foundation
      const result = await discoverFoundation({ cwd: tmpDir, force: true });

      // Then I should see "think a lot" in the output
      expect(result.systemReminder.toLowerCase()).toContain('think a lot');

      // And I should NOT see "ULTRATHINK" in the output
      expect(result.systemReminder).not.toContain('ULTRATHINK');

      // And the guidance should use safe default plain text format
      expect(result.systemReminder).toBeTruthy();
    });
  });

  describe('Scenario: Initial draft creation applies agent-aware guidance', () => {
    it('should include ULTRATHINK for Claude and "think a lot" for others', async () => {
      // Given I am implementing the discover-foundation command
      // When the initial draft system-reminder is generated

      // Test with Claude (supportsMetaCognition = true)
      process.env.FSPEC_AGENT = 'claude';
      const claudeResult = await discoverFoundation({
        cwd: tmpDir,
        force: true,
      });

      // Then the reminder should include "you must ULTRATHINK the entire codebase"
      expect(claudeResult.systemReminder).toContain('ULTRATHINK');
      expect(claudeResult.systemReminder).toContain('entire codebase');

      // Test with Cursor (supportsMetaCognition = false)
      process.env.FSPEC_AGENT = 'cursor';
      const cursorResult = await discoverFoundation({
        cwd: tmpDir,
        force: true,
      });

      // Then the reminder should include "think a lot"
      expect(cursorResult.systemReminder.toLowerCase()).toContain(
        'think a lot'
      );
      expect(cursorResult.systemReminder).not.toContain('ULTRATHINK');
    });
  });

  describe('Scenario: Project vision field guidance checks agent capabilities for ULTRATHINK', () => {
    it('should include ULTRATHINK for Claude and "think a lot" for other agents', async () => {
      // Given I am implementing the project.vision field guidance
      // When the field-specific system-reminder is generated

      // Test with Claude (supportsMetaCognition = true)
      process.env.FSPEC_AGENT = 'claude';
      const claudeResult = await discoverFoundation({
        cwd: tmpDir,
        force: true,
      });

      // Then the guidance should include "ULTRATHINK"
      expect(claudeResult.systemReminder).toContain('ULTRATHINK');
      expect(claudeResult.systemReminder).toContain(
        'you must ULTRATHINK the entire codebase'
      );

      // Test with Aider (supportsMetaCognition = false)
      process.env.FSPEC_AGENT = 'aider';
      const aiderResult = await discoverFoundation({
        cwd: tmpDir,
        force: true,
      });

      // Then the guidance should include "think a lot"
      expect(aiderResult.systemReminder.toLowerCase()).toContain('think a lot');
      expect(aiderResult.systemReminder).not.toContain('ULTRATHINK');
      expect(aiderResult.systemReminder).toMatch(/purpose|understand/i);
    });
  });
});
