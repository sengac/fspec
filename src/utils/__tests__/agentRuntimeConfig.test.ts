/**
 * Feature: spec/features/agent-runtime-detection-for-context-aware-cli-output.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
  existsSync,
  mkdirSync,
  writeFileSync,
  unlinkSync,
  rmdirSync,
  readFileSync,
} from 'fs';
import { join } from 'path';
import {
  getAgentConfig,
  writeAgentConfig,
  formatAgentOutput,
} from '../agentRuntimeConfig';

describe('Feature: Agent runtime detection for context-aware CLI output', () => {
  const testDir = join(__dirname, '__test-runtime-config__');
  const configPath = join(testDir, 'spec', 'fspec-config.json');
  let originalEnv: string | undefined;

  beforeEach(() => {
    // Save original FSPEC_AGENT env var
    originalEnv = process.env.FSPEC_AGENT;

    // Create test directory structure
    if (!existsSync(testDir)) {
      mkdirSync(testDir, { recursive: true });
    }
    const specDir = join(testDir, 'spec');
    if (!existsSync(specDir)) {
      mkdirSync(specDir);
    }
  });

  afterEach(() => {
    // Restore original env var
    if (originalEnv !== undefined) {
      process.env.FSPEC_AGENT = originalEnv;
    } else {
      delete process.env.FSPEC_AGENT;
    }

    // Clean up test files
    try {
      if (existsSync(configPath)) {
        unlinkSync(configPath);
      }
      if (existsSync(join(testDir, 'spec'))) {
        rmdirSync(join(testDir, 'spec'), { recursive: true });
      }
      if (existsSync(testDir)) {
        rmdirSync(testDir, { recursive: true });
      }
    } catch (err) {
      // Ignore cleanup errors
    }
  });

  describe('Scenario: FSPEC_AGENT environment variable overrides config file', () => {
    it('should use env var agent even when config file specifies different agent', () => {
      // Given a project with spec/fspec-config.json containing {"agent": "cursor"}
      writeFileSync(configPath, JSON.stringify({ agent: 'cursor' }));

      // And FSPEC_AGENT environment variable is set to "claude"
      process.env.FSPEC_AGENT = 'claude';

      // When I run "fspec validate"
      // (This will be tested via getAgentConfig() or similar function)
      const detectedAgent = getAgentConfig(testDir);

      // Then the output should use Claude Code format with <system-reminder> tags
      expect(detectedAgent.id).toBe('claude');
      expect(detectedAgent.supportsSystemReminders).toBe(true);

      // And the config file agent setting should be ignored
      expect(detectedAgent.id).not.toBe('cursor');
    });
  });

  describe('Scenario: Agent detected during init and written to config file', () => {
    it('should write detected agent to spec/fspec-config.json during init', () => {
      // Given I run "fspec init" in a new project
      // And Claude Code is the detected agent
      const detectedAgent = 'claude';

      // When fspec init completes
      writeAgentConfig(testDir, detectedAgent);

      // Then spec/fspec-config.json should be created
      expect(existsSync(configPath)).toBe(true);

      // And it should contain {"agent": "claude"}
      const config = JSON.parse(readFileSync(configPath, 'utf-8'));
      expect(config.agent).toBe('claude');

      // And subsequent commands should read this config for output formatting
      const loadedAgent = getAgentConfig(testDir);
      expect(loadedAgent.id).toBe('claude');
    });
  });

  describe('Scenario: Claude Code agent receives system-reminder output', () => {
    it('should format output with system-reminder tags for Claude', () => {
      // Given spec/fspec-config.json contains {"agent": "claude"}
      writeFileSync(configPath, JSON.stringify({ agent: 'claude' }));

      // And FSPEC_AGENT is not set
      delete process.env.FSPEC_AGENT;

      // When I run "fspec update-work-unit-status WORK-001 testing"
      const agent = getAgentConfig(testDir);
      const output = formatAgentOutput(
        agent,
        'IMPORTANT: Complete testing phase first'
      );

      // Then the output should contain <system-reminder> tags
      expect(output).toContain('<system-reminder>');
      expect(output).toContain('</system-reminder>');

      // And guidance text should be wrapped in system-reminder format
      expect(output).toContain('IMPORTANT: Complete testing phase first');
    });
  });

  describe('Scenario: Cursor IDE agent receives bold text with emoji output', () => {
    it('should format output with bold text and emoji for Cursor', () => {
      // Given spec/fspec-config.json contains {"agent": "cursor"}
      writeFileSync(configPath, JSON.stringify({ agent: 'cursor' }));

      // And FSPEC_AGENT is not set
      delete process.env.FSPEC_AGENT;

      // When I run "fspec update-work-unit-status WORK-001 testing"
      const agent = getAgentConfig(testDir);
      const output = formatAgentOutput(
        agent,
        'IMPORTANT: Complete testing phase first'
      );

      // Then the output should contain "**⚠️ IMPORTANT:**"
      expect(output).toContain('**⚠️ IMPORTANT:**');

      // And no <system-reminder> tags should appear
      expect(output).not.toContain('<system-reminder>');
      expect(output).not.toContain('</system-reminder>');
    });
  });

  describe('Scenario: Aider CLI agent receives plain text output', () => {
    it('should format output with plain text for Aider', () => {
      // Given spec/fspec-config.json contains {"agent": "aider"}
      writeFileSync(configPath, JSON.stringify({ agent: 'aider' }));

      // And FSPEC_AGENT is not set
      delete process.env.FSPEC_AGENT;

      // When I run "fspec update-work-unit-status WORK-001 testing"
      const agent = getAgentConfig(testDir);
      const output = formatAgentOutput(
        agent,
        'IMPORTANT: Complete testing phase first'
      );

      // Then the output should contain "**IMPORTANT:**"
      expect(output).toContain('**IMPORTANT:**');

      // And no emoji should appear in the output
      expect(output).not.toContain('⚠️');
      expect(output).not.toContain('✅');
      expect(output).not.toContain('❌');

      // And no <system-reminder> tags should appear
      expect(output).not.toContain('<system-reminder>');
    });
  });

  describe('Scenario: Fallback to safe default when no config exists', () => {
    it('should use plain text format when no config file exists', () => {
      // Given spec/fspec-config.json does not exist
      // (Already ensured by not creating it)

      // And FSPEC_AGENT is not set
      delete process.env.FSPEC_AGENT;

      // When I run "fspec validate"
      const agent = getAgentConfig(testDir);
      const output = formatAgentOutput(
        agent,
        'IMPORTANT: Fix validation errors'
      );

      // Then the output should use plain text format
      expect(output).toContain('**IMPORTANT:**');

      // And no <system-reminder> tags should appear
      expect(output).not.toContain('<system-reminder>');

      // And no emoji should appear in the output
      expect(output).not.toContain('⚠️');
    });
  });
});

// Helper functions are now imported from ../agentRuntimeConfig
