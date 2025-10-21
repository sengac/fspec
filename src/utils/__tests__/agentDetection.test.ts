/**
 * Feature: spec/features/support-multiple-ai-agents-beyond-claude.feature
 *
 * Tests for Agent Auto-Detection
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdirSync, rmSync, existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { detectAgents, hasAnyAgentInstalled } from '../agentDetection';

describe('Feature: Support multiple AI agents beyond Claude', () => {
  let testDir: string;

  beforeEach(() => {
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    mkdirSync(testDir, { recursive: true });
  });

  afterEach(() => {
    if (existsSync(testDir)) {
      rmSync(testDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Interactive mode with auto-detection', () => {
    it('should detect Cursor when .cursor/ directory exists', () => {
      // Given I am in a project directory
      // And a directory ".cursor/" exists
      mkdirSync(join(testDir, '.cursor'), { recursive: true });

      // When I check for installed agents
      const detected = detectAgents(testDir);

      // Then "Cursor" should be detected
      expect(detected).toHaveLength(1);
      expect(detected[0].agent.id).toBe('cursor');
      expect(detected[0].agent.name).toBe('Cursor');
      expect(detected[0].detectedPath).toContain('.cursor');
    });

    it('should detect Claude Code when .claude/ directory exists', () => {
      mkdirSync(join(testDir, '.claude'), { recursive: true });

      const detected = detectAgents(testDir);

      expect(detected).toHaveLength(1);
      expect(detected[0].agent.id).toBe('claude');
    });

    it('should detect multiple agents when multiple directories exist', () => {
      mkdirSync(join(testDir, '.cursor'), { recursive: true });
      mkdirSync(join(testDir, '.claude'), { recursive: true });

      const detected = detectAgents(testDir);

      expect(detected.length).toBeGreaterThanOrEqual(2);
      const agentIds = detected.map(d => d.agent.id);
      expect(agentIds).toContain('cursor');
      expect(agentIds).toContain('claude');
    });

    it('should return empty array when no agent directories exist', () => {
      const detected = detectAgents(testDir);

      expect(detected).toHaveLength(0);
    });

    it('should detect agent by .claude/commands/ path', () => {
      mkdirSync(join(testDir, '.claude', 'commands'), { recursive: true });

      const detected = detectAgents(testDir);

      expect(detected).toHaveLength(1);
      expect(detected[0].agent.id).toBe('claude');
    });

    it('should return true when any agent is installed', () => {
      mkdirSync(join(testDir, '.cursor'), { recursive: true });

      const hasAgent = hasAnyAgentInstalled(testDir);

      expect(hasAgent).toBe(true);
    });

    it('should return false when no agents are installed', () => {
      const hasAgent = hasAnyAgentInstalled(testDir);

      expect(hasAgent).toBe(false);
    });
  });

  describe('Detection path validation', () => {
    it('should only detect once per agent even with multiple matching paths', () => {
      // Create both .claude/ and .claude/commands/
      mkdirSync(join(testDir, '.claude', 'commands'), { recursive: true });

      const detected = detectAgents(testDir);

      // Should only detect Claude once
      const claudeDetections = detected.filter(d => d.agent.id === 'claude');
      expect(claudeDetections).toHaveLength(1);
    });
  });
});
