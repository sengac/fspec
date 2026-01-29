/**
 * Feature: spec/features/unconfigured-research-tool-visibility-and-discovery.feature
 *
 * Tests for 'fspec research' command listing behavior
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { research } from '../research';
import * as fs from 'fs';
import * as path from 'path';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';

describe('Feature: Unconfigured research tool visibility and discovery', () => {
  let setup: TestDirectorySetup;
  let configPath: string;
  let consoleLogSpy: any;
  let consoleErrorSpy: any;

  beforeEach(async () => {
    setup = await setupTestDirectory('research-listing');
    configPath = path.join(setup.testDir, 'spec', 'fspec-config.json');

    // Ensure spec directory exists
    fs.mkdirSync(path.join(setup.testDir, 'spec'), { recursive: true });

    // Clear environment variables that might affect tool configuration
    delete process.env.PERPLEXITY_API_KEY;
    delete process.env.JIRA_URL;
    delete process.env.JIRA_TOKEN;
    delete process.env.CONFLUENCE_URL;
    delete process.env.CONFLUENCE_TOKEN;
    delete process.env.SLACK_WEBHOOK_URL;
    delete process.env.TEAMS_WEBHOOK_URL;
    delete process.env.DISCORD_WEBHOOK_URL;

    // Spy on console output
    consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(async () => {
    consoleLogSpy?.mockRestore();
    consoleErrorSpy?.mockRestore();
  });

  describe('Scenario: Discovery message when listing configured tools', () => {
    it('should show only configured tools with footer about --all', async () => {
      // @step Given only AST is configured
      // AST needs no config, so empty config = AST only
      fs.writeFileSync(configPath, JSON.stringify({}), 'utf-8');

      // @step When I run 'fspec research'
      await research([], { cwd: setup.testDir });

      const output = consoleLogSpy.mock.calls
        .map((call: any) => call[0])
        .join('\n');

      // @step Then I should see AST marked as ready
      expect(output).toContain('✓');
      expect(output).toContain('ast');
      expect(output).toMatch(/ready|Ready to use/i);

      // @step And I should see a footer message stating the number of unconfigured tools
      expect(output).toMatch(/\d+ additional tool/i);

      // @step And the footer should mention using --all to see them
      expect(output).toContain('--all');
    });
  });

  describe('Scenario: List all tools including unconfigured ones', () => {
    it('should show all tools with status and JSON examples when --all flag is used', async () => {
      // @step Given I have no research tools configured except AST
      fs.writeFileSync(configPath, JSON.stringify({}), 'utf-8');

      // @step When I run 'fspec research --all'
      await research([], {
        cwd: setup.testDir,
        all: true,
        userConfigPath: '/nonexistent/user-config.json',
      });

      const output = consoleLogSpy.mock.calls
        .map((call: any) => call[0])
        .join('\n');

      // @step Then I should see all 5 research tools listed
      expect(output).toContain('ast');
      expect(output).toContain('perplexity');
      expect(output).toContain('jira');
      expect(output).toContain('confluence');
      expect(output).toContain('stakeholder');

      // @step And AST should show as configured with ✓ indicator
      expect(output).toMatch(/✓.*ast/);

      // @step And Perplexity should show as not configured with ✗ indicator
      expect(output).toMatch(/✗.*perplexity/);

      // @step And each unconfigured tool should show JSON config example
      expect(output).toContain('"research"');
      expect(output).toContain('"perplexity"');
      expect(output).toContain('"apiKey"');
      expect(output).toContain('spec/fspec-config.json');
    });

    it('should show multiple configured tools when partially configured', async () => {
      // Setup: Configure Perplexity and JIRA
      const config = {
        research: {
          perplexity: {
            apiKey: 'test-key',
          },
          jira: {
            url: 'https://example.atlassian.net',
            token: 'test-token',
          },
        },
      };
      fs.writeFileSync(configPath, JSON.stringify(config), 'utf-8');

      await research([], { cwd: setup.testDir });

      const output = consoleLogSpy.mock.calls
        .map((call: any) => call[0])
        .join('\n');

      // Should show AST, Perplexity, JIRA as configured
      expect(output).toMatch(/✓.*ast/);
      expect(output).toMatch(/✓.*perplexity/);
      expect(output).toMatch(/✓.*jira/);

      // Footer should mention 2 unconfigured tools (confluence, stakeholder)
      expect(output).toMatch(/2 additional tool/i);
    });
  });

  describe('Default listing behavior', () => {
    it('should list only configured tools by default (no --all flag)', async () => {
      fs.writeFileSync(configPath, JSON.stringify({}), 'utf-8');

      await research([], { cwd: setup.testDir });

      const output = consoleLogSpy.mock.calls
        .map((call: any) => call[0])
        .join('\n');

      // Should show AST
      expect(output).toContain('ast');

      // Should NOT show unconfigured tools in main listing
      expect(output).not.toMatch(/✗.*perplexity/);
      expect(output).not.toMatch(/✗.*jira/);

      // Should mention them in footer
      expect(output).toMatch(/additional tool/i);
    });
  });
});
