/**
 * Feature: spec/features/interactive-research-command-with-multiple-backend-support.feature
 *
 * Tests for interactive research command with multiple backend support.
 * These tests verify the 'fspec research' command that auto-discovers and executes
 * research tools from spec/research-scripts/ directory.
 */

import { describe, it, expect, beforeEach, afterEach, beforeAll } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import { execSync } from 'child_process';

const PROJECT_ROOT = process.cwd();
const RESEARCH_SCRIPTS_DIR = path.join(PROJECT_ROOT, 'spec/research-scripts');

describe('Feature: Interactive research command with multiple backend support', () => {
  beforeAll(() => {
    // Enable test mode for research scripts
    process.env.FSPEC_TEST_MODE = '1';
  });

  describe('Scenario: List available research tools without arguments', () => {
    it('should list all available research tools with descriptions', () => {
      // @step Given research tools exist in spec/research-scripts/ directory
      expect(fs.existsSync(RESEARCH_SCRIPTS_DIR)).toBe(true);

      // @step And the tools include perplexity, jira, and confluence
      const perplexityScript = path.join(RESEARCH_SCRIPTS_DIR, 'perplexity');
      const jiraScript = path.join(RESEARCH_SCRIPTS_DIR, 'jira');
      const confluenceScript = path.join(RESEARCH_SCRIPTS_DIR, 'confluence');
      expect(fs.existsSync(perplexityScript)).toBe(true);
      expect(fs.existsSync(jiraScript)).toBe(true);
      expect(fs.existsSync(confluenceScript)).toBe(true);

      // @step When I run "fspec research" without arguments
      const result = execSync('fspec research', {
        encoding: 'utf8',
        cwd: PROJECT_ROOT,
      });

      // @step Then the output should list all available research tools
      expect(result).toContain('perplexity');
      expect(result).toContain('jira');
      expect(result).toContain('confluence');

      // @step And each tool should show its name and description
      expect(result.length).toBeGreaterThan(100);

      // @step And each tool should show usage examples
      expect(result).toContain('fspec research --tool=');
    });
  });

  describe('Scenario: Error on invalid research tool', () => {
    it('should return error for non-existent tool', () => {
      // @step Given no research tool named "invalid" exists
      // (TypeScript plugin system - no bundled or custom tool with this name)

      // @step When I run "fspec research --tool=invalid"
      try {
        execSync('fspec research --tool=invalid', {
          encoding: 'utf8',
          cwd: PROJECT_ROOT,
          stderr: 'pipe',
        });
        throw new Error('Should have thrown an error');
      } catch (error: any) {
        // @step Then the command should exit with error code 1
        expect(error.status).toBe(1);

        // @step And the output should contain "Research tool not found: invalid"
        const errorOutput = error.stderr.toString();
        expect(errorOutput).toContain('Research tool not found: invalid');

        // @step And the output should suggest available bundled tools
        expect(errorOutput).toContain('Available bundled tools:');

        // @step And the output should suggest how to create custom tools
        expect(errorOutput).toContain('spec/research-tools/');
      }
    });
  });

  describe('Scenario: Auto-discover new research tool', () => {
    let customToolDir: string;
    let testToolPath: string;

    beforeEach(() => {
      customToolDir = path.join(PROJECT_ROOT, 'spec/research-tools');
      testToolPath = path.join(customToolDir, 'github.js');

      // Create custom tools directory if it doesn't exist
      if (!fs.existsSync(customToolDir)) {
        fs.mkdirSync(customToolDir, { recursive: true });
      }
    });

    afterEach(() => {
      if (fs.existsSync(testToolPath)) {
        fs.unlinkSync(testToolPath);
      }
    });

    it('should auto-discover newly added TypeScript custom tools', () => {
      // @step Given the TypeScript plugin system is active
      // @step When I add a new custom tool spec/research-tools/github.js
      fs.writeFileSync(
        testToolPath,
        `export const tool = {
  name: 'github',
  description: 'GitHub research tool for issues and PRs',
  async execute(args) { return 'GitHub tool output'; },
  help() { return 'GitHub tool help'; }
};`,
        'utf8'
      );
      expect(fs.existsSync(testToolPath)).toBe(true);

      // @step And I run "fspec research --tool=github"
      const result = execSync('fspec research --tool=github', {
        encoding: 'utf8',
        cwd: PROJECT_ROOT,
      });

      // @step Then the custom tool should be loaded and executed
      expect(result).toContain('GitHub tool output');
    });
  });
});
