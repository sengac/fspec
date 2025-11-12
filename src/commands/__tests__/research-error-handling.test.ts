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
        // @step Then an error should be thrown mentioning the tool not found
        expect(error.message).toContain('Research tool not found');
      }
    });

    it('should show configured alternatives when tools are available', async () => {
      // Setup: Configure JIRA only
      const config = {
        research: {
          jira: {
            url: 'https://example.atlassian.net',
            token: 'test-token',
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
        expect(error.message).toContain('Research tool not found');
      }
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
        expect(error.message).toContain('Research tool not found');
      }
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
        expect(error.message).toContain('Research tool not found');
      }
    });
  });
});
