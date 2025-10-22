/**
 * Feature: spec/features/agent-specific-activation-message-not-customized-in-fspec-init-success-output.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import { getAgentById } from '../agentRegistry';
import { getActivationMessage } from '../activationMessage';

describe('Feature: Agent-specific activation message', () => {
  describe('Scenario: Claude Code user sees agent-specific activation message', () => {
    it('should return Claude-specific activation message', () => {
      // Given Claude Code is the detected agent
      const agent = getAgentById('claude')!;

      // When generating activation message
      const message = getActivationMessage(agent);

      // Then the message should be Claude-specific
      expect(message).toContain('Run /fspec in Claude Code to activate');
    });
  });

  describe('Scenario: Cursor user sees IDE-specific activation instructions', () => {
    it('should return Cursor-specific activation message', () => {
      // Given Cursor is the detected agent
      const agent = getAgentById('cursor')!;

      // When generating activation message
      const message = getActivationMessage(agent);

      // Then the message should reference Cursor
      expect(message).toContain('Cursor');
      expect(message).toContain('.cursor/commands/');
    });
  });

  describe('Scenario: Aider user sees CLI-specific activation instructions', () => {
    it('should return Aider-specific activation message', () => {
      // Given Aider is the detected agent
      const agent = getAgentById('aider')!;

      // When generating activation message
      const message = getActivationMessage(agent);

      // Then the message should reference Aider configuration
      expect(message).toContain('Aider');
      expect(message).toContain('.aider');
    });
  });

  describe('Scenario: Unknown agent receives generic fallback message', () => {
    it('should return generic fallback message for unknown agents', () => {
      // Given an unknown agent (safe default)
      const unknownAgent = {
        id: 'unknown',
        name: 'Unknown Agent',
        category: 'cli' as const,
        slashCommandPath: '',
        supportsSystemReminders: false,
        supportsMetaCognition: false,
      };

      // When generating activation message
      const message = getActivationMessage(unknownAgent as any);

      // Then the message should be generic fallback
      expect(message).toContain('Refer to your AI agent documentation to activate fspec');
    });
  });
});

// Helper function is now imported from ../activationMessage
