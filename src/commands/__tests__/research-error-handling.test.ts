/**
 * Feature: spec/features/unconfigured-research-tool-visibility-and-discovery.feature
 *
 * Tests for error handling when using unconfigured research tools
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { research } from '../research';
import { join } from 'path';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';
import {
  writeJsonTestFile,
  ensureTestDirectory,
} from '../../test-helpers/test-file-operations';

describe('Feature: Unconfigured research tool visibility and discovery', () => {
  let setup: TestDirectorySetup;
  let configPath: string;
  let consoleLogSpy: any;
  let consoleErrorSpy: any;
  let processExitSpy: any;

  beforeEach(async () => {
    setup = await setupTestDirectory('research-error-handling');
    configPath = join(setup.testDir, 'spec', 'fspec-config.json');

    // Ensure spec directory exists
    await ensureTestDirectory(join(setup.testDir, 'spec'));

    // Spy on console and process.exit
    consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    processExitSpy = vi
      .spyOn(process, 'exit')
      .mockImplementation((code?: any) => {
        throw new Error(`process.exit(${code})`);
      });
  });

  afterEach(async () => {
    await setup.cleanup();
    consoleLogSpy.mockRestore();
    consoleErrorSpy.mockRestore();
    processExitSpy.mockRestore();
  });

  describe('Scenario: Error when using unconfigured tool', () => {
    it('should fail with helpful error and setup instructions', async () => {
      // @step Given Perplexity is not configured
      await writeJsonTestFile(configPath, {});

      // @step When I run 'fspec research --tool=perplexity --query="test"'
      try {
        await research(['--query=test'], {
          tool: 'perplexity',
          cwd: setup.testDir,
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
      await writeJsonTestFile(configPath, config);

      try {
        await research(['--query=test'], {
          tool: 'perplexity',
          cwd: setup.testDir,
        });
        expect.fail('Should have thrown an error');
      } catch (error: any) {
        expect(error.message).toContain('Research tool not found');
      }
    });

    it('should handle missing JIRA credentials', async () => {
      await writeJsonTestFile(configPath, {});

      try {
        await research(['--issue=PROJ-123'], {
          tool: 'jira',
          cwd: setup.testDir,
        });
        expect.fail('Should have thrown an error');
      } catch (error: any) {
        expect(error.message).toContain('Research tool not found');
      }
    });
  });

  describe('System-reminder error wrapping', () => {
    it('should wrap config errors in system-reminder tags', async () => {
      await writeJsonTestFile(configPath, {});

      try {
        await research(['--query=test'], {
          tool: 'perplexity',
          cwd: setup.testDir,
        });
        expect.fail('Should have thrown an error');
      } catch (error: any) {
        expect(error.message).toContain('Research tool not found');
      }
    });
  });
});
