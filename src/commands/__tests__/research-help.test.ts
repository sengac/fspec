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
      // @step Given the research tool 'perplexity' exists and implements --help
      const toolPath = path.join(researchScriptsDir, 'perplexity');
      expect(fs.existsSync(toolPath)).toBe(true);

      // @step When I run 'fspec research --tool=perplexity --help'
      const result = execSync('fspec research --tool=perplexity --help', {
        encoding: 'utf-8',
      });

      // @step Then I should see Perplexity-specific help documentation
      expect(result).toContain('PERPLEXITY RESEARCH TOOL');

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

  describe('Scenario: Show warning when tool lacks help implementation', () => {
    let legacyToolPath: string;

    beforeEach(() => {
      // Create a legacy tool without --help implementation
      // It requires --query but doesn't recognize --help, so it will error
      legacyToolPath = path.join(researchScriptsDir, 'legacy-tool');
      fs.writeFileSync(
        legacyToolPath,
        '#!/bin/bash\n' +
          '# Legacy tool that does NOT implement --help\n' +
          '# It only recognizes --query flag\n' +
          'if [ "$1" != "--query" ]; then\n' +
          '  echo "Error: Missing required flag --query" >&2\n' +
          '  exit 1\n' +
          'fi\n' +
          'echo "Legacy tool output"\n' +
          'exit 0',
        { mode: 0o755 }
      );
    });

    afterEach(() => {
      // Clean up legacy tool
      if (fs.existsSync(legacyToolPath)) {
        fs.unlinkSync(legacyToolPath);
      }
    });

    it('should show warning for tool without --help', () => {
      // @step Given a research tool exists but does not implement --help flag
      expect(fs.existsSync(legacyToolPath)).toBe(true);

      // @step When I run 'fspec research --tool=legacy-tool --help'
      const result = execSync('fspec research --tool=legacy-tool --help', {
        encoding: 'utf-8',
      });

      // @step Then I should see a warning that the tool does not implement --help
      expect(result).toMatch(/warning|does not implement --help/i);

      // @step And the output should show generic usage instructions for the tool
      expect(result).toContain('fspec research --tool=legacy-tool');

      // @step And the output should indicate where to find the tool script
      expect(result).toContain('spec/research-scripts/legacy-tool');
    });
  });
});
