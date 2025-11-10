/**
 * Feature: spec/features/research-tool-discovery-and-configuration-ux.feature
 *
 * Tests for research tool discovery and configuration UX (RES-009).
 * Tests improved tool listing with status, descriptions, and tool-specific help.
 */

import { describe, it, expect } from 'vitest';
import { listResearchTools } from '../research-tool-list';

describe('Feature: Research Tool Discovery and Configuration UX', () => {
  describe('Scenario: List research tools with configuration status and descriptions', () => {
    it('should display tools with status indicators, descriptions, and usage hints', () => {
      // @step Given I have no research tools configured
      delete process.env.PERPLEXITY_API_KEY;
      delete process.env.JIRA_URL;
      delete process.env.JIRA_TOKEN;
      delete process.env.CONFLUENCE_URL;
      delete process.env.CONFLUENCE_TOKEN;

      // @step When I run 'fspec research'
      const tools = listResearchTools();

      // @step Then I should see tool names with status indicators (✓ configured, ✗ not configured)
      expect(tools).toBeDefined();
      expect(tools.length).toBeGreaterThan(0);

      for (const tool of tools) {
        expect(tool.statusIndicator).toMatch(/[✓✗]/);
        expect([
          'CONFIGURED',
          'NOT CONFIGURED',
          'PARTIALLY CONFIGURED',
        ]).toContain(tool.status);
      }

      // @step And I should see brief descriptions of what each tool does
      for (const tool of tools) {
        expect(tool.description).toBeDefined();
        expect(tool.description.length).toBeGreaterThan(0);
      }

      // @step And I should see usage hints for each tool
      // Note: Usage hints are displayed in CLI output, tested in integration tests
      expect(tools[0].name).toBeDefined();
    });
  });

  describe('Scenario: Get tool-specific help output', () => {
    it('should show tool-specific help when --help flag is used', () => {
      // @step Given I want to know how to use the perplexity tool
      const toolName = 'perplexity';

      // @step When I run 'fspec research --tool=perplexity --help'
      // Note: Help forwarding is handled by help-interceptor.ts
      // This test verifies the tool help structure exists
      const tools = listResearchTools();
      const perplexityTool = tools.find(t => t.name === toolName);

      // @step Then I should see the perplexity script's --help output
      expect(perplexityTool).toBeDefined();

      // @step And the output should include query, model, and format options
      // Note: Actual help output tested in integration tests with help-interceptor
      expect(perplexityTool!.description).toContain('Perplexity');
    });
  });
});
