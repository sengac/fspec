/**
 * Feature: spec/features/duplicate-next-steps-message-in-fspec-init-output.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import { render } from 'ink-testing-library';
import React from 'react';
import { AgentSelector } from '../AgentSelector';
import type { AgentConfig } from '../../utils/agentRegistry';

describe('Feature: Duplicate "Next steps" message in fspec init output', () => {
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
      rootStubFile: 'CLAUDE.md',
      detectionPaths: ['.claude/'],
      available: true,
      category: 'cli',
      popularity: 1,
    },
  ];

  describe('Scenario: Interactive mode shows single "Next steps" message (AFTER FIX)', () => {
    it('should NOT show "Next steps:" in AgentSelector when agent is selected (FIXED BEHAVIOR)', async () => {
      // Given I run 'fspec init' in interactive mode
      // When I select Claude from the agent menu
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

      // Then the AgentSelector component displays a success message WITHOUT 'Next steps:'
      // This is the DESIRED behavior after the fix
      expect(output).toContain('✓ Installed fspec for Claude Code');
      expect(output).not.toContain('Next steps:');
      expect(output).not.toContain('Run /fspec in Claude Code to activate');
    });
  });
});
