/**
 * Feature: spec/features/tool-specific-help-system.feature
 *
 * Tests for tool-specific help forwarding in fspec research command.
 * Ensures --help flag forwards to tool scripts instead of showing generic help.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';

describe('Feature: Tool-Specific Help System', () => {
  const researchScriptsDir = path.join(process.cwd(), 'spec/research-scripts');

  describe('Scenario: Show generic research help when no tool specified', () => {
    it('should show generic research help with tool list', () => {
      // @step Given I am using fspec research command
      const command = 'fspec research';

      // @step When I run 'fspec research --help'
      const result = execSync(`${command} --help`, { encoding: 'utf-8' });

      // @step Then I should see generic research help documentation
      expect(result).toContain('RESEARCH');
      expect(result).toContain('Execute research tools');

      // @step And the output should contain a list of available research tools
      expect(result).toContain('ast');
      expect(result).toContain('perplexity');
      expect(result).toContain('confluence');
      expect(result).toContain('jira');
      expect(result).toContain('stakeholder');
    });
  });

  describe('Scenario: Forward help to tool script when tool specified', () => {
    it('should forward --help to perplexity tool script', () => {
      // @step Given the research tool 'perplexity' exists as an integrated tool
      // Note: perplexity is now an integrated tool, not an external script
      // The tool registry is checked at runtime

      // @step When I run 'fspec research --tool=perplexity --help'
      const result = execSync('fspec research --tool=perplexity --help', {
        encoding: 'utf-8',
      });

      // @step Then I should see Perplexity-specific help documentation
      expect(result).toContain('PERPLEXITY');

      // @step And the output should contain tool-specific options like --query and --model
      expect(result).toContain('--query');
      expect(result).toContain('--model');

      // @step And the output should NOT contain generic research help
      expect(result).not.toContain(
        'Execute research tools to answer questions'
      );
    });
  });

  describe('Scenario: Show error when tool does not exist', () => {
    it('should show error with available tools list', () => {
      // @step Given I am using fspec research command
      const command = 'fspec research';

      // @step When I run 'fspec research --tool=nonexistent --help'
      let error: any;
      try {
        execSync(`${command} --tool=nonexistent --help`, { encoding: 'utf-8' });
      } catch (e) {
        error = e;
      }

      // @step Then I should see an error message indicating the tool was not found
      expect(error).toBeDefined();
      expect(error.stderr || error.stdout).toContain('not found');

      // @step And the output should contain a list of available tools
      const combinedOutput = (error.stderr || '') + (error.stdout || '');
      expect(combinedOutput).toMatch(
        /ast|perplexity|confluence|jira|stakeholder/
      );

      // @step And the command should exit with code 1
      expect(error.status).toBe(1);
    });
  });
});

/**
 * Feature: spec/features/fix-fspec-research-tool-ast-help-command-should-display-tool-specific-help-instead-of-failing.feature
 *
 * BUG-074: Fix research tool help command to use structured help system
 */
describe('Feature: Fix research tool help command (BUG-074)', () => {
  describe('Scenario: Display help for bundled tool with standard sections', () => {
    it('should display structured help for ast tool', () => {
      // @step Given the ast research tool is bundled in src/research-tools/
      // @step When I run 'fspec research --tool=ast --help'
      const result = execSync('fspec research --tool=ast --help', {
        encoding: 'utf-8',
      });

      // @step Then the help output should contain a USAGE section
      expect(result).toContain('USAGE');

      // @step And the help output should contain an OPTIONS section
      expect(result).toContain('OPTIONS');

      // @step And the help output should contain an EXAMPLES section
      expect(result).toContain('EXAMPLES');

      // @step And the help output should contain a WHEN TO USE section
      expect(result).toContain('WHEN TO USE');
    });
  });

  describe('Scenario: Display help for tool requiring configuration', () => {
    it('should show CONFIGURATION section for jira tool', () => {
      // @step Given the jira research tool requires configuration in ~/.fspec/fspec-config.json
      // @step When I run 'fspec research --tool=jira --help'
      const result = execSync('fspec research --tool=jira --help', {
        encoding: 'utf-8',
      });

      // @step Then the help output should contain a CONFIGURATION section
      expect(result).toContain('CONFIGURATION');

      // @step And the CONFIGURATION section should show the required credentials
      expect(result).toMatch(/jiraUrl|username|apiToken/);
    });
  });

  describe('Scenario: Display help for custom tool in spec/research-tools/', () => {
    let customToolPath: string;

    beforeEach(() => {
      // @step Given a custom tool named 'custom' exists in spec/research-tools/custom.js
      customToolPath = path.join(
        process.cwd(),
        'spec/research-tools/custom.js'
      );
      const customToolDir = path.dirname(customToolPath);

      if (!fs.existsSync(customToolDir)) {
        fs.mkdirSync(customToolDir, { recursive: true });
      }

      // @step And the custom tool implements getHelpConfig() method
      fs.writeFileSync(
        customToolPath,
        `
export const tool = {
  name: 'custom',
  description: 'Custom test tool',
  async execute(args) {
    return JSON.stringify({ results: [] });
  },
  getHelpConfig() {
    return {
      name: 'custom',
      description: 'Custom test research tool',
      usage: 'fspec research --tool=custom [options]',
      whenToUse: 'Use for testing custom tools',
      options: [
        { flag: '--query <text>', description: 'Search query' }
      ],
      examples: [
        { command: '--query "test"', description: 'Search for test' }
      ]
    };
  }
};
`,
        'utf8'
      );
    });

    afterEach(() => {
      if (fs.existsSync(customToolPath)) {
        fs.unlinkSync(customToolPath);
      }
    });

    it('should format custom tool help identically to bundled tools', () => {
      // @step When I run 'fspec research --tool=custom --help'
      const result = execSync('fspec research --tool=custom --help', {
        encoding: 'utf-8',
      });

      // @step Then the help output should be formatted identically to bundled tools
      expect(result).toContain('CUSTOM RESEARCH TOOL');

      // @step And the help output should contain all standard sections
      expect(result).toContain('USAGE');
      expect(result).toContain('OPTIONS');
      expect(result).toContain('EXAMPLES');
      expect(result).toContain('WHEN TO USE');
    });
  });

  describe('Scenario: Error when requesting help for nonexistent tool', () => {
    it('should show error with list of available tools', () => {
      // @step Given no research tool named 'nonexistent' exists
      // @step When I run 'fspec research --tool=nonexistent --help'
      let error: any;
      try {
        execSync('fspec research --tool=nonexistent --help', {
          encoding: 'utf-8',
        });
      } catch (e) {
        error = e;
      }

      // @step Then the command should display an error message
      expect(error).toBeDefined();
      const output = error.stderr || error.stdout || '';
      expect(output).toContain('not found');

      // @step And the error should list all available research tools
      expect(output).toMatch(/ast|jira|perplexity|confluence|stakeholder/);

      // @step And the command should exit with code 1
      expect(error.status).toBe(1);
    });
  });

  describe('Scenario: Refactor tool from string-based help to structured config', () => {
    it('should verify ast tool uses getHelpConfig()', async () => {
      // @step Given the ast tool has a help() method returning a hand-crafted string
      // @step When I refactor ast.ts to use getHelpConfig() returning ResearchToolHelpConfig
      const { getResearchTool } = await import('../../research-tools/registry');
      const astTool = await getResearchTool('ast', process.cwd());

      // @step Then the help output should match the previous format
      expect(astTool.getHelpConfig).toBeDefined();
      const config = astTool.getHelpConfig();
      expect(config.name).toBe('ast');

      // @step And the formatting should be handled by formatResearchToolHelp()
      expect(config.usage).toBeDefined();
      expect(config.options).toBeDefined();

      // @step And TypeScript should validate the config structure at compile time
      // (Validated at compile time by TypeScript)
      expect(config).toHaveProperty('description');
    });
  });
});
