/**
 * Feature: spec/features/replace-hardcoded-npm-test-in-git-checkpoint-conflict-resolution-with-configured-test-command.feature
 *
 * This test file validates that detectConflicts function uses configured test command
 * instead of hardcoded "npm test" in conflict resolution system-reminders.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';

// Import detectConflicts - will need to be exported from git-checkpoint.ts
// This is intentionally importing a function that doesn't exist yet (red phase)
import type { ConflictInfo } from '../git-checkpoint.js';

// Mock import that will fail until implementation
let detectConflicts: (cwd: string, targetOid: string) => Promise<ConflictInfo>;

try {
  // This will fail until we export detectConflicts
  const gitCheckpoint = await import('../git-checkpoint.js');
  detectConflicts = (gitCheckpoint as any).detectConflicts;
} catch {
  // Function not exported yet - tests will fail (red phase)
  detectConflicts = async () => {
    throw new Error('detectConflicts is not exported yet');
  };
}

describe('Feature: Replace hardcoded npm test in git-checkpoint conflict resolution with configured test command', () => {
  let testDir: string;
  let originalHome: string | undefined;

  beforeEach(async () => {
    // Create temporary test directory
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });

    // Override HOME to isolate from user-level config
    originalHome = process.env.HOME;
    process.env.HOME = testDir;

    // Create empty .fspec directory to prevent fallback to any existing user config
    await mkdir(join(testDir, '.fspec'), { recursive: true });
  });

  afterEach(async () => {
    // Restore HOME
    if (originalHome !== undefined) {
      process.env.HOME = originalHome;
    }
    // Cleanup test directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Python project shows pytest in conflict resolution', () => {
    it('should use pytest from config in conflict resolution system-reminder', async () => {
      // Given I have a Python project with "pytest" configured as test command
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      await writeFile(
        configPath,
        JSON.stringify({
          tools: {
            test: {
              command: 'pytest',
            },
          },
        })
      );

      // And git checkpoint restoration causes merge conflicts
      // When the detectConflicts function generates the system-reminder
      const result = await detectConflicts(testDir, 'fake-oid', true);

      // Then the system-reminder should contain "Run: pytest"
      expect(result.systemReminder).toContain('Run: pytest');

      // And the system-reminder should NOT contain "Run: npm test"
      expect(result.systemReminder).not.toContain('Run: npm test');
    });
  });

  describe('Scenario: Rust project shows cargo test in conflict resolution', () => {
    it('should use cargo test from config in conflict resolution system-reminder', async () => {
      // Given I have a Rust project with "cargo test" configured as test command
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      await writeFile(
        configPath,
        JSON.stringify({
          tools: {
            test: {
              command: 'cargo test',
            },
          },
        })
      );

      // And git checkpoint restoration causes merge conflicts
      // When the detectConflicts function generates the system-reminder
      const result = await detectConflicts(testDir, 'fake-oid', true);

      // Then the system-reminder should contain "Run: cargo test"
      expect(result.systemReminder).toContain('Run: cargo test');

      // And the system-reminder should NOT contain "Run: npm test"
      expect(result.systemReminder).not.toContain('Run: npm test');
    });
  });

  describe('Scenario: Go project shows go test in conflict resolution', () => {
    it('should use go test from config in conflict resolution system-reminder', async () => {
      // Given I have a Go project with "go test ./..." configured as test command
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      await writeFile(
        configPath,
        JSON.stringify({
          tools: {
            test: {
              command: 'go test ./...',
            },
          },
        })
      );

      // And git checkpoint restoration causes merge conflicts
      // When the detectConflicts function generates the system-reminder
      const result = await detectConflicts(testDir, 'fake-oid', true);

      // Then the system-reminder should contain "Run: go test ./..."
      expect(result.systemReminder).toContain('Run: go test ./...');

      // And the system-reminder should NOT contain "Run: npm test"
      expect(result.systemReminder).not.toContain('Run: npm test');
    });
  });

  describe('Scenario: Project without config shows generic fallback', () => {
    it('should use generic fallback when no config exists', async () => {
      // Given I have a project without spec/fspec-config.json
      // (no config file created - testDir has no config)

      // And git checkpoint restoration causes merge conflicts
      // When the detectConflicts function generates the system-reminder
      const result = await detectConflicts(testDir, 'fake-oid', true);

      // Then the system-reminder should contain "Run: your configured test command"
      expect(result.systemReminder).toContain(
        'Run: your configured test command'
      );

      // And the system-reminder should NOT contain "Run: npm test"
      expect(result.systemReminder).not.toContain('Run: npm test');
    });
  });

  describe('Scenario: JavaScript project shows npm test when configured', () => {
    it('should use npm test from config when explicitly configured', async () => {
      // Given I have a JavaScript project with "npm test" configured as test command
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      await writeFile(
        configPath,
        JSON.stringify({
          tools: {
            test: {
              command: 'npm test',
            },
          },
        })
      );

      // And git checkpoint restoration causes merge conflicts
      // When the detectConflicts function generates the system-reminder
      const result = await detectConflicts(testDir, 'fake-oid', true);

      // Then the system-reminder should contain "Run: npm test"
      expect(result.systemReminder).toContain('Run: npm test');
    });
  });
});
