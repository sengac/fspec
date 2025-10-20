/**
 * Feature: spec/features/hook-configuration-schema-and-validation.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { loadHookConfig } from '../config.js';

let testDir: string;

describe('Feature: Hook configuration schema and validation', () => {
  beforeEach(async () => {
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec/hooks'), { recursive: true });
  });

  afterEach(async () => {
    try {
      await rm(testDir, { recursive: true, force: true, maxRetries: 3 });
    } catch (error) {
      // Ignore cleanup errors - test cleanup is not critical
      console.warn(`Failed to clean up ${testDir}:`, error);
    }
  });

  describe('Scenario: Load valid hook configuration with single hook', () => {
    it('should load configuration with single hook', async () => {
      // Given I have a file "spec/fspec-hooks.json" with content
      await writeFile(
        join(testDir, 'spec/fspec-hooks.json'),
        JSON.stringify({
          hooks: {
            'post-implementing': [
              {
                name: 'lint',
                command: 'spec/hooks/lint.sh',
                blocking: true,
              },
            ],
          },
        })
      );
      await writeFile(
        join(testDir, 'spec/hooks/lint.sh'),
        '#!/bin/bash\necho "lint"'
      );

      // When I load the hook configuration
      const config = await loadHookConfig(testDir);

      // Then the configuration should be valid
      expect(config).toBeDefined();

      // And the hook "lint" should be registered for event "post-implementing"
      expect(config.hooks['post-implementing']).toHaveLength(1);
      expect(config.hooks['post-implementing'][0].name).toBe('lint');

      // And the hook should have blocking set to true
      expect(config.hooks['post-implementing'][0].blocking).toBe(true);

      // And the hook should have timeout set to 60 (default)
      expect(config.hooks['post-implementing'][0].timeout).toBe(60);
    });
  });

  describe('Scenario: Load hook configuration with timeout override', () => {
    it('should load hook with custom timeout', async () => {
      // Given I have a file "spec/fspec-hooks.json" with content
      await writeFile(
        join(testDir, 'spec/fspec-hooks.json'),
        JSON.stringify({
          hooks: {
            'post-testing': [
              {
                name: 'e2e-tests',
                command: 'test.sh',
                timeout: 300,
              },
            ],
          },
        })
      );
      const testShPath = join(testDir, 'test.sh');
      await writeFile(testShPath, '#!/bin/bash\necho "test"');

      // Ensure file exists before loading config
      const { access } = await import('node:fs/promises');
      await access(testShPath);

      // When I load the hook configuration
      const config = await loadHookConfig(testDir);

      // Then the hook "e2e-tests" should have timeout set to 300
      expect(config.hooks['post-testing'][0].timeout).toBe(300);
    });
  });

  describe('Scenario: Load hook configuration with conditions', () => {
    it('should load hook with conditions', async () => {
      // Given I have a file "spec/fspec-hooks.json" with content
      await writeFile(
        join(testDir, 'spec/fspec-hooks.json'),
        JSON.stringify({
          hooks: {
            'post-implementing': [
              {
                name: 'security',
                command: 'audit.sh',
                condition: {
                  tags: ['@security'],
                  prefix: ['AUTH'],
                },
              },
            ],
          },
        })
      );
      const auditShPath = join(testDir, 'audit.sh');
      await writeFile(auditShPath, '#!/bin/bash\necho "audit"');

      // Ensure file exists before loading config
      const { access } = await import('node:fs/promises');
      await access(auditShPath);

      // When I load the hook configuration
      const config = await loadHookConfig(testDir);

      // Then the hook "security" should have condition tags set to ["@security"]
      expect(config.hooks['post-implementing'][0].condition?.tags).toEqual([
        '@security',
      ]);

      // And the hook "security" should have condition prefix set to ["AUTH"]
      expect(config.hooks['post-implementing'][0].condition?.prefix).toEqual([
        'AUTH',
      ]);
    });
  });

  describe('Scenario: Reject invalid JSON configuration', () => {
    it('should throw error for invalid JSON', async () => {
      // Given I have a file "spec/fspec-hooks.json" with invalid JSON content
      await writeFile(
        join(testDir, 'spec/fspec-hooks.json'),
        '{ invalid json }'
      );

      // When I try to load the hook configuration
      // Then an error should be thrown
      await expect(loadHookConfig(testDir)).rejects.toThrow();

      // And the error message should contain "Invalid JSON"
      await expect(loadHookConfig(testDir)).rejects.toThrow(/Invalid JSON/i);

      // And the error message should be helpful
      await expect(loadHookConfig(testDir)).rejects.toThrow(
        /fspec-hooks\.json/
      );
    });
  });

  describe('Scenario: Reject configuration with non-existent hook command', () => {
    it('should throw error for missing hook command file', async () => {
      // Given I have a file "spec/fspec-hooks.json" with content
      await writeFile(
        join(testDir, 'spec/fspec-hooks.json'),
        JSON.stringify({
          hooks: {
            'post-implementing': [
              {
                name: 'missing',
                command: 'spec/hooks/missing.sh',
              },
            ],
          },
        })
      );
      // And the file "spec/hooks/missing.sh" does not exist

      // When I try to load the hook configuration
      // Then an error should be thrown
      await expect(loadHookConfig(testDir)).rejects.toThrow();

      // And the error message should contain "Hook command not found: spec/hooks/missing.sh"
      await expect(loadHookConfig(testDir)).rejects.toThrow(
        /Hook command not found.*spec\/hooks\/missing\.sh/
      );
    });
  });

  describe('Scenario: Load configuration with global defaults', () => {
    it('should load global defaults and apply to hooks', async () => {
      // Given I have a file "spec/fspec-hooks.json" with content
      await writeFile(
        join(testDir, 'spec/fspec-hooks.json'),
        JSON.stringify({
          global: {
            timeout: 120,
            shell: '/bin/bash',
          },
          hooks: {
            'post-implementing': [
              {
                name: 'lint',
                command: 'spec/hooks/lint.sh',
              },
            ],
          },
        })
      );
      await writeFile(
        join(testDir, 'spec/hooks/lint.sh'),
        '#!/bin/bash\necho "lint"'
      );

      // When I load the hook configuration
      const config = await loadHookConfig(testDir);

      // Then the global timeout should be set to 120
      expect(config.global?.timeout).toBe(120);

      // And the global shell should be set to "/bin/bash"
      expect(config.global?.shell).toBe('/bin/bash');

      // And the hook "lint" should use the global timeout of 120
      expect(config.hooks['post-implementing'][0].timeout).toBe(120);
    });
  });
});
