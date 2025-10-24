/**
 * Feature: spec/features/duplicate-installation-success-message-in-fspec-init.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import { render } from 'ink-testing-library';
import React from 'react';
import { AgentSelector } from '../AgentSelector';
import type { AgentConfig } from '../../utils/agentRegistry';

describe('Feature: Duplicate installation success message in fspec init', () => {
  const mockAgents: AgentConfig[] = [
    {
      id: 'claude',
      name: 'Claude Code',
      description: 'Anthropic CLI',
      slashCommandPath: '.claude/commands/',
      slashCommandFormat: 'markdown',
      supportsSystemReminders: true,
      supportsMetaCognition: true,
      docTemplate: 'CLAUDE.md',
      detectionPaths: ['.claude/'],
      available: true,
      category: 'cli',
      popularity: 1,
    },
  ];

  describe('Scenario: Initialize fspec for Claude Code without duplicate message', () => {
    it('should NOT display duplicate success message from AgentSelector component', async () => {
      // Given: I am in a project directory without fspec initialized
      // When: I run 'fspec init' and select Claude Code (simulated by pressing Enter)
      const { lastFrame, stdin } = render(
        React.createElement(AgentSelector, {
          agents: mockAgents,
          preSelected: [],
          onSubmit: () => {
            // Selection submitted
          },
        })
      );

      // Simulate Enter key to select agent
      stdin.write('\r');

      // Give component time to update
      await new Promise((resolve) => {
        setTimeout(resolve, 150);
      });

      const output = lastFrame();

      // Then: The AgentSelector should NOT display any success message
      // The success message should ONLY come from init.ts action handler
      // This prevents duplicate messages (one from component, one from action handler)
      expect(output).not.toContain('✓ Installed fspec for');
      expect(output).not.toContain('✓ Installed fspec for Claude Code');

      // The component should exit silently after submitting the selection
      // allowing the action handler to display the comprehensive success message with file list
    });
  });
});
