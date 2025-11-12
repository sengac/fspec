/**
 * Feature: spec/features/unconfigured-research-tool-visibility-and-discovery.feature
 *
 * Tests for error handling when using unconfigured research tools
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { research } from '../research';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

describe('Feature: Unconfigured research tool visibility and discovery', () => {
  let testDir: string;
  let configPath: string;
  let consoleLogSpy: any;
  let consoleErrorSpy: any;
  let processExitSpy: any;

  beforeEach(() => {
    // Create temporary test directory
    testDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fspec-test-'));
    configPath = path.join(testDir, 'spec', 'fspec-config.json');

    // Ensure spec directory exists
    fs.mkdirSync(path.join(testDir, 'spec'), { recursive: true });

    // Spy on console and process.exit
    consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    processExitSpy = vi
      .spyOn(process, 'exit')
      .mockImplementation((code?: any) => {
        throw new Error(`process.exit(${code})`);
      });
  });

  afterEach(() => {
    // Cleanup
    if (fs.existsSync(testDir)) {
      fs.rmSync(testDir, { recursive: true, force: true });
    }

    consoleLogSpy.mockRestore();
    consoleErrorSpy.mockRestore();
    processExitSpy.mockRestore();
  });

  describe('Scenario: Error when using unconfigured tool', () => {
    it('should fail with helpful error and setup instructions', async () => {
      // @step Given Perplexity is not configured
      fs.writeFileSync(configPath, JSON.stringify({}), 'utf-8');

      // @step When I run 'fspec research --tool=perplexity --query="test"'
      try {
        await research(['--query=test'], {
          tool: 'perplexity',
          cwd: testDir,
        });
        expect.fail('Should have thrown an error');
      } catch (error: any) {
        // Expect process.exit(1) to be called
        expect(error.message).toContain('process.exit(1)');
      }

      const errorOutput = consoleErrorSpy.mock.calls
        .map((call: any) => call[0])
        .join('\n');

      // @step Then the command should fail with exit code 1
      expect(processExitSpy).toHaveBeenCalledWith(1);

      // @step And the error should mention missing apiKey
      expect(errorOutput).toMatch(/apiKey|api key|not configured/i);

      // @step And the error should show JSON config example for spec/fspec-config.json
      expect(errorOutput).toContain('spec/fspec-config.json');
      expect(errorOutput).toContain('"research"');
      expect(errorOutput).toContain('"perplexity"');
      expect(errorOutput).toContain('"apiKey"');

      // @step And the error should suggest using AST as alternative
      expect(errorOutput).toMatch(/alternative|alternatively|use.*ast/i);
      expect(errorOutput).toContain('ast');
    });

    it('should show configured alternatives when tools are available', async () => {
      // Setup: Configure JIRA only
      const config = {
        research: {
          jira: {
            jiraUrl: 'https://example.atlassian.net',
            username: 'test@example.com',
            apiToken: 'test-token',
          },
        },
      };
      fs.writeFileSync(configPath, JSON.stringify(config), 'utf-8');

      try {
        await research(['--query=test'], {
          tool: 'perplexity',
          cwd: testDir,
        });
        expect.fail('Should have thrown an error');
      } catch (error: any) {
        expect(error.message).toContain('process.exit(1)');
      }

      const errorOutput = consoleErrorSpy.mock.calls
        .map((call: any) => call[0])
        .join('\n');

      // Should suggest both AST and JIRA as alternatives
      expect(errorOutput).toContain('ast');
      expect(errorOutput).toContain('jira');
    });

    it('should handle missing JIRA credentials', async () => {
      fs.writeFileSync(configPath, JSON.stringify({}), 'utf-8');

      try {
        await research(['--issue=PROJ-123'], {
          tool: 'jira',
          cwd: testDir,
        });
        expect.fail('Should have thrown an error');
      } catch (error: any) {
        expect(error.message).toContain('process.exit(1)');
      }

      const errorOutput = consoleErrorSpy.mock.calls
        .map((call: any) => call[0])
        .join('\n');

      // Should mention all required JIRA fields
      expect(errorOutput).toMatch(/jiraUrl|jira url/i);
      expect(errorOutput).toMatch(/username/i);
      expect(errorOutput).toMatch(/apiToken|api token/i);

      // Should show JSON example
      expect(errorOutput).toContain('"jira"');
      expect(errorOutput).toContain('spec/fspec-config.json');
    });
  });

  describe('System-reminder error wrapping', () => {
    it('should wrap config errors in system-reminder tags', async () => {
      fs.writeFileSync(configPath, JSON.stringify({}), 'utf-8');

      try {
        await research(['--query=test'], {
          tool: 'perplexity',
          cwd: testDir,
        });
        expect.fail('Should have thrown an error');
      } catch (error: any) {
        expect(error.message).toContain('process.exit(1)');
      }

      const errorOutput = consoleErrorSpy.mock.calls
        .map((call: any) => call[0])
        .join('\n');

      // Check for system-reminder tags (AI visibility)
      expect(errorOutput).toContain('<system-reminder>');
      expect(errorOutput).toContain('</system-reminder>');
    });
  });
});
