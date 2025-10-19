/**
 * Feature: spec/features/hook-management-cli-commands.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { listHooks } from '../list-hooks.js';
import { validateHooks } from '../validate-hooks.js';
import { addHook } from '../add-hook.js';
import { removeHook } from '../remove-hook.js';
import type { HookConfig } from '../../hooks/types.js';

describe('Feature: Hook management CLI commands', () => {
  let testDir: string;
  let configPath: string;

  beforeEach(async () => {
    // Create a temporary test directory
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await mkdir(testDir, { recursive: true });
    configPath = join(testDir, 'spec', 'fspec-hooks.json');
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    // Clean up test directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: List hooks displays all configured hooks', () => {
    it('should display hooks grouped by event', async () => {
      // Given I have a hook configuration file with hooks for "post-implementing"
      // And the hooks are "lint" and "test"
      const config: HookConfig = {
        hooks: {
          'post-implementing': [
            {
              name: 'lint',
              command: 'spec/hooks/lint.sh',
              blocking: false,
            },
            {
              name: 'test',
              command: 'spec/hooks/test.sh',
              blocking: false,
            },
          ],
        },
      };
      await writeFile(configPath, JSON.stringify(config, null, 2));

      // When I run "fspec list-hooks"
      const result = await listHooks({ cwd: testDir });

      // Then the output should display hooks grouped by event
      expect(result.events).toHaveLength(1);
      expect(result.events[0].event).toBe('post-implementing');

      // And the output should show "post-implementing: lint, test"
      expect(result.events[0].hooks).toEqual(['lint', 'test']);
    });
  });

  describe('Scenario: Validate hooks passes when all scripts exist', () => {
    it('should exit with code 0 and indicate validation passed', async () => {
      // Given I have a hook configuration with valid JSON
      // And all hook script files exist
      const config: HookConfig = {
        hooks: {
          'post-implementing': [
            {
              name: 'lint',
              command: 'spec/hooks/lint.sh',
              blocking: false,
            },
          ],
        },
      };
      await writeFile(configPath, JSON.stringify(config, null, 2));

      // Create the hook script file
      const hooksDir = join(testDir, 'spec', 'hooks');
      await mkdir(hooksDir, { recursive: true });
      await writeFile(
        join(hooksDir, 'lint.sh'),
        '#!/bin/bash\necho "Linting..."',
        {
          mode: 0o755,
        }
      );

      // When I run "fspec validate-hooks"
      const result = await validateHooks({ cwd: testDir });

      // Then the command should exit with code 0
      expect(result.exitCode).toBe(0);

      // And the output should indicate validation passed
      expect(result.valid).toBe(true);
    });
  });

  describe('Scenario: Validate hooks fails when script is missing', () => {
    it('should exit with non-zero code and show error message', async () => {
      // Given I have a hook configuration referencing "spec/hooks/missing.sh"
      // And the file "spec/hooks/missing.sh" does not exist
      const config: HookConfig = {
        hooks: {
          'post-implementing': [
            {
              name: 'lint',
              command: 'spec/hooks/missing.sh',
              blocking: false,
            },
          ],
        },
      };
      await writeFile(configPath, JSON.stringify(config, null, 2));

      // When I run "fspec validate-hooks"
      const result = await validateHooks({ cwd: testDir });

      // Then the command should exit with non-zero code
      expect(result.exitCode).toBe(1);

      // And the output should contain "Hook command not found: spec/hooks/missing.sh"
      expect(result.errors).toContain(
        'Hook command not found: spec/hooks/missing.sh'
      );
    });
  });

  describe('Scenario: Add hook to configuration', () => {
    it('should add hook to configuration file', async () => {
      // Given I have a hook configuration file
      const config: HookConfig = {
        hooks: {},
      };
      await writeFile(configPath, JSON.stringify(config, null, 2));

      // When I run "fspec add-hook --name lint --event post-implementing --command spec/hooks/lint.sh --blocking"
      await addHook({
        name: 'lint',
        event: 'post-implementing',
        command: 'spec/hooks/lint.sh',
        blocking: true,
        cwd: testDir,
      });

      // Then a new hook named "lint" should be added to "post-implementing"
      // And the hook should have blocking set to true
      // And the config file should be updated
      const { readFile } = await import('fs/promises');
      const updatedConfig = JSON.parse(
        await readFile(configPath, 'utf-8')
      ) as HookConfig;

      expect(updatedConfig.hooks['post-implementing']).toHaveLength(1);
      expect(updatedConfig.hooks['post-implementing'][0].name).toBe('lint');
      expect(updatedConfig.hooks['post-implementing'][0].command).toBe(
        'spec/hooks/lint.sh'
      );
      expect(updatedConfig.hooks['post-implementing'][0].blocking).toBe(true);
    });
  });

  describe('Scenario: Remove hook from configuration', () => {
    it('should remove hook from configuration file', async () => {
      // Given I have a hook configuration with hook "lint" in "post-implementing"
      const config: HookConfig = {
        hooks: {
          'post-implementing': [
            {
              name: 'lint',
              command: 'spec/hooks/lint.sh',
              blocking: false,
            },
            {
              name: 'test',
              command: 'spec/hooks/test.sh',
              blocking: false,
            },
          ],
        },
      };
      await writeFile(configPath, JSON.stringify(config, null, 2));

      // When I run "fspec remove-hook --name lint --event post-implementing"
      await removeHook({
        name: 'lint',
        event: 'post-implementing',
        cwd: testDir,
      });

      // Then the hook "lint" should be removed from "post-implementing"
      // And the config file should be updated
      const { readFile } = await import('fs/promises');
      const updatedConfig = JSON.parse(
        await readFile(configPath, 'utf-8')
      ) as HookConfig;

      expect(updatedConfig.hooks['post-implementing']).toHaveLength(1);
      expect(updatedConfig.hooks['post-implementing'][0].name).toBe('test');
    });
  });

  describe('Scenario: List hooks when no config file exists', () => {
    it('should display a friendly message', async () => {
      // Given the file "spec/fspec-hooks.json" does not exist
      // (testDir has no hooks config file)

      // When I run "fspec list-hooks"
      const result = await listHooks({ cwd: testDir });

      // Then the output should display a friendly message
      // And the message should indicate no hooks are configured
      expect(result.message).toContain('No hooks are configured');
    });
  });
});
