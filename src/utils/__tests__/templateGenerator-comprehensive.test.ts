/**
 * Feature: spec/features/spec-agent-md-files-contain-stub-instead-of-comprehensive-workflow.feature
 *
 * Tests that spec/AGENT.md files contain comprehensive Project Management Guidelines
 * (not slash command content or 17-line stubs)
 */

import { describe, it, expect } from 'vitest';
import type { AgentConfig } from '../agentRegistry';
import { generateAgentDoc } from '../templateGenerator';

describe('Feature: spec/AGENT.md files contain stub instead of comprehensive workflow', () => {
  describe('Scenario: spec/AGENT.md contains comprehensive Project Management Guidelines', () => {
    it('should generate spec/CLAUDE.md with approximately 2069 lines', async () => {
      // Given generateAgentDoc() is modified to use getProjectManagementTemplate() as base
      const claudeAgent: AgentConfig = {
        id: 'claude',
        name: 'Claude Code',
        description: 'Official CLI for Claude',
        slashCommandPath: '.claude/commands/',
        slashCommandFormat: 'markdown',
        supportsSystemReminders: true,
        supportsMetaCognition: true,
        docTemplate: 'CLAUDE.md',
        rootStubFile: 'CLAUDE.md',
        detectionPaths: ['.claude/', '.claude/commands/'],
        available: true,
        category: 'ide',
      };

      // When fspec init --agent=claude is run (generating spec/CLAUDE.md)
      const result = await generateAgentDoc(claudeAgent);

      // Then spec/CLAUDE.md should contain approximately 2069 lines
      const lineCount = result.split('\n').length;
      expect(lineCount).toBeGreaterThan(2000); // Allow some variance
      expect(lineCount).toBeLessThan(2200);
    });

    it('should be titled "Project Management and Specification Guidelines for fspec"', async () => {
      const claudeAgent: AgentConfig = {
        id: 'claude',
        name: 'Claude Code',
        description: 'Official CLI for Claude',
        slashCommandPath: '.claude/commands/',
        slashCommandFormat: 'markdown',
        supportsSystemReminders: true,
        supportsMetaCognition: true,
        docTemplate: 'CLAUDE.md',
        rootStubFile: 'CLAUDE.md',
        detectionPaths: [],
        available: true,
        category: 'ide',
      };

      const result = await generateAgentDoc(claudeAgent);

      // And spec/CLAUDE.md should be titled "Project Management and Specification Guidelines for fspec"
      expect(result).toContain(
        '# Project Management and Specification Guidelines for fspec'
      );
    });

    it('should include work unit management documentation', async () => {
      const claudeAgent: AgentConfig = {
        id: 'claude',
        name: 'Claude Code',
        description: 'Official CLI for Claude',
        slashCommandPath: '.claude/commands/',
        slashCommandFormat: 'markdown',
        supportsSystemReminders: true,
        supportsMetaCognition: true,
        docTemplate: 'CLAUDE.md',
        rootStubFile: 'CLAUDE.md',
        detectionPaths: [],
        available: true,
        category: 'ide',
      };

      const result = await generateAgentDoc(claudeAgent);

      // And spec/CLAUDE.md should include work unit management documentation
      expect(result).toContain('## Project Management Workflow');
      expect(result).toContain('### Understanding Work Organization');
      expect(result).toContain('Work Units');
      expect(result).toContain('Kanban States');
    });

    it('should include Reverse ACDD documentation', async () => {
      const claudeAgent: AgentConfig = {
        id: 'claude',
        name: 'Claude Code',
        description: 'Official CLI for Claude',
        slashCommandPath: '.claude/commands/',
        slashCommandFormat: 'markdown',
        supportsSystemReminders: true,
        supportsMetaCognition: true,
        docTemplate: 'CLAUDE.md',
        rootStubFile: 'CLAUDE.md',
        detectionPaths: [],
        available: true,
        category: 'ide',
      };

      const result = await generateAgentDoc(claudeAgent);

      // And spec/CLAUDE.md should include Reverse ACDD documentation
      expect(result).toContain('## Reverse ACDD for Existing Codebases');
      expect(result).toContain('fspec reverse');
    });

    it('should include coverage tracking system documentation', async () => {
      const claudeAgent: AgentConfig = {
        id: 'claude',
        name: 'Claude Code',
        description: 'Official CLI for Claude',
        slashCommandPath: '.claude/commands/',
        slashCommandFormat: 'markdown',
        supportsSystemReminders: true,
        supportsMetaCognition: true,
        docTemplate: 'CLAUDE.md',
        rootStubFile: 'CLAUDE.md',
        detectionPaths: [],
        available: true,
        category: 'ide',
      };

      const result = await generateAgentDoc(claudeAgent);

      // And spec/CLAUDE.md should include coverage tracking system documentation
      expect(result).toContain(
        '## Coverage Tracking: Linking Specs, Tests, and Implementation'
      );
      expect(result).toContain('fspec link-coverage');
      expect(result).toContain('.feature.coverage');
    });

    it('should include lifecycle hooks documentation', async () => {
      const claudeAgent: AgentConfig = {
        id: 'claude',
        name: 'Claude Code',
        description: 'Official CLI for Claude',
        slashCommandPath: '.claude/commands/',
        slashCommandFormat: 'markdown',
        supportsSystemReminders: true,
        supportsMetaCognition: true,
        docTemplate: 'CLAUDE.md',
        rootStubFile: 'CLAUDE.md',
        detectionPaths: [],
        available: true,
        category: 'ide',
      };

      const result = await generateAgentDoc(claudeAgent);

      // And spec/CLAUDE.md should include lifecycle hooks documentation
      expect(result).toContain('## Lifecycle Hooks for Workflow Automation');
      expect(result).toContain('fspec-hooks.json');
    });

    it('should include git checkpoints documentation', async () => {
      const claudeAgent: AgentConfig = {
        id: 'claude',
        name: 'Claude Code',
        description: 'Official CLI for Claude',
        slashCommandPath: '.claude/commands/',
        slashCommandFormat: 'markdown',
        supportsSystemReminders: true,
        supportsMetaCognition: true,
        docTemplate: 'CLAUDE.md',
        rootStubFile: 'CLAUDE.md',
        detectionPaths: [],
        available: true,
        category: 'ide',
      };

      const result = await generateAgentDoc(claudeAgent);

      // And spec/CLAUDE.md should include git checkpoints documentation
      expect(result).toContain('## Git Checkpoints for Safe Experimentation');
      expect(result).toContain('fspec checkpoint');
    });
  });

  describe('Scenario: Agent-specific transformations are applied to Project Management template', () => {
    it('should transform system-reminder tags in ALL contexts (including code examples)', async () => {
      // Given generateAgentDoc() uses getProjectManagementTemplate() as base content
      const cursorAgent: AgentConfig = {
        id: 'cursor',
        name: 'Cursor',
        description: 'AI-first code editor',
        slashCommandPath: '.cursor/commands/',
        slashCommandFormat: 'markdown',
        supportsSystemReminders: false,
        supportsMetaCognition: true,
        docTemplate: 'CURSOR.md',
        rootStubFile: 'CURSOR.md',
        detectionPaths: ['.cursor/', '.cursor/commands/'],
        available: true,
        category: 'ide',
      };

      // When generating spec/CURSOR.md for Cursor agent
      const result = await generateAgentDoc(cursorAgent);

      // Then ALL system-reminder tags should be transformed (including in code examples)
      // so Cursor sees what error messages would look like FOR CURSOR
      expect(result).not.toContain('<system-reminder>');
      expect(result).not.toContain('</system-reminder>');
      // IDE agents get emoji in transformations
      if (result.includes('IMPORTANT')) {
        expect(result).toContain('**⚠️ IMPORTANT:**');
      }
    });

    it('should replace agent name placeholders with "Cursor"', async () => {
      const cursorAgent: AgentConfig = {
        id: 'cursor',
        name: 'Cursor',
        description: 'AI-first code editor',
        slashCommandPath: '.cursor/commands/',
        slashCommandFormat: 'markdown',
        supportsSystemReminders: false,
        supportsMetaCognition: true,
        docTemplate: 'CURSOR.md',
        rootStubFile: 'CURSOR.md',
        detectionPaths: [],
        available: true,
        category: 'ide',
      };

      const result = await generateAgentDoc(cursorAgent);

      // And agent name placeholders should be replaced with "Cursor"
      expect(result).not.toContain('{{AGENT_NAME}}');
      // Note: "Claude Code" may appear in research references (factual links), which is acceptable
      expect(result).toContain('Cursor'); // Should have agent-specific name
    });

    it('should not contain unresolved placeholders', async () => {
      const cursorAgent: AgentConfig = {
        id: 'cursor',
        name: 'Cursor',
        description: 'AI-first code editor',
        slashCommandPath: '.cursor/commands/',
        slashCommandFormat: 'markdown',
        supportsSystemReminders: false,
        supportsMetaCognition: true,
        docTemplate: 'CURSOR.md',
        rootStubFile: 'CURSOR.md',
        detectionPaths: [],
        available: true,
        category: 'ide',
      };

      const result = await generateAgentDoc(cursorAgent);

      // And no unresolved placeholders should remain
      expect(result).not.toContain('{{AGENT_NAME}}');
      expect(result).not.toContain('{{SLASH_COMMAND_PATH}}');
      expect(result).not.toContain('{{DOC_TEMPLATE}}');
      // Note: This template is project management guidelines, not slash command docs
      // So slash command paths are not expected to appear
    });

    it('should contain the full Project Management Guidelines with transformations applied', async () => {
      const cursorAgent: AgentConfig = {
        id: 'cursor',
        name: 'Cursor',
        description: 'AI-first code editor',
        slashCommandPath: '.cursor/commands/',
        slashCommandFormat: 'markdown',
        supportsSystemReminders: false,
        supportsMetaCognition: true,
        docTemplate: 'CURSOR.md',
        rootStubFile: 'CURSOR.md',
        detectionPaths: [],
        available: true,
        category: 'ide',
      };

      const result = await generateAgentDoc(cursorAgent);

      // And the file should contain the full Project Management Guidelines with transformations applied
      const lineCount = result.split('\n').length;
      expect(lineCount).toBeGreaterThan(2000); // Should be comprehensive, not a stub
      expect(result).toContain(
        'Project Management and Specification Guidelines'
      );
    });
  });

  describe('Scenario: Fix eliminates circular reference in spec/CLAUDE.md', () => {
    it('should not contain circular reference "See spec/CLAUDE.md"', async () => {
      // Given the current spec/CLAUDE.md has 17 lines with circular reference
      // When generateAgentDoc() is fixed to use getProjectManagementTemplate()
      const claudeAgent: AgentConfig = {
        id: 'claude',
        name: 'Claude Code',
        description: 'Official CLI for Claude',
        slashCommandPath: '.claude/commands/',
        slashCommandFormat: 'markdown',
        supportsSystemReminders: true,
        supportsMetaCognition: true,
        docTemplate: 'CLAUDE.md',
        rootStubFile: 'CLAUDE.md',
        detectionPaths: [],
        available: true,
        category: 'ide',
      };

      // And fspec init --agent=claude is run
      const result = await generateAgentDoc(claudeAgent);

      // Then spec/CLAUDE.md should not contain the text "See spec/CLAUDE.md"
      expect(result).not.toContain('See [spec/CLAUDE.md](spec/CLAUDE.md)');
      expect(result).not.toContain('See spec/CLAUDE.md');
    });

    it('should contain comprehensive standalone Project Management Guidelines', async () => {
      const claudeAgent: AgentConfig = {
        id: 'claude',
        name: 'Claude Code',
        description: 'Official CLI for Claude',
        slashCommandPath: '.claude/commands/',
        slashCommandFormat: 'markdown',
        supportsSystemReminders: true,
        supportsMetaCognition: true,
        docTemplate: 'CLAUDE.md',
        rootStubFile: 'CLAUDE.md',
        detectionPaths: [],
        available: true,
        category: 'ide',
      };

      const result = await generateAgentDoc(claudeAgent);

      // And spec/CLAUDE.md should contain comprehensive standalone Project Management Guidelines
      expect(result).toContain('Project Management Workflow');
      expect(result).toContain('Reverse ACDD');
      expect(result).toContain('Coverage Tracking');
      expect(result).toContain('Lifecycle Hooks');
      expect(result).toContain('Git Checkpoints');
    });

    it('should have approximately 2069 lines', async () => {
      const claudeAgent: AgentConfig = {
        id: 'claude',
        name: 'Claude Code',
        description: 'Official CLI for Claude',
        slashCommandPath: '.claude/commands/',
        slashCommandFormat: 'markdown',
        supportsSystemReminders: true,
        supportsMetaCognition: true,
        docTemplate: 'CLAUDE.md',
        rootStubFile: 'CLAUDE.md',
        detectionPaths: [],
        available: true,
        category: 'ide',
      };

      const result = await generateAgentDoc(claudeAgent);

      // And spec/CLAUDE.md should have approximately 2069 lines
      const lineCount = result.split('\n').length;
      expect(lineCount).toBeGreaterThan(2000);
      expect(lineCount).toBeLessThan(2200);
    });
  });
});
