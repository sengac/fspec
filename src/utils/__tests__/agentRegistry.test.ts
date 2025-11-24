/**
 * Feature: spec/features/support-multiple-ai-agents-beyond-claude.feature
 *
 * Tests for Agent Registry and Configuration
 */

import { describe, it, expect } from 'vitest';
import type { AgentConfig } from '../agentRegistry';
import {
  getAgentById,
  getAvailableAgents,
  AGENT_REGISTRY,
} from '../agentRegistry';

describe('Feature: Support multiple AI agents beyond Claude', () => {
  describe('Scenario: Agent registry with all capabilities', () => {
    it('should have configuration for Claude with correct fields', () => {
      // Given the agent registry is loaded
      // When I query the configuration for "claude"
      const claudeConfig = getAgentById('claude');

      // Then it should have all required fields
      expect(claudeConfig).toBeDefined();
      expect(claudeConfig?.id).toBe('claude');
      expect(claudeConfig?.name).toBe('Claude Code');
      expect(claudeConfig?.supportsSystemReminders).toBe(true);
      expect(claudeConfig?.supportsMetaCognition).toBe(true);
      expect(claudeConfig?.category).toBe('cli');
      expect(claudeConfig?.slashCommandFormat).toBe('markdown');
    });

    it('should have configuration for all 19 agents', () => {
      // Given the agent registry is loaded
      const agents = getAvailableAgents();

      // Then it should contain 19 agents
      expect(agents.length).toBe(19);
    });

    it('should have Cursor configuration with correct fields', () => {
      const cursorConfig = getAgentById('cursor');

      expect(cursorConfig).toBeDefined();
      expect(cursorConfig?.id).toBe('cursor');
      expect(cursorConfig?.name).toBe('Cursor');
      expect(cursorConfig?.supportsSystemReminders).toBe(false);
      expect(cursorConfig?.supportsMetaCognition).toBe(false);
      expect(cursorConfig?.category).toBe('ide');
      expect(cursorConfig?.slashCommandFormat).toBe('markdown');
      expect(cursorConfig?.slashCommandPath).toBe('.cursor/commands/');
    });

    it('should have Aider configuration with CLI category and no meta-cognition', () => {
      const aiderConfig = getAgentById('aider');

      expect(aiderConfig).toBeDefined();
      expect(aiderConfig?.id).toBe('aider');
      expect(aiderConfig?.name).toBe('Aider');
      expect(aiderConfig?.supportsSystemReminders).toBe(false);
      expect(aiderConfig?.supportsMetaCognition).toBe(false);
      expect(aiderConfig?.category).toBe('cli');
    });

    it('should have Gemini CLI configuration with TOML format', () => {
      const geminiConfig = getAgentById('gemini');

      expect(geminiConfig).toBeDefined();
      expect(geminiConfig?.slashCommandFormat).toBe('toml');
      expect(geminiConfig?.slashCommandPath).toBe('.gemini/commands/');
    });

    it('should have detection paths for each agent', () => {
      const claudeConfig = getAgentById('claude');
      const cursorConfig = getAgentById('cursor');

      expect(claudeConfig?.detectionPaths).toContain('.claude/');
      expect(cursorConfig?.detectionPaths).toContain('.cursor/');
    });

    it('should have doc template for each agent', () => {
      const agents = getAvailableAgents();

      agents.forEach(agent => {
        expect(agent.docTemplate).toBeDefined();
        expect(typeof agent.docTemplate).toBe('string');
      });
    });

    it('should include Codex in the registry', () => {
      const codexConfig = getAgentById('codex');

      expect(codexConfig).toBeDefined();
      expect(codexConfig?.name).toContain('Codex');
    });
  });

  describe('AgentConfig interface validation', () => {
    it('should validate all required fields exist', () => {
      const agents = getAvailableAgents();

      agents.forEach(agent => {
        expect(agent).toHaveProperty('id');
        expect(agent).toHaveProperty('name');
        expect(agent).toHaveProperty('description');
        expect(agent).toHaveProperty('slashCommandPath');
        expect(agent).toHaveProperty('slashCommandFormat');
        expect(agent).toHaveProperty('supportsSystemReminders');
        expect(agent).toHaveProperty('supportsMetaCognition');
        expect(agent).toHaveProperty('docTemplate');
        expect(agent).toHaveProperty('detectionPaths');
        expect(agent).toHaveProperty('available');
        expect(agent).toHaveProperty('category');
      });
    });
  });
});
