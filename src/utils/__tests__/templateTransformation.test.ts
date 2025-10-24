/**
 * Feature: spec/features/support-multiple-ai-agents-beyond-claude.feature
 *
 * Tests for Template Transformation (system-reminders, meta-cognition, placeholders)
 */

import { describe, it, expect } from 'vitest';
import type { AgentConfig } from '../agentRegistry';
import {
  stripSystemReminders,
  removeMetaCognitivePrompts,
  replacePlaceholders,
  generateAgentDoc,
} from '../templateGenerator';

describe('Feature: Support multiple AI agents beyond Claude', () => {
  describe('Scenario: Transform system-reminders for Cline', () => {
    it('should remove <system-reminder> tags for agents without support', () => {
      // Given I have a template with <system-reminder> tags
      const template = `
# Documentation

<system-reminder>
CRITICAL: Follow ACDD workflow.
DO NOT skip phases.
</system-reminder>

Some content here.
`;

      const clineAgent: Partial<AgentConfig> = {
        id: 'cline',
        supportsSystemReminders: false,
        category: 'ide',
      };

      // When I generate documentation for Cline
      const result = stripSystemReminders(template, clineAgent as AgentConfig);

      // Then "<system-reminder>" tags should be removed
      expect(result).not.toContain('<system-reminder>');
      expect(result).not.toContain('</system-reminder>');
    });
  });

  describe('Scenario: Transform system-reminders for IDE agents with emoji', () => {
    it('should transform system-reminders to bold with emoji for IDE agents', () => {
      const template = `
<system-reminder>
PREFILL DETECTED in generated feature file.
DO NOT use Write or Edit tools.
ALWAYS use fspec commands.
</system-reminder>
`;

      const cursorAgent: Partial<AgentConfig> = {
        id: 'cursor',
        supportsSystemReminders: false,
        category: 'ide',
      };

      const result = stripSystemReminders(template, cursorAgent as AgentConfig);

      // Then system-reminder content should be transformed to "**⚠️ IMPORTANT:**" blocks
      expect(result).toContain('**⚠️ IMPORTANT:**');
      expect(result).toContain('PREFILL DETECTED');
      expect(result).not.toContain('<system-reminder>');
    });
  });

  describe('Scenario: Transform system-reminders for CLI agents without emoji', () => {
    it('should transform system-reminders to bold without emoji for CLI agents', () => {
      const template = `
<system-reminder>
ACDD VIOLATION: Cannot estimate without feature file.
DO NOT skip phases.
</system-reminder>
`;

      const aiderAgent: Partial<AgentConfig> = {
        id: 'aider',
        supportsSystemReminders: false,
        category: 'cli',
      };

      const result = stripSystemReminders(template, aiderAgent as AgentConfig);

      // Then system-reminder content should be transformed to "**IMPORTANT:**"
      expect(result).toContain('**IMPORTANT:**');
      expect(result).toContain('ACDD VIOLATION');
      // And no emoji should be included
      expect(result).not.toContain('⚠️');
      expect(result).not.toContain('<system-reminder>');
    });
  });

  describe('Scenario: Remove meta-cognitive prompts for CLI agents', () => {
    it('should remove "ultrathink" from CLI agent documentation', () => {
      const template = `
Before proceeding, ultrathink your next steps and deeply consider the implications.
`;

      const aiderAgent: Partial<AgentConfig> = {
        id: 'aider',
        supportsMetaCognition: false,
        category: 'cli',
      };

      const result = removeMetaCognitivePrompts(
        template,
        aiderAgent as AgentConfig
      );

      // Then the file should not contain "ultrathink"
      expect(result).not.toContain('ultrathink');
      // And should not contain "deeply consider"
      expect(result).not.toContain('deeply consider');
    });

    it('should remove "take a moment to reflect" from CLI agent documentation', () => {
      const template = `
Take a moment to reflect on your approach before implementing.
`;

      const aiderAgent: Partial<AgentConfig> = {
        id: 'aider',
        supportsMetaCognition: false,
        category: 'cli',
      };

      const result = removeMetaCognitivePrompts(
        template,
        aiderAgent as AgentConfig
      );

      expect(result).not.toContain('take a moment to reflect');
    });

    it('should preserve meta-cognitive prompts for IDE agents', () => {
      const template = `
Before proceeding, ultrathink your next steps.
`;

      const cursorAgent: Partial<AgentConfig> = {
        id: 'cursor',
        supportsMetaCognition: true,
        category: 'ide',
      };

      const result = removeMetaCognitivePrompts(
        template,
        cursorAgent as AgentConfig
      );

      // Meta-cognitive prompts should be preserved for IDE agents
      expect(result).toContain('ultrathink');
    });
  });

  describe('Scenario: Template transformation with placeholders', () => {
    it('should replace {{AGENT_NAME}} placeholder', () => {
      const template = `
# {{AGENT_NAME}} Documentation
Welcome to {{AGENT_NAME}}!
`;

      const cursorAgent: Partial<AgentConfig> = {
        id: 'cursor',
        name: 'Cursor',
        slashCommandPath: '.cursor/commands/',
      };

      const result = replacePlaceholders(template, cursorAgent as AgentConfig);

      // Then "{{AGENT_NAME}}" placeholders should be replaced with "Cursor"
      expect(result).toContain('# Cursor Documentation');
      expect(result).toContain('Welcome to Cursor!');
      expect(result).not.toContain('{{AGENT_NAME}}');
    });

    it('should replace {{SLASH_COMMAND_PATH}} placeholder', () => {
      const template = `
Slash commands are located at {{SLASH_COMMAND_PATH}}.
`;

      const cursorAgent: Partial<AgentConfig> = {
        id: 'cursor',
        name: 'Cursor',
        slashCommandPath: '.cursor/commands/',
      };

      const result = replacePlaceholders(template, cursorAgent as AgentConfig);

      // And "{{SLASH_COMMAND_PATH}}" placeholders should be replaced
      expect(result).toContain('.cursor/commands/');
      expect(result).not.toContain('{{SLASH_COMMAND_PATH}}');
    });

    it('should ensure no unresolved placeholders remain', () => {
      const template = `
# {{AGENT_NAME}} - {{AGENT_ID}}
Path: {{SLASH_COMMAND_PATH}}
`;

      const cursorAgent: Partial<AgentConfig> = {
        id: 'cursor',
        name: 'Cursor',
        slashCommandPath: '.cursor/commands/',
      };

      const result = replacePlaceholders(template, cursorAgent as AgentConfig);

      // Then no unresolved placeholders should remain
      expect(result).not.toMatch(/\{\{[A-Z_]+\}\}/);
    });
  });

  describe('Full template generation', () => {
    it('should generate comprehensive documentation for Aider (CLI agent)', async () => {
      const aiderAgent: AgentConfig = {
        id: 'aider',
        name: 'Aider',
        description: 'CLI agent',
        slashCommandPath: '.aider/',
        slashCommandFormat: 'markdown',
        supportsSystemReminders: false,
        supportsMetaCognition: false,
        docTemplate: 'AIDER.md',
        detectionPaths: [],
        available: true,
        category: 'cli',
      };

      const result = await generateAgentDoc(aiderAgent);

      // Should generate comprehensive Project Management Guidelines (not a stub)
      const lineCount = result.split('\n').length;
      expect(lineCount).toBeGreaterThan(2000);
      // Should transform ALL system-reminders (including in code examples)
      // so Aider sees what error messages would look like FOR AIDER
      expect(result).not.toContain('<system-reminder>');
      // Should not contain meta-cognitive prompts
      expect(result).not.toContain('ultrathink');
      // Should not contain unresolved placeholders
      expect(result).not.toContain('{{AGENT_NAME}}');
      expect(result).not.toContain('{{SLASH_COMMAND_PATH}}');
    });
  });

  describe('Scenario: Nested system-reminder transformation', () => {
    it('should handle nested system-reminder tags correctly', () => {
      // Given a template contains nested system-reminder tags
      const template = `
<system-reminder>
OUTER REMINDER
<system-reminder>
INNER REMINDER
DO NOT skip this.
</system-reminder>
ALWAYS follow both.
</system-reminder>
`;

      const cursorAgent: Partial<AgentConfig> = {
        id: 'cursor',
        supportsSystemReminders: false,
        category: 'ide',
      };

      // When the template generator transforms the content for a non-Claude agent
      const result = stripSystemReminders(template, cursorAgent as AgentConfig);

      // Then both outer and inner system-reminders should be transformed
      expect(result).toContain('**⚠️ IMPORTANT:**');
      expect(result).toContain('OUTER REMINDER');
      expect(result).toContain('INNER REMINDER');

      // And the content structure should remain intact
      expect(result).toContain('**ALWAYS:** follow both.');
      expect(result).toContain('**DO NOT:** skip this.');

      // And no closing tags should be left unmatched
      expect(result).not.toContain('<system-reminder>');
      expect(result).not.toContain('</system-reminder>');
    });
  });

  describe('Scenario: Safe meta-cognitive prompt removal', () => {
    it('should preserve words containing "think" when removing meta-cognitive prompts', () => {
      // Given a template contains the words "rethinking" and "thinking"
      // And the template contains "ultrathink your next steps"
      const template = `
I am rethinking my approach to this problem.
Critical thinking is important.
Before proceeding, ultrathink your next steps.
Deeply consider the implications.
`;

      const aiderAgent: Partial<AgentConfig> = {
        id: 'aider',
        supportsMetaCognition: false,
        category: 'cli',
      };

      // When the template generator removes meta-cognitive prompts
      const result = removeMetaCognitivePrompts(
        template,
        aiderAgent as AgentConfig
      );

      // Then "ultrathink your next steps" should be removed
      expect(result).not.toContain('ultrathink your next steps');

      // And "rethinking" should be preserved
      expect(result).toContain('rethinking');

      // And "thinking" should be preserved
      expect(result).toContain('thinking');

      // And "deeply consider" should be removed
      expect(result).not.toContain('deeply consider');
    });
  });
});
